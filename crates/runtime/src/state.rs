//! Owner-batch candidate validation. No external effect or authority is created here.
use crate::coherence::EvidenceCoherent;
use flow_nexus_types::State;
use flow_nexus_types::evidence::{
    EndPhase, Invariant, LaunchPhase, Mutation, MutationBatch, Record, RecordKey,
    VerificationState, VerificationTarget,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordState {
    pub records: Vec<Record>,
}

pub trait Keyed {
    type Key;
    fn key(&self) -> Self::Key;
}
pub trait VerificationTargeted {
    type Target;
    fn verification_target(&self) -> Option<Self::Target>;
}
pub trait Queryable {
    type Key;
    type Record;
    fn record(&self, key: &Self::Key) -> Option<&Self::Record>;
}
pub trait Mutatable: Sized {
    type Batch;
    type Failure;
    fn mutate(&self, batch: &Self::Batch) -> Result<Self, Self::Failure>;
}
pub trait Validatable {
    type Failure;
    fn validate(&self, previous: &Self) -> Result<(), Self::Failure>;
    fn validate_configuration(&self, previous: &Self) -> Result<(), Self::Failure>;
    fn validate_references(&self) -> Result<(), Self::Failure>;
    fn validate_ownership(&self) -> Result<(), Self::Failure>;
    fn validate_allocators(&self, previous: &Self) -> Result<(), Self::Failure>;
    fn validate_verification(&self) -> Result<(), Self::Failure>;
    fn validate_unresolved(&self, previous: &Self) -> Result<(), Self::Failure>;
}
impl Keyed for Record {
    type Key = RecordKey;
    fn key(&self) -> RecordKey {
        match self {
            Self::ConfigurationRecord(_) => RecordKey::ConfigurationSingleton,
            Self::Flow(v) => RecordKey::Flow(v.flow_id.clone()),
            Self::Permaflow(v) => RecordKey::Permaflow(v.address.clone()),
            Self::ProcessBinding(v) => RecordKey::ProcessBinding(v.flow_id.clone()),
            Self::LaunchAttempt(v) => RecordKey::LaunchAttempt(v.request_id.clone()),
            Self::EndAttempt(v) => RecordKey::EndAttempt(v.request_id.clone()),
            Self::AddressReservation(v) => RecordKey::AddressReservation(v.address.clone()),
            Self::Queue(v) => RecordKey::Queue(v.address.clone()),
            Self::Verification(v) => RecordKey::Verification(v.verification_target.clone()),
            Self::RequestAllocator(_) => RecordKey::RequestAllocatorSingleton,
            Self::ArrivalAllocator(v) => RecordKey::ArrivalAllocator(v.address.clone()),
        }
    }
}
impl Keyed for Mutation {
    type Key = RecordKey;
    fn key(&self) -> RecordKey {
        match self {
            Self::Put(v) => v.key(),
            Self::Remove(v) => v.clone(),
        }
    }
}
impl VerificationTargeted for RecordKey {
    type Target = VerificationTarget;
    fn verification_target(&self) -> Option<VerificationTarget> {
        match self {
            Self::Flow(v) => Some(VerificationTarget::Flow(v.clone())),
            Self::ProcessBinding(v) => Some(VerificationTarget::Binding(v.clone())),
            Self::LaunchAttempt(v) => Some(VerificationTarget::Launch(v.clone())),
            Self::EndAttempt(v) => Some(VerificationTarget::End(v.clone())),
            Self::AddressReservation(v) => Some(VerificationTarget::Reservation(v.clone())),
            _ => None,
        }
    }
}
impl Queryable for RecordState {
    type Key = RecordKey;
    type Record = Record;
    fn record(&self, key: &RecordKey) -> Option<&Record> {
        self.records.iter().find(|v| v.key() == *key)
    }
}
impl Mutatable for RecordState {
    type Batch = MutationBatch;
    type Failure = Invariant;
    fn mutate(&self, batch: &MutationBatch) -> Result<Self, Invariant> {
        let mut candidate = self.clone();
        let mut changed = Vec::new();
        for mutation in &batch.0 {
            let key = mutation.key();
            if changed.contains(&key) {
                return Err(Invariant::DuplicateMutation(key));
            }
            changed.push(key.clone());
            if let Mutation::Put(Record::Verification(v)) = mutation {
                if matches!(v.verification_state, VerificationState::Verified(_)) {
                    return Err(Invariant::ReconciliationRequired(
                        v.verification_target.clone(),
                    ));
                }
            }
            candidate.records.retain(|v| v.key() != key);
            if let Mutation::Put(v) = mutation {
                candidate.records.push(v.clone());
            }
        }
        // Invalidate after the entire batch, so mutation order cannot preserve a stale proof.
        for key in &changed {
            if let Some(target) = key.verification_target() {
                candidate.records.retain(|v| !matches!(v, Record::Verification(proof) if proof.verification_target == target));
            }
        }
        candidate.validate(self)?;
        Ok(candidate)
    }
}
impl Validatable for RecordState {
    type Failure = Invariant;
    fn validate(&self, previous: &Self) -> Result<(), Invariant> {
        let mut keys = Vec::new();
        for v in &self.records {
            let key = v.key();
            if keys.contains(&key) {
                return Err(Invariant::DuplicateMutation(key));
            }
            keys.push(key);
        }
        self.validate_configuration(previous)?;
        self.validate_references()?;
        self.validate_evidence()?;
        self.validate_ownership()?;
        self.validate_allocators(previous)?;
        self.validate_verification()?;
        self.validate_unresolved(previous)
    }
    fn validate_configuration(&self, previous: &Self) -> Result<(), Invariant> {
        let key = RecordKey::ConfigurationSingleton;
        let next = match self.record(&key) {
            Some(Record::ConfigurationRecord(v)) => Some(v),
            _ => None,
        };
        let old = match previous.record(&key) {
            Some(Record::ConfigurationRecord(v)) => Some(v),
            _ => None,
        };
        if let (Some(a), Some(b)) = (old, next) {
            if a.configuration.memory_path != b.configuration.memory_path {
                return Err(Invariant::StoreRelocation);
            }
        }
        if old.is_some() && next.is_none() {
            return Err(Invariant::DanglingReference(key));
        }
        let Some(row) = next else {
            return Ok(());
        };
        let c = &row.configuration;
        let alphabet = c.identity_policy.identity_alphabet.0.as_bytes();
        if alphabet.len() < 2
            || alphabet.iter().enumerate().any(|(index, value)| {
                !value.is_ascii_alphanumeric() || alphabet[..index].contains(value)
            })
        {
            return Err(Invariant::InvalidAlphabet);
        }
        let mut addresses = Vec::new();
        for declaration in &c.declaration_vector {
            if addresses.contains(&declaration.address) {
                return Err(Invariant::DuplicateAddress(declaration.address.clone()));
            }
            addresses.push(declaration.address.clone());
            for id in &declaration.module_id_vector {
                if c.module_source_vector
                    .iter()
                    .filter(|v| v.module_id == *id)
                    .count()
                    != 1
                {
                    return Err(Invariant::DanglingReference(key.clone()));
                }
            }
            if c.layer_assignment_vector
                .iter()
                .filter(|v| {
                    v.aspect == declaration.address.aspect && v.layer == declaration.address.layer
                })
                .count()
                != 1
            {
                return Err(Invariant::DanglingReference(key.clone()));
            }
        }
        for (index, module) in c.module_source_vector.iter().enumerate() {
            if c.module_source_vector[..index]
                .iter()
                .any(|v| v.module_id == module.module_id)
            {
                return Err(Invariant::AmbiguousReference(key.clone()));
            }
        }
        for (index, assignment) in c.layer_assignment_vector.iter().enumerate() {
            if c.layer_assignment_vector[..index]
                .iter()
                .any(|v| v.aspect == assignment.aspect && v.layer == assignment.layer)
            {
                return Err(Invariant::AmbiguousReference(key.clone()));
            }
            if c.model_profile_vector
                .iter()
                .filter(|v| v.model_profile_name == assignment.model_profile_name)
                .count()
                != 1
            {
                return Err(Invariant::DanglingReference(key.clone()));
            }
        }
        for (index, model) in c.model_profile_vector.iter().enumerate() {
            if c.model_profile_vector[..index]
                .iter()
                .any(|v| v.model_profile_name == model.model_profile_name)
            {
                return Err(Invariant::AmbiguousReference(key.clone()));
            }
            if c.harness_profile_vector
                .iter()
                .filter(|v| v.harness_profile_name == model.harness_profile_name)
                .count()
                != 1
            {
                return Err(Invariant::DanglingReference(key.clone()));
            }
        }
        for (index, harness) in c.harness_profile_vector.iter().enumerate() {
            if c.harness_profile_vector[..index]
                .iter()
                .any(|v| v.harness_profile_name == harness.harness_profile_name)
            {
                return Err(Invariant::AmbiguousReference(key.clone()));
            }
        }
        Ok(())
    }
    fn validate_references(&self) -> Result<(), Invariant> {
        for record in &self.records {
            match record {
                Record::Permaflow(history) => {
                    for (index, id) in history.flow_id_vector.iter().enumerate() {
                        if history.flow_id_vector[..index].contains(id) {
                            return Err(Invariant::HistoryOwnership(id.clone()));
                        }
                        match self.record(&RecordKey::Flow(id.clone())) {
                            Some(Record::Flow(flow)) if flow.address == history.address => {}
                            _ => return Err(Invariant::HistoryOwnership(id.clone())),
                        }
                    }
                }
                Record::Flow(flow) => {
                    let mut memberships = 0;
                    for value in &self.records {
                        if let Record::Permaflow(history) = value {
                            if history.flow_id_vector.contains(&flow.flow_id) {
                                if history.address != flow.address {
                                    return Err(Invariant::HistoryOwnership(flow.flow_id.clone()));
                                }
                                memberships += 1;
                            }
                        }
                    }
                    if memberships != 1 {
                        return Err(Invariant::HistoryOwnership(flow.flow_id.clone()));
                    }
                    let mut cursor = flow.flow_id_option.clone();
                    let mut visited = vec![flow.flow_id.clone()];
                    while let Some(id) = cursor {
                        if visited.contains(&id) {
                            return Err(Invariant::PredecessorCycle(flow.flow_id.clone()));
                        }
                        visited.push(id.clone());
                        match self.record(&RecordKey::Flow(id.clone())) {
                            Some(Record::Flow(previous)) => {
                                if previous.address != flow.address {
                                    return Err(Invariant::PredecessorAddress(
                                        flow.flow_id.clone(),
                                    ));
                                }
                                cursor = previous.flow_id_option.clone();
                            }
                            _ => return Err(Invariant::DanglingReference(RecordKey::Flow(id))),
                        }
                    }
                }
                Record::ProcessBinding(v) => {
                    if self.record(&RecordKey::Flow(v.flow_id.clone())).is_none() {
                        return Err(Invariant::DanglingReference(RecordKey::Flow(
                            v.flow_id.clone(),
                        )));
                    }
                }
                Record::EndAttempt(v) => {
                    if self.record(&RecordKey::Flow(v.flow_id.clone())).is_none() {
                        return Err(Invariant::DanglingReference(RecordKey::Flow(
                            v.flow_id.clone(),
                        )));
                    }
                }
                Record::LaunchAttempt(v) => {
                    match self.record(&RecordKey::Flow(v.flow_id.clone())) {
                        Some(Record::Flow(flow)) if flow.address != v.address => {
                            return Err(Invariant::DuplicateIdentity(v.flow_id.clone()));
                        }
                        None if matches!(v.launch_phase, LaunchPhase::Complete) => {
                            return Err(Invariant::DanglingReference(RecordKey::Flow(
                                v.flow_id.clone(),
                            )));
                        }
                        _ => {}
                    }
                    if let Some(origin) = &v.origin_admission_id_option {
                        let found = self.records.iter().any(|row| matches!(row,
                            Record::Queue(queue) if queue.address == v.address
                                && queue.admitted_request_vector.iter().any(|admission| admission.request_id == origin.0)));
                        if !found {
                            return Err(Invariant::DanglingAdmission(origin.clone()));
                        }
                    }
                    if v.address != v.resolved_launch.declaration.address
                        || v.resolved_launch.model_profile.harness_profile_name
                            != v.resolved_launch.harness_profile.harness_profile_name
                    {
                        return Err(Invariant::SnapshotMismatch(record.key()));
                    }
                    if !matches!(v.launch_phase, LaunchPhase::Complete) {
                        match self.record(&RecordKey::AddressReservation(v.address.clone())) {
                            Some(Record::AddressReservation(r))
                                if r.flow_id == v.flow_id && r.request_id == v.request_id => {}
                            _ => return Err(Invariant::AttemptReservation(record.key())),
                        }
                    }
                }
                Record::AddressReservation(v) => {
                    match self.record(&RecordKey::LaunchAttempt(v.request_id.clone())) {
                        Some(Record::LaunchAttempt(a))
                            if a.flow_id == v.flow_id
                                && a.address == v.address
                                && !matches!(a.launch_phase, LaunchPhase::Complete) => {}
                        _ => return Err(Invariant::AttemptReservation(record.key())),
                    }
                }
                Record::Queue(v) => {
                    if self
                        .record(&RecordKey::Permaflow(v.address.clone()))
                        .is_none()
                    {
                        return Err(Invariant::DanglingReference(RecordKey::Permaflow(
                            v.address.clone(),
                        )));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn validate_ownership(&self) -> Result<(), Invariant> {
        // A historical identity remains reserved even after its address becomes available.
        let mut claims: Vec<IdentityClaim<'_>> = Vec::new();
        let mut launches = Vec::new();
        for record in &self.records {
            let claim = match record {
                Record::Flow(v) => Some(IdentityClaim {
                    id: &v.flow_id,
                    address: &v.address,
                }),
                Record::AddressReservation(v) => Some(IdentityClaim {
                    id: &v.flow_id,
                    address: &v.address,
                }),
                Record::LaunchAttempt(v) => {
                    if launches.contains(&v.flow_id) {
                        return Err(Invariant::DuplicateIdentity(v.flow_id.clone()));
                    }
                    launches.push(v.flow_id.clone());
                    Some(IdentityClaim {
                        id: &v.flow_id,
                        address: &v.address,
                    })
                }
                _ => None,
            };
            if let Some(claim) = claim {
                if claims
                    .iter()
                    .any(|old| old.id == claim.id && old.address != claim.address)
                {
                    return Err(Invariant::DuplicateIdentity(claim.id.clone()));
                }
                claims.push(claim);
            }
        }
        // A flow and its own reservation describe one owner; distinct identities conflict.
        for record in &self.records {
            let owner = match record {
                Record::Flow(v) if v.state != State::Ended => Some(v.address.clone()),
                Record::AddressReservation(v) => Some(v.address.clone()),
                _ => None,
            };
            if let Some(address) = owner {
                let identity = match record {
                    Record::Flow(v) => &v.flow_id,
                    Record::AddressReservation(v) => &v.flow_id,
                    _ => unreachable!(),
                };
                for earlier in &self.records {
                    match earlier {
                        Record::Flow(v)
                            if v.state != State::Ended
                                && v.address == address
                                && v.flow_id != *identity =>
                        {
                            return Err(Invariant::DuplicateAddress(address.clone()));
                        }
                        Record::AddressReservation(v)
                            if v.address == address && v.flow_id != *identity =>
                        {
                            return Err(Invariant::DuplicateAddress(address.clone()));
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
    fn validate_allocators(&self, previous: &Self) -> Result<(), Invariant> {
        let key = RecordKey::RequestAllocatorSingleton;
        let next = match self.record(&key) {
            Some(Record::RequestAllocator(v)) => Some(v.request_id.0),
            _ => None,
        };
        let old = match previous.record(&key) {
            Some(Record::RequestAllocator(v)) => Some(v.request_id.0),
            _ => None,
        };
        if old.is_some() && next.is_none() {
            return Err(Invariant::AllocationStateRequired(key.clone()));
        }
        if let Some(n) = next {
            if n < 0 {
                return Err(Invariant::CounterOverflow(key.clone()));
            }
            if old.is_some_and(|old| n < old) {
                return Err(Invariant::CounterRegression(key.clone()));
            }
        }
        let mut logical_ids = Vec::new();
        for record in &self.records {
            let request_id = match record {
                Record::LaunchAttempt(v) => Some(&v.request_id),
                Record::EndAttempt(v) => Some(&v.request_id),
                Record::AddressReservation(v) => Some(&v.request_id),
                _ => None,
            };
            if matches!(record, Record::LaunchAttempt(_) | Record::EndAttempt(_)) {
                if let Some(id) = request_id {
                    if logical_ids.contains(id) {
                        return Err(Invariant::DuplicateRequest(id.clone()));
                    }
                    logical_ids.push(id.clone());
                }
            }
            if let Some(id) = request_id {
                let n = next.ok_or_else(|| Invariant::AllocationStateRequired(key.clone()))?;
                if id.0 < 0 || id.0 >= n {
                    return Err(Invariant::CounterRegression(key.clone()));
                }
            }
            if let Record::Queue(queue) = record {
                let ordinal_key = RecordKey::ArrivalAllocator(queue.address.clone());
                let ordinal = match self.record(&ordinal_key) {
                    Some(Record::ArrivalAllocator(v)) => v.arrival_ordinal.0,
                    _ => return Err(Invariant::AllocationStateRequired(ordinal_key)),
                };
                let mut last = None;
                for admitted in &queue.admitted_request_vector {
                    if admitted.arrival_ordinal.0 < 0
                        || last.is_some_and(|old| admitted.arrival_ordinal.0 <= old)
                        || admitted.arrival_ordinal.0 >= ordinal
                    {
                        return Err(Invariant::QueueOrder(queue.address.clone()));
                    }
                    last = Some(admitted.arrival_ordinal.0);
                    if logical_ids.contains(&admitted.request_id) {
                        return Err(Invariant::DuplicateRequest(admitted.request_id.clone()));
                    }
                    logical_ids.push(admitted.request_id.clone());
                    let n = next.ok_or_else(|| Invariant::AllocationStateRequired(key.clone()))?;
                    if admitted.request_id.0 < 0 || admitted.request_id.0 >= n {
                        return Err(Invariant::CounterRegression(key.clone()));
                    }
                }
            }
            if let Record::ArrivalAllocator(v) = record {
                if v.arrival_ordinal.0 < 0 {
                    return Err(Invariant::CounterOverflow(record.key()));
                }
                if let Some(Record::ArrivalAllocator(old)) = previous.record(&record.key()) {
                    if v.arrival_ordinal.0 < old.arrival_ordinal.0 {
                        return Err(Invariant::CounterRegression(record.key()));
                    }
                }
            }
        }
        for record in &previous.records {
            if matches!(record, Record::ArrivalAllocator(_)) && self.record(&record.key()).is_none()
            {
                return Err(Invariant::AllocationStateRequired(record.key()));
            }
        }
        Ok(())
    }
    fn validate_verification(&self) -> Result<(), Invariant> {
        for record in &self.records {
            if let Record::Verification(v) = record {
                let key = match &v.verification_target {
                    VerificationTarget::Flow(id) => RecordKey::Flow(id.clone()),
                    VerificationTarget::Binding(id) => RecordKey::ProcessBinding(id.clone()),
                    VerificationTarget::Launch(id) => RecordKey::LaunchAttempt(id.clone()),
                    VerificationTarget::End(id) => RecordKey::EndAttempt(id.clone()),
                    VerificationTarget::Reservation(address) => {
                        RecordKey::AddressReservation(address.clone())
                    }
                };
                if self.record(&key).is_none() {
                    return Err(Invariant::DanglingReference(key));
                }
                // Imported evidence remains data. Only the future Operation witness path can create Verified.
            }
        }
        Ok(())
    }
    fn validate_unresolved(&self, previous: &Self) -> Result<(), Invariant> {
        for old in &previous.records {
            let active = match old {
                Record::Flow(v) => v.state != State::Ended,
                Record::ProcessBinding(_) | Record::AddressReservation(_) => true,
                Record::LaunchAttempt(v) => !matches!(v.launch_phase, LaunchPhase::Complete),
                Record::EndAttempt(v) => !matches!(v.end_phase, EndPhase::Complete),
                _ => false,
            };
            if active && self.record(&old.key()) != Some(old) {
                return Err(Invariant::UnresolvedEffect(old.key()));
            }
        }
        Ok(())
    }
}

struct IdentityClaim<'a> {
    id: &'a flow_nexus_types::FlowId,
    address: &'a flow_nexus_types::Address,
}
