//! Serialized Memory commands and sealed reservation ordering/lifecycle capabilities.
use crate::{
    reservation::{Reservable, ReservationResolvable},
    store::{
        LayoutBearing, MemoryStore, Readable, Reopenable, StoreAnchor, StoreAnchored, StoreFailure,
        StoreLayout, Writable,
    },
};
use flow_nexus_types::evidence::{
    Invariant, MutationBatch, ReadSelector, Record, ReservationOutcome, ReservationResolution,
    ReserveLaunch,
};
use kameo::{
    actor::{Actor, ActorId, ActorRef, Spawn, WeakActorRef},
    error::ActorStopReason,
    message::{Context, Message},
};
use std::{convert::Infallible, future::Future};
use tokio::sync::oneshot;

#[derive(Clone, Debug)]
pub enum MemoryCommand {
    Read(ReadSelector),
    OwnerBatch(MutationBatch),
    ResolveReservation(ReserveLaunch),
}
#[derive(Debug)]
pub enum MemoryReply {
    Records(Vec<Record>),
    Committed,
    Rejected(Invariant),
    Reservation(ReservationOutcome),
    Resolution(ReservationResolution),
    StoreFailed(StoreFailure),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorTransportFailure {
    ReadUnavailable,
    MutationUncertain,
    WrongActor,
    NotAdmitted,
}
pub struct MemoryActor {
    // Taken by on_stop before shutdown is announced, so a restart can reopen the same store.
    store: Option<MemoryStore>,
}
#[derive(Clone)]
pub struct MemoryPort {
    actor: ActorRef<MemoryActor>,
    layout: StoreLayout,
    anchor: StoreAnchor,
}
/// An observation target, never permission to submit or to declare final absence.
#[derive(Clone)]
pub struct ReservationTarget {
    actor: ActorRef<MemoryActor>,
    request: ReserveLaunch,
}
/// Single-consumer dispatch preparation; target copies cannot resubmit this command.
pub struct PreparedReservation {
    target: ReservationTarget,
}
/// Minted only after enqueue completes on the captured actor.
#[derive(Clone)]
pub struct OrderedReservation {
    target: ReservationTarget,
}
pub struct ReservationSubmission {
    ticket: OrderedReservation,
    pending: oneshot::Receiver<MemoryReply>,
}
/// Minted only after the old actor stopped and released its store, then reopened that layout.
pub struct RecoveredMemory {
    old_actor_id: ActorId,
    port: MemoryPort,
}
// This message is private: claimed fields cannot manufacture its ordering guarantee.
struct ResolveFinal(ReserveLaunch);
struct ReserveDispatch {
    request: ReserveLaunch,
    reply: oneshot::Sender<MemoryReply>,
}

impl Actor for MemoryActor {
    type Args = MemoryStore;
    type Error = Infallible;
    async fn on_start(store: Self::Args, _: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self { store: Some(store) })
    }
    async fn on_stop(
        &mut self,
        _: WeakActorRef<Self>,
        _: ActorStopReason,
    ) -> Result<(), Self::Error> {
        drop(self.store.take());
        Ok(())
    }
}
impl Message<MemoryCommand> for MemoryActor {
    type Reply = Result<MemoryReply, Infallible>;
    async fn handle(
        &mut self,
        command: MemoryCommand,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(store) = self.store.as_mut() else {
            return Ok(MemoryReply::StoreFailed(StoreFailure::Closed));
        };
        Ok(match command {
            MemoryCommand::Read(selector) => match store.read(&selector) {
                Ok(records) => MemoryReply::Records(records),
                Err(error) => MemoryReply::StoreFailed(error),
            },
            MemoryCommand::OwnerBatch(batch) => match store.mutate(&batch) {
                Ok(()) => MemoryReply::Committed,
                Err(StoreFailure::Integrity(error)) => MemoryReply::Rejected(error),
                Err(error) => MemoryReply::StoreFailed(error),
            },
            MemoryCommand::ResolveReservation(request) => match store.resolve(&request) {
                // No ordering or old-actor termination witness accompanied this observation.
                Ok(ReservationResolution::AbsentFinal) => {
                    MemoryReply::Resolution(ReservationResolution::Pending)
                }
                Ok(resolution) => MemoryReply::Resolution(resolution),
                Err(error) => MemoryReply::StoreFailed(error),
            },
        })
    }
}
impl Message<ReserveDispatch> for MemoryActor {
    type Reply = ();
    async fn handle(&mut self, dispatch: ReserveDispatch, _: &mut Context<Self, Self::Reply>) {
        let reply = match self.store.as_mut() {
            Some(store) => match store.reserve(&dispatch.request) {
                Ok(outcome) => MemoryReply::Reservation(outcome),
                Err(error) => MemoryReply::StoreFailed(error),
            },
            None => MemoryReply::StoreFailed(StoreFailure::Closed),
        };
        // A cancelled receiver loses only transport: the already-committed fact remains in Memory.
        let _ = dispatch.reply.send(reply);
    }
}
impl Message<ResolveFinal> for MemoryActor {
    type Reply = Result<MemoryReply, Infallible>;
    async fn handle(
        &mut self,
        request: ResolveFinal,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(match self.store.as_ref() {
            Some(store) => match store.resolve(&request.0) {
                Ok(resolution) => MemoryReply::Resolution(resolution),
                Err(error) => MemoryReply::StoreFailed(error),
            },
            None => MemoryReply::StoreFailed(StoreFailure::Closed),
        })
    }
}
impl From<MemoryStore> for MemoryPort {
    fn from(store: MemoryStore) -> Self {
        Self {
            layout: store.layout(),
            anchor: store.anchor(),
            actor: MemoryActor::spawn(store),
        }
    }
}
pub trait MemoryAccessible {
    type Command;
    type Reply;
    type TransportFailure;
    fn request(
        &self,
        command: Self::Command,
    ) -> impl Future<Output = Result<Self::Reply, Self::TransportFailure>> + Send;
}
impl MemoryAccessible for MemoryPort {
    type Command = MemoryCommand;
    type Reply = MemoryReply;
    type TransportFailure = ActorTransportFailure;
    async fn request(&self, command: MemoryCommand) -> Result<MemoryReply, ActorTransportFailure> {
        let failure = match &command {
            MemoryCommand::Read(_) | MemoryCommand::ResolveReservation(_) => {
                ActorTransportFailure::ReadUnavailable
            }
            MemoryCommand::OwnerBatch(_) => ActorTransportFailure::MutationUncertain,
        };
        self.actor.ask(command).await.map_err(|_| failure)
    }
}
pub trait ReservationControlled {
    type Request;
    type Prepared;
    type Target;
    type Submission;
    type Reply;
    type Failure;
    fn prepare(&self, request: Self::Request) -> Self::Prepared;
    fn submit(
        &self,
        prepared: Self::Prepared,
    ) -> impl Future<Output = Result<Self::Submission, Self::Failure>> + Send;
    fn probe(
        &self,
        target: &Self::Target,
    ) -> impl Future<Output = Result<Self::Reply, Self::Failure>> + Send;
}
impl ReservationControlled for MemoryPort {
    type Request = ReserveLaunch;
    type Prepared = PreparedReservation;
    type Target = ReservationTarget;
    type Submission = ReservationSubmission;
    type Reply = MemoryReply;
    type Failure = ActorTransportFailure;
    fn prepare(&self, request: ReserveLaunch) -> PreparedReservation {
        PreparedReservation {
            target: ReservationTarget {
                actor: self.actor.clone(),
                request,
            },
        }
    }
    async fn submit(
        &self,
        prepared: PreparedReservation,
    ) -> Result<ReservationSubmission, ActorTransportFailure> {
        if self.actor.id() != prepared.target.actor.id() {
            return Err(ActorTransportFailure::WrongActor);
        }
        let (reply, pending) = oneshot::channel();
        self.actor
            .tell(ReserveDispatch {
                request: prepared.target.request.clone(),
                reply,
            })
            .send()
            .await
            .map_err(|error| match error {
                kameo::error::SendError::ActorNotRunning(_)
                | kameo::error::SendError::ActorRestarting(_)
                | kameo::error::SendError::MailboxFull(_)
                | kameo::error::SendError::Timeout(_) => ActorTransportFailure::NotAdmitted,
                _ => ActorTransportFailure::MutationUncertain,
            })?;
        Ok(ReservationSubmission {
            ticket: OrderedReservation {
                target: prepared.target,
            },
            pending,
        })
    }
    async fn probe(
        &self,
        target: &ReservationTarget,
    ) -> Result<MemoryReply, ActorTransportFailure> {
        if self.actor.id() != target.actor.id() {
            return Err(ActorTransportFailure::WrongActor);
        }
        self.request(MemoryCommand::ResolveReservation(target.request.clone()))
            .await
    }
}
pub trait TargetedReservation {
    type Target;
    fn target(&self) -> Self::Target;
}
impl TargetedReservation for PreparedReservation {
    type Target = ReservationTarget;
    fn target(&self) -> ReservationTarget {
        self.target.clone()
    }
}
pub trait TrackedSubmission: Sized {
    type Ticket;
    type Reply;
    type Failure;
    fn ticket(&self) -> Self::Ticket;
    fn discard(self) -> Self::Ticket;
    fn receive(self) -> impl Future<Output = Result<Self::Reply, Self::Failure>> + Send;
}
impl TrackedSubmission for ReservationSubmission {
    type Ticket = OrderedReservation;
    type Reply = MemoryReply;
    type Failure = ActorTransportFailure;
    fn ticket(&self) -> OrderedReservation {
        self.ticket.clone()
    }
    fn discard(self) -> OrderedReservation {
        let Self { ticket, pending } = self;
        drop(pending);
        ticket
    }
    async fn receive(self) -> Result<MemoryReply, ActorTransportFailure> {
        self.pending
            .await
            .map_err(|_| ActorTransportFailure::MutationUncertain)
    }
}
pub trait OrderedResolvable {
    type Reply;
    type Failure;
    fn resolve(&self) -> impl Future<Output = Result<Self::Reply, Self::Failure>> + Send;
}
impl OrderedResolvable for OrderedReservation {
    type Reply = MemoryReply;
    type Failure = ActorTransportFailure;
    async fn resolve(&self) -> Result<MemoryReply, ActorTransportFailure> {
        self.target
            .actor
            .ask(ResolveFinal(self.target.request.clone()))
            .await
            .map_err(|_| ActorTransportFailure::ReadUnavailable)
    }
}
pub trait MemoryRestartable {
    type Recovered;
    type Failure;
    fn restart(&self) -> impl Future<Output = Result<Self::Recovered, Self::Failure>> + Send;
}
impl MemoryRestartable for MemoryPort {
    type Recovered = RecoveredMemory;
    type Failure = StoreFailure;
    async fn restart(&self) -> Result<RecoveredMemory, StoreFailure> {
        let _ = self.actor.stop_gracefully().await;
        self.actor.wait_for_shutdown().await;
        let store = MemoryStore::reopen(self.layout.clone(), &self.anchor)?;
        Ok(RecoveredMemory {
            old_actor_id: self.actor.id(),
            port: MemoryPort::from(store),
        })
    }
}
impl AsRef<MemoryPort> for RecoveredMemory {
    fn as_ref(&self) -> &MemoryPort {
        &self.port
    }
}
pub trait RecoveredResolvable {
    type Target;
    type Reply;
    type Failure;
    fn resolve(
        &self,
        target: &Self::Target,
    ) -> impl Future<Output = Result<Self::Reply, Self::Failure>> + Send;
}
impl RecoveredResolvable for RecoveredMemory {
    type Target = ReservationTarget;
    type Reply = MemoryReply;
    type Failure = ActorTransportFailure;
    async fn resolve(
        &self,
        target: &ReservationTarget,
    ) -> Result<MemoryReply, ActorTransportFailure> {
        if target.actor.id() != self.old_actor_id {
            return Err(ActorTransportFailure::WrongActor);
        }
        self.port
            .actor
            .ask(ResolveFinal(target.request.clone()))
            .await
            .map_err(|_| ActorTransportFailure::ReadUnavailable)
    }
}
