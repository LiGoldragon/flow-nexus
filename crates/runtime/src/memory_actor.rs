//! Component-owned typed replies preserve rejection/storage failures and transport uncertainty.
use crate::store::{MemoryStore, Readable, StoreFailure, Writable};
use flow_nexus_types::evidence::{Invariant, MutationBatch, ReadSelector, Record};
use kameo::{
    actor::{Actor, ActorRef, Spawn},
    message::{Context, Message},
};
use std::{convert::Infallible, future::Future};

#[derive(Clone, Debug)]
pub enum MemoryCommand {
    Read(ReadSelector),
    OwnerBatch(MutationBatch),
}
#[derive(Debug)]
pub enum MemoryReply {
    Records(Vec<Record>),
    Committed,
    Rejected(Invariant),
    StoreFailed(StoreFailure),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorTransportFailure {
    ReadUnavailable,
    MutationUncertain,
}
pub struct MemoryActor {
    store: MemoryStore,
}
#[derive(Clone)]
pub struct MemoryPort {
    actor: ActorRef<MemoryActor>,
}

impl Actor for MemoryActor {
    type Args = MemoryStore;
    type Error = Infallible;
    async fn on_start(store: Self::Args, _: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self { store })
    }
}
impl Message<MemoryCommand> for MemoryActor {
    type Reply = Result<MemoryReply, Infallible>;
    async fn handle(
        &mut self,
        command: MemoryCommand,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(match command {
            MemoryCommand::Read(selector) => match self.store.read(&selector) {
                Ok(records) => MemoryReply::Records(records),
                Err(error) => MemoryReply::StoreFailed(error),
            },
            MemoryCommand::OwnerBatch(batch) => match self.store.mutate(&batch) {
                Ok(()) => MemoryReply::Committed,
                Err(StoreFailure::Integrity(error)) => MemoryReply::Rejected(error),
                Err(error) => MemoryReply::StoreFailed(error),
            },
        })
    }
}
impl From<MemoryStore> for MemoryPort {
    fn from(store: MemoryStore) -> Self {
        Self {
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
            MemoryCommand::Read(_) => ActorTransportFailure::ReadUnavailable,
            MemoryCommand::OwnerBatch(_) => ActorTransportFailure::MutationUncertain,
        };
        self.actor.ask(command).await.map_err(|_| failure)
    }
}
