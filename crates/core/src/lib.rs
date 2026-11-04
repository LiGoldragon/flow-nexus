//! Pure F1 definition validation and atomic mutation runtime.

pub mod generated { pub mod memory; pub mod operation; }
pub mod memory;
pub use generated::operation::{Operation, Outcome};

use signal_flow_nexus_version_1::{
    Complex, ComplexName, DefinitionRejection, DefinitionSet, DuplicateTool_Data, FlowName,
    MutationFailure, PersistenceFailure, Restricted_Data, Route, Snapshot, Tool, ToolConflict_Data,
    Tools, UnknownMemberRoute_Data,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistenceResult { Committed, Unavailable(PersistenceFailure), Uncertain }

pub trait Persistence: Send + Sync + 'static {
    fn persist(&self, complex: &Complex) -> PersistenceResult;
    fn remove(&self, name: &ComplexName) -> PersistenceResult;
}

#[derive(Clone)]
pub struct CapturedSnapshot {
    definitions: Arc<BTreeMap<String, Complex>>,
    pending: Vec<ComplexName>,
}
impl CapturedSnapshot {
    pub fn public(&self) -> Snapshot {
        Snapshot {
            definition_set: DefinitionSet(self.definitions.values().cloned().collect()),
            complex_name_vector: self.pending.clone(),
        }
    }
}

struct State {
    definitions: Arc<BTreeMap<String, Complex>>,
    reservations: HashMap<String, u64>,
    pending: BTreeSet<String>,
    ready: bool,
}
pub struct Core<P> {
    persistence: Arc<P>,
    state: Mutex<State>,
    next_token: std::sync::atomic::AtomicU64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryFailure { AlreadyReady, Invalid(DefinitionRejection) }

impl<P: Persistence> Core<P> {
    pub fn awaiting_recovery(persistence: P) -> Self {
        Self {
            persistence: Arc::new(persistence),
            state: Mutex::new(State {
                definitions: Arc::new(BTreeMap::new()), reservations: HashMap::new(),
                pending: BTreeSet::new(), ready: false,
            }),
            next_token: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Publishes the one startup recovery set. Runtime replacement has no path
    /// back to this whole-set operation.
    pub fn recover(&self, recovered: DefinitionSet) -> Result<(), RecoveryFailure> {
        let definitions = validate_set(&recovered).map_err(RecoveryFailure::Invalid)?;
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.ready { return Err(RecoveryFailure::AlreadyReady); }
        state.definitions = Arc::new(definitions);
        state.ready = true;
        Ok(())
    }

    pub fn capture(&self) -> Option<CapturedSnapshot> {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.ready.then(|| CapturedSnapshot {
            definitions: Arc::clone(&state.definitions),
            pending: state.pending.iter().cloned().map(ComplexName).collect(),
        })
    }

    pub fn execute(&self, operation: Operation) -> Option<Outcome> {
        self.capture()?;
        Some(match operation {
            Operation::Defining(complex) => self.define(complex),
            Operation::Undefining(name) => self.undefine(name),
            Operation::Describing => Outcome::Described(self.capture().expect("ready stays ready").public()),
        })
    }

    fn define(&self, complex: Complex) -> Outcome {
        if let Err(rejection) = validate_complex(&complex) {
            return Outcome::Refused(MutationFailure::Invalid(rejection));
        }
        let name = complex.complex_name.clone();
        let token = match self.reserve(&name, false) {
            Ok(token) => token, Err(failure) => return Outcome::Refused(failure),
        };
        let result = self.persistence.persist(&complex);
        self.finish(name, token, result, |latest| {
            latest.insert(complex.complex_name.0.clone(), complex);
        }, Outcome::Defined)
    }

    fn undefine(&self, name: ComplexName) -> Outcome {
        let token = match self.reserve(&name, true) {
            Ok(token) => token, Err(failure) => return Outcome::Refused(failure),
        };
        let result = self.persistence.remove(&name);
        let removed = name.clone();
        self.finish(name, token, result, move |latest| { latest.remove(&removed.0); }, Outcome::Undefined)
    }

    fn reserve(&self, name: &ComplexName, require_existing: bool) -> Result<u64, MutationFailure> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.pending.contains(&name.0) { return Err(MutationFailure::Uncertain(name.clone())); }
        if state.reservations.contains_key(&name.0) { return Err(MutationFailure::Busy(name.clone())); }
        if require_existing && !state.definitions.contains_key(&name.0) {
            return Err(MutationFailure::Unknown(name.clone()));
        }
        let token = self.next_token.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        state.reservations.insert(name.0.clone(), token);
        Ok(token)
    }

    fn finish(
        &self, name: ComplexName, token: u64, result: PersistenceResult,
        committed: impl FnOnce(&mut BTreeMap<String, Complex>),
        success: impl FnOnce(ComplexName) -> Outcome,
    ) -> Outcome {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.reservations.get(&name.0) != Some(&token) {
            return Outcome::Refused(MutationFailure::Uncertain(name));
        }
        state.reservations.remove(&name.0);
        match result {
            PersistenceResult::Committed => {
                let mut latest = (*state.definitions).clone();
                committed(&mut latest);
                state.definitions = Arc::new(latest);
                success(name)
            }
            PersistenceResult::Unavailable(failure) => Outcome::Refused(MutationFailure::Unavailable(failure)),
            PersistenceResult::Uncertain => {
                state.pending.insert(name.0.clone());
                Outcome::Refused(MutationFailure::Uncertain(name))
            }
        }
    }
}

pub fn validate_set(set: &DefinitionSet) -> Result<BTreeMap<String, Complex>, DefinitionRejection> {
    let mut definitions = BTreeMap::new();
    for complex in &set.0 {
        validate_complex(complex)?;
        if definitions.insert(complex.complex_name.0.clone(), complex.clone()).is_some() {
            return Err(DefinitionRejection::DuplicateComplex(complex.complex_name.clone()));
        }
    }
    Ok(definitions)
}

pub fn validate_complex(complex: &Complex) -> Result<(), DefinitionRejection> {
    if complex.complex_name.0.is_empty() { return Err(DefinitionRejection::EmptyComplexName); }
    let mut members = HashSet::new();
    for member in &complex.member_vector {
        if member.flow_name.0.is_empty() { return Err(DefinitionRejection::EmptyFlowName); }
        if !members.insert(member.flow_name.0.as_str()) {
            return Err(DefinitionRejection::DuplicateMember(member.flow_name.clone()));
        }
    }
    if !members.contains(complex.speaker.0.0.as_str()) {
        return Err(DefinitionRejection::UnknownSpeaker(complex.speaker.0.clone()));
    }
    for member in &complex.member_vector {
        for route in &member.route_vector {
            if let Route::Speaks(target) = route {
                if !members.contains(target.0.as_str()) {
                    return Err(DefinitionRejection::UnknownMemberRoute(UnknownMemberRoute_Data {
                        first_flow_name: member.flow_name.clone(), second_flow_name: target.clone(),
                    }));
                }
            }
        }
        if let Tools::Restricted(Restricted_Data { allow, deny }) = &member.tools {
            validate_tools(&member.flow_name, &allow.0, &deny.0)?;
        }
    }
    Ok(())
}

fn validate_tools(flow_name: &FlowName, allow: &[Tool], deny: &[Tool]) -> Result<(), DefinitionRejection> {
    let mut allowed = HashSet::new();
    for tool in allow {
        if !allowed.insert(tool.0.as_str()) {
            return Err(DefinitionRejection::DuplicateTool(DuplicateTool_Data { flow_name: flow_name.clone(), tool: tool.clone() }));
        }
    }
    let mut denied = HashSet::new();
    for tool in deny {
        if !denied.insert(tool.0.as_str()) {
            return Err(DefinitionRejection::DuplicateTool(DuplicateTool_Data { flow_name: flow_name.clone(), tool: tool.clone() }));
        }
        if allowed.contains(tool.0.as_str()) {
            return Err(DefinitionRejection::ToolConflict(ToolConflict_Data { flow_name: flow_name.clone(), tool: tool.clone() }));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_flow_nexus_version_1::{Beliefs, Continuity, Effort, Harness, Member, Missing, ModelName, Speaker, Workspace};
    use std::sync::Barrier;

    fn complex(name: &str) -> Complex {
        let flow = FlowName(format!("{name}-member"));
        Complex { complex_name: ComplexName(name.into()), member_vector: vec![Member {
            flow_name: flow.clone(), harness: Harness::Codex, model_name: ModelName("configured".into()),
            effort: Effort::Medium, continuity: Continuity::Ephemeral, workspace: Workspace::Temporary,
            beliefs: Beliefs { module_name_vector: vec![], missing: Missing::Refuse },
            tools: Tools::HarnessDefault, route_vector: vec![],
        }], speaker: Speaker(flow) }
    }
    struct Commit;
    impl Persistence for Commit {
        fn persist(&self, _: &Complex) -> PersistenceResult { PersistenceResult::Committed }
        fn remove(&self, _: &ComplexName) -> PersistenceResult { PersistenceResult::Committed }
    }
    fn ready<P: Persistence>(p: P, definitions: Vec<Complex>) -> Core<P> {
        let core = Core::awaiting_recovery(p);
        core.recover(DefinitionSet(definitions)).expect("valid recovery");
        core
    }
    fn names(snapshot: Snapshot) -> Vec<String> {
        snapshot.definition_set.0.into_iter().map(|value| value.complex_name.0).collect()
    }

    #[test]
    fn replace_one_preserves_other_and_captured_snapshot() {
        let core = ready(Commit, vec![complex("one"), complex("two")]);
        let before = core.capture().unwrap();
        let mut replacement = complex("one");
        replacement.member_vector[0].model_name = ModelName("replacement".into());
        assert_eq!(core.execute(Operation::Defining(replacement)), Some(Outcome::Defined(ComplexName("one".into()))));
        assert_eq!(names(core.capture().unwrap().public()), ["one", "two"]);
        assert_eq!(before.public().definition_set.0[0].member_vector[0].model_name.0, "configured");
    }

    struct Refuse;
    impl Persistence for Refuse {
        fn persist(&self, value: &Complex) -> PersistenceResult { unavailable(&value.complex_name) }
        fn remove(&self, value: &ComplexName) -> PersistenceResult { unavailable(value) }
    }
    fn unavailable(name: &ComplexName) -> PersistenceResult {
        PersistenceResult::Unavailable(PersistenceFailure {
            complex_name: name.clone(), message: signal_flow_nexus_version_1::Message("precommit".into()),
        })
    }
    #[test]
    fn invalid_and_precommit_failure_preserve_old() {
        let core = ready(Refuse, vec![complex("one")]);
        let mut invalid = complex("one");
        invalid.speaker = Speaker(FlowName("absent".into()));
        assert!(matches!(core.execute(Operation::Defining(invalid)), Some(Outcome::Refused(MutationFailure::Invalid(_)))));
        assert!(matches!(core.execute(Operation::Defining(complex("one"))), Some(Outcome::Refused(MutationFailure::Unavailable(_)))));
        assert_eq!(names(core.capture().unwrap().public()), ["one"]);
    }

    struct UncertainFirst(std::sync::atomic::AtomicU64);
    impl Persistence for UncertainFirst {
        fn persist(&self, _: &Complex) -> PersistenceResult {
            if self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed) == 0 { PersistenceResult::Uncertain } else { PersistenceResult::Committed }
        }
        fn remove(&self, _: &ComplexName) -> PersistenceResult { PersistenceResult::Committed }
    }
    #[test]
    fn uncertainty_fences_one_name_and_other_progresses() {
        let core = ready(UncertainFirst(0.into()), vec![]);
        assert!(matches!(core.execute(Operation::Defining(complex("one"))), Some(Outcome::Refused(MutationFailure::Uncertain(_)))));
        assert!(matches!(core.execute(Operation::Defining(complex("one"))), Some(Outcome::Refused(MutationFailure::Uncertain(_)))));
        assert_eq!(core.execute(Operation::Defining(complex("two"))), Some(Outcome::Defined(ComplexName("two".into()))));
        let snapshot = core.capture().unwrap().public();
        assert_eq!(names(snapshot.clone()), ["two"]);
        assert_eq!(snapshot.complex_name_vector, [ComplexName("one".into())]);
    }

    struct Blocking { entered: Arc<Barrier>, release: Arc<Barrier> }
    impl Persistence for Blocking {
        fn persist(&self, value: &Complex) -> PersistenceResult {
            if value.complex_name.0 == "one" { self.entered.wait(); self.release.wait(); }
            PersistenceResult::Committed
        }
        fn remove(&self, _: &ComplexName) -> PersistenceResult { PersistenceResult::Committed }
    }
    #[test]
    fn same_name_busy_other_name_merges_into_latest() {
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let core = Arc::new(ready(Blocking { entered: entered.clone(), release: release.clone() }, vec![]));
        let first = { let core = core.clone(); std::thread::spawn(move || core.execute(Operation::Defining(complex("one")))) };
        entered.wait();
        assert!(matches!(core.execute(Operation::Defining(complex("one"))), Some(Outcome::Refused(MutationFailure::Busy(_)))));
        assert_eq!(core.execute(Operation::Defining(complex("two"))), Some(Outcome::Defined(ComplexName("two".into()))));
        release.wait();
        assert_eq!(first.join().unwrap(), Some(Outcome::Defined(ComplexName("one".into()))));
        assert_eq!(names(core.capture().unwrap().public()), ["one", "two"]);
    }

    #[test]
    fn recovery_validates_duplicates_and_is_one_shot() {
        let core = Core::awaiting_recovery(Commit);
        assert!(matches!(core.recover(DefinitionSet(vec![complex("one"), complex("one")])), Err(RecoveryFailure::Invalid(DefinitionRejection::DuplicateComplex(_)))));
        core.recover(DefinitionSet(vec![complex("one")])).unwrap();
        assert_eq!(core.recover(DefinitionSet(vec![])), Err(RecoveryFailure::AlreadyReady));
    }
}
