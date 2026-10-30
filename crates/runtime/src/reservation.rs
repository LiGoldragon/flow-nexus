//! Typed reservation of a Flow identity and address before external creation.
use crate::store::{MemoryStore, Readable, StoreFailure, Writable};
use flow_nexus_types::State;
use flow_nexus_types::evidence::{
    Invariant, Mutation, MutationBatch, Record, RecordKey, ReservationOutcome,
    ReservationResolution, ReserveLaunch, ResourceEvidence, Uncertain_Data,
};
use flow_nexus_types::memory::{AddressReservation, LaunchAttempt, RequestAllocator};

pub trait Reservable {
    type Request;
    type Outcome;
    type Failure;
    fn reserve(&mut self, request: &Self::Request) -> Result<Self::Outcome, Self::Failure>;
}

pub trait ReservationResolvable {
    type Request;
    type Resolution;
    type Failure;
    fn resolve(&self, request: &Self::Request) -> Result<Self::Resolution, Self::Failure>;
}

impl Reservable for MemoryStore {
    type Request = ReserveLaunch;
    type Outcome = ReservationOutcome;
    type Failure = StoreFailure;

    fn reserve(&mut self, request: &ReserveLaunch) -> Result<ReservationOutcome, StoreFailure> {
        let state = self.snapshot()?;
        let allocator = match state.records.iter().find_map(|record| match record {
            Record::RequestAllocator(row) => Some(row),
            _ => None,
        }) {
            Some(row) => row,
            None => return Ok(ReservationOutcome::AllocatorUnavailable),
        };
        if allocator.request_id != request.request_id {
            return Ok(ReservationOutcome::CounterAdvanced(
                allocator.request_id.clone(),
            ));
        }
        let configuration = match state.records.iter().find_map(|record| match record {
            Record::ConfigurationRecord(row) => Some(&row.configuration),
            _ => None,
        }) {
            Some(configuration) => configuration,
            None => return Ok(ReservationOutcome::ConfigurationUnavailable),
        };
        let resolved = &request.resolved_launch;
        if !configuration
            .declaration_vector
            .contains(&resolved.declaration)
            || !configuration
                .layer_assignment_vector
                .iter()
                .any(|assignment| {
                    assignment.aspect == resolved.declaration.address.aspect
                        && assignment.layer == resolved.declaration.address.layer
                        && assignment.model_profile_name
                            == resolved.model_profile.model_profile_name
                })
            || !configuration
                .model_profile_vector
                .contains(&resolved.model_profile)
            || !configuration
                .harness_profile_vector
                .contains(&resolved.harness_profile)
            || configuration.identity_policy != resolved.identity_policy
            || configuration.capsule_configuration != resolved.capsule_configuration
            || resolved.module_content_vector.len() != resolved.declaration.module_id_vector.len()
            || resolved
                .declaration
                .module_id_vector
                .iter()
                .zip(&resolved.module_content_vector)
                .any(|(id, content)| {
                    content.module_id != *id
                        || !configuration.module_source_vector.iter().any(|source| {
                            source.module_id == *id && source.module_kind == content.module_kind
                        })
                })
        {
            return Ok(ReservationOutcome::Refused(Invariant::SnapshotMismatch(
                RecordKey::LaunchAttempt(request.request_id.clone()),
            )));
        }
        if state.records.iter().any(|record| match record {
            Record::Flow(row) => row.flow_id == request.flow_id,
            Record::LaunchAttempt(row) => row.flow_id == request.flow_id,
            Record::AddressReservation(row) => row.flow_id == request.flow_id,
            _ => false,
        }) {
            return Ok(ReservationOutcome::IdentityCollision(
                request.flow_id.clone(),
            ));
        }
        if state.records.iter().any(|record| match record {
            Record::Flow(row) => {
                row.address == request.resolved_launch.declaration.address
                    && row.state != State::Ended
            }
            Record::AddressReservation(row) => {
                row.address == request.resolved_launch.declaration.address
            }
            _ => false,
        }) {
            return Ok(ReservationOutcome::AddressOccupied(
                request.resolved_launch.declaration.address.clone(),
            ));
        }
        let next = match request.request_id.0.checked_add(1) {
            Some(value) => value,
            None => return Ok(ReservationOutcome::CounterOverflow),
        };
        let attempt = LaunchAttempt {
            request_id: request.request_id.clone(),
            flow_id: request.flow_id.clone(),
            address: request.resolved_launch.declaration.address.clone(),
            origin_admission_id_option: request.origin_admission_id_option.clone(),
            resolved_launch: request.resolved_launch.clone(),
            resource_evidence: ResourceEvidence::Uncertain(Uncertain_Data {
                creation_locator: request.creation_intent.creation_locator.clone(),
                attempted_action: request.creation_intent.attempted_action.clone(),
            }),
            creation_intent: request.creation_intent.clone(),
            launch_phase: flow_nexus_types::evidence::LaunchPhase::Reserved,
        };
        let reservation = AddressReservation {
            address: attempt.address.clone(),
            flow_id: attempt.flow_id.clone(),
            request_id: attempt.request_id.clone(),
        };
        let result = self.mutate(&MutationBatch(vec![
            Mutation::Put(Record::RequestAllocator(RequestAllocator {
                request_id: flow_nexus_types::evidence::RequestId(next),
            })),
            Mutation::Put(Record::LaunchAttempt(attempt)),
            Mutation::Put(Record::AddressReservation(reservation)),
        ]));
        match result {
            Ok(()) => Ok(ReservationOutcome::Reserved(request.request_id.clone())),
            Err(StoreFailure::Integrity(error)) => Ok(ReservationOutcome::Refused(error)),
            Err(error) => Err(error),
        }
    }
}

impl ReservationResolvable for MemoryStore {
    type Request = ReserveLaunch;
    type Resolution = ReservationResolution;
    type Failure = StoreFailure;

    fn resolve(&self, request: &ReserveLaunch) -> Result<ReservationResolution, StoreFailure> {
        let state = self.snapshot()?;
        if let Some(record) = state.records.iter().find(|record| match record {
            Record::LaunchAttempt(row) => row.request_id == request.request_id,
            Record::EndAttempt(row) => row.request_id == request.request_id,
            Record::Queue(queue) => queue
                .admitted_request_vector
                .iter()
                .any(|admitted| admitted.request_id == request.request_id),
            _ => false,
        }) {
            return match record {
                Record::LaunchAttempt(row)
                    if row.flow_id == request.flow_id
                        && row.resolved_launch == request.resolved_launch
                        && row.creation_intent == request.creation_intent
                        && row.origin_admission_id_option == request.origin_admission_id_option =>
                {
                    Ok(ReservationResolution::Matched(row.clone()))
                }
                _ => Ok(ReservationResolution::Conflicting(record.clone())),
            };
        }
        Ok(ReservationResolution::AbsentFinal)
    }
}
