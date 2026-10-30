//! Structural evidence consistency only. A matching stored claim is not live authority.
use crate::state::{Queryable, RecordState};
use flow_nexus_types::evidence::{
    CapsuleHandle, CreationLocator, EndPhase, Invariant, LaunchPhase, Record, RecordKey,
    ResourceEvidence, StartWitness,
};

pub trait Located {
    type Locator;
    fn locator(&self) -> &Self::Locator;
}
impl Located for ResourceEvidence {
    type Locator = CreationLocator;
    fn locator(&self) -> &CreationLocator {
        match self {
            Self::NoneConfirmed(v) => v,
            Self::LocatedPartial(v) => &v.creation_locator,
            Self::LocatedComplete(v) => &v.creation_locator,
            Self::Uncertain(v) => &v.creation_locator,
        }
    }
}
impl Located for CapsuleHandle {
    type Locator = CreationLocator;
    fn locator(&self) -> &CreationLocator {
        &self.creation_locator
    }
}
pub trait EvidenceCoherent {
    type Failure;
    fn validate_evidence(&self) -> Result<(), Self::Failure>;
}
impl EvidenceCoherent for RecordState {
    type Failure = Invariant;
    fn validate_evidence(&self) -> Result<(), Invariant> {
        for record in &self.records {
            if let Record::LaunchAttempt(attempt) = record {
                let key = RecordKey::LaunchAttempt(attempt.request_id.clone());
                let locator = &attempt.creation_intent.creation_locator;
                if attempt.resource_evidence.locator() != locator {
                    return Err(Invariant::SnapshotMismatch(key));
                }
                let phase_handle = match &attempt.launch_phase {
                    LaunchPhase::Prepared(v) | LaunchPhase::Submitted(v) => Some(v),
                    LaunchPhase::Started(v) => Some(&v.capsule_handle),
                    LaunchPhase::Cleanup(v) | LaunchPhase::RecoveryRequired(v) => {
                        if v != &attempt.resource_evidence {
                            return Err(Invariant::SnapshotMismatch(key));
                        }
                        None
                    }
                    _ => None,
                };
                if let Some(handle) = phase_handle {
                    if handle.locator() != locator {
                        return Err(Invariant::SnapshotMismatch(key));
                    }
                    match &attempt.resource_evidence {
                        ResourceEvidence::LocatedComplete(v) if v != handle => {
                            return Err(Invariant::SnapshotMismatch(key));
                        }
                        ResourceEvidence::LocatedPartial(v) => {
                            if v.pane_id_option
                                .as_ref()
                                .is_some_and(|pane| pane != &handle.pane_id)
                                || v.process_incarnation_option
                                    .as_ref()
                                    .is_some_and(|process| process != &handle.process_incarnation)
                            {
                                return Err(Invariant::SnapshotMismatch(key));
                            }
                        }
                        ResourceEvidence::NoneConfirmed(_) => {
                            return Err(Invariant::SnapshotMismatch(key));
                        }
                        _ => {}
                    }
                }
            }
            if let Record::EndAttempt(attempt) = record {
                let witness = match &attempt.end_phase {
                    EndPhase::Intended(v)
                    | EndPhase::StopRequested(v)
                    | EndPhase::RecoveryRequired(v) => Some(v),
                    EndPhase::Exited(v) => {
                        self.validate_target_handle(&attempt.flow_id, &v.capsule_handle, record)?;
                        None
                    }
                    EndPhase::Complete => None,
                };
                if let Some(StartWitness {
                    capsule_handle,
                    session_id,
                    transcript_path,
                }) = witness
                {
                    self.validate_target_handle(&attempt.flow_id, capsule_handle, record)?;
                    if let Some(Record::Flow(flow)) =
                        self.record(&RecordKey::Flow(attempt.flow_id.clone()))
                    {
                        if flow
                            .session_id_option
                            .as_ref()
                            .is_some_and(|v| v != session_id)
                            || flow
                                .transcript_path_option
                                .as_ref()
                                .is_some_and(|v| v != transcript_path)
                        {
                            return Err(Invariant::SnapshotMismatch(RecordKey::EndAttempt(
                                attempt.request_id.clone(),
                            )));
                        }
                    }
                }
            }
            if let Record::ProcessBinding(binding) = record {
                for row in &self.records {
                    if let Record::LaunchAttempt(launch) = row {
                        if launch.flow_id == binding.flow_id {
                            if let ResourceEvidence::LocatedComplete(handle) =
                                &launch.resource_evidence
                            {
                                if handle.process_incarnation != binding.process_incarnation {
                                    return Err(Invariant::SnapshotMismatch(
                                        RecordKey::ProcessBinding(binding.flow_id.clone()),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
trait TargetHandleCoherent {
    fn validate_target_handle(
        &self,
        id: &flow_nexus_types::FlowId,
        handle: &CapsuleHandle,
        record: &Record,
    ) -> Result<(), Invariant>;
}
impl TargetHandleCoherent for RecordState {
    fn validate_target_handle(
        &self,
        id: &flow_nexus_types::FlowId,
        handle: &CapsuleHandle,
        record: &Record,
    ) -> Result<(), Invariant> {
        use crate::state::Keyed;
        if let Some(Record::ProcessBinding(binding)) =
            self.record(&RecordKey::ProcessBinding(id.clone()))
        {
            if binding.process_incarnation != handle.process_incarnation {
                return Err(Invariant::SnapshotMismatch(record.key()));
            }
        }
        for row in &self.records {
            if let Record::LaunchAttempt(launch) = row {
                if launch.flow_id == *id {
                    if launch.creation_intent.creation_locator != handle.creation_locator {
                        return Err(Invariant::SnapshotMismatch(record.key()));
                    }
                    if let ResourceEvidence::LocatedComplete(known) = &launch.resource_evidence {
                        if known != handle {
                            return Err(Invariant::SnapshotMismatch(record.key()));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
