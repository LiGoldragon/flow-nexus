//! Flow definition validation, binary Memory, and process-lifetime publication.

pub mod generated { pub mod memory; pub mod operation; }
pub mod memory;
pub use generated::operation::{Operation, Outcome};

use generated::memory::Answer;
use memory::{DefinitionMemory, LoadFailure};
use signal_flow_nexus_version_1::{
    Complex, ComplexName, DefinitionFailure, DefinitionRejection, DefinitionSet,
    DuplicateTool_Data, FlowName, Restricted_Data, Route, Tool, ToolConflict_Data, Tools,
    UnknownMemberRoute_Data,
};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct Core {
    memory: DefinitionMemory,
    definitions: Mutex<Arc<BTreeMap<String, Complex>>>,
}

impl Core {
    pub fn start(memory: DefinitionMemory) -> Result<Self, LoadFailure> {
        let definitions = validate_set(&memory.load()?)?;
        Ok(Self { memory, definitions: Mutex::new(Arc::new(definitions)) })
    }

    pub fn capture(&self) -> Arc<BTreeMap<String, Complex>> {
        Arc::clone(&self.definitions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    pub fn execute(&self, operation: Operation) -> Outcome {
        match operation {
            Operation::Defining(complex) => self.define(complex),
            Operation::Describing(name) => self.describe(name),
            Operation::Listing => self.list(),
        }
    }

    fn define(&self, complex: Complex) -> Outcome {
        if let Err(rejection) = validate_complex(&complex) {
            return Outcome::Refused(DefinitionFailure::Invalid(rejection));
        }
        // Memory and publication are serialized as one change.
        let mut definitions = self.definitions.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match self.memory.store(&complex) {
            Answer::Succeeded => {
                let mut next = (**definitions).clone();
                next.insert(complex.complex_name.0.clone(), complex.clone());
                *definitions = Arc::new(next);
                Outcome::Defined(complex.complex_name)
            }
            Answer::Failed(refusal) => Outcome::Refused(DefinitionFailure::Memory(refusal)),
        }
    }

    fn describe(&self, name: ComplexName) -> Outcome {
        match self.capture().get(&name.0).cloned() {
            Some(complex) => Outcome::Described(complex),
            None => Outcome::Refused(DefinitionFailure::Unknown(name)),
        }
    }

    fn list(&self) -> Outcome {
        Outcome::Listed(self.capture().values().map(|complex| complex.complex_name.clone()).collect())
    }
}

pub fn validate_set(set: &DefinitionSet) -> Result<BTreeMap<String, Complex>, LoadFailure> {
    let mut definitions = BTreeMap::new();
    for complex in &set.0 {
        validate_complex(complex).map_err(LoadFailure::Invalid)?;
        if definitions.insert(complex.complex_name.0.clone(), complex.clone()).is_some() {
            return Err(LoadFailure::Invalid(DefinitionRejection::DuplicateComplex(complex.complex_name.clone())));
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
    use signal_flow_nexus_version_1::{
        Beliefs, Continuity, Effort, Harness, Member, Missing, ModelName, Speaker, Workspace,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(1);
            let path = std::env::temp_dir().join(format!("flow-core-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir(&path).unwrap(); Self(path)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    fn complex(name: &str, model: &str) -> Complex {
        let flow_name = FlowName(format!("{name}-member"));
        Complex { complex_name: ComplexName(name.into()), member_vector: vec![Member {
            flow_name: flow_name.clone(), harness: Harness::Codex, model_name: ModelName(model.into()),
            effort: Effort::Medium, continuity: Continuity::Ephemeral, workspace: Workspace::Temporary,
            beliefs: Beliefs { module_name_vector: vec![], missing: Missing::Refuse },
            tools: Tools::HarnessDefault, route_vector: vec![],
        }], speaker: Speaker(flow_name) }
    }

    #[test]
    fn define_restart_describe_and_list() {
        let fixture = Fixture::new();
        let core = Core::start(DefinitionMemory::open(&fixture.0).unwrap()).unwrap();
        let value = complex("one", "configured");
        assert_eq!(core.execute(Operation::Defining(value.clone())), Outcome::Defined(ComplexName("one".into())));
        drop(core);
        let restarted = Core::start(DefinitionMemory::open(&fixture.0).unwrap()).unwrap();
        assert_eq!(restarted.execute(Operation::Describing(ComplexName("one".into()))), Outcome::Described(value));
        assert_eq!(restarted.execute(Operation::Listing), Outcome::Listed(vec![ComplexName("one".into())]));
    }

    #[test]
    fn invalid_define_preserves_the_published_and_persisted_value() {
        let fixture = Fixture::new();
        let core = Core::start(DefinitionMemory::open(&fixture.0).unwrap()).unwrap();
        let original = complex("one", "original");
        assert!(matches!(core.execute(Operation::Defining(original.clone())), Outcome::Defined(_)));
        let mut invalid = complex("one", "replacement");
        invalid.speaker = Speaker(FlowName("missing".into()));
        assert!(matches!(core.execute(Operation::Defining(invalid)), Outcome::Refused(DefinitionFailure::Invalid(_))));
        drop(core);
        let restarted = Core::start(DefinitionMemory::open(&fixture.0).unwrap()).unwrap();
        assert_eq!(restarted.execute(Operation::Describing(ComplexName("one".into()))), Outcome::Described(original));
    }
}
