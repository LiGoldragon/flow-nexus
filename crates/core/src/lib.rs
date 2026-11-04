//! Flow definition validation, binary Memory, and process-lifetime publication.

pub mod generated { pub mod memory; pub mod operation; }
pub mod memory;
pub use generated::operation::{Operation, Outcome};

use generated::memory::Answer;
use memory::{DefinitionMemory, LoadFailure};
use flow_nexus_emission::generated::emission as emission;
use signal_flow_nexus_version_1::{
    Complex, ComplexName, CreateFailed, DefinitionRejection, DefinitionSet,
    DuplicateTool_Data, FlowId, FlowName, FurnishFailed, FurnishFailure, Harness, LaunchRequest,
    Missing, MissingBeliefs, PromptFailed, RegisterFailed, RegisterFailure, RequestFailure, Restricted_Data, Route, StartFailed,
    Tool, ToolConflict_Data, Tools, UnknownMemberRoute_Data, UnsupportedHarness, UnsupportedTools,
};
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmissionTransport { Unavailable, Uncertain }

pub trait EmissionPort {
    fn emit(&mut self, query: emission::Query) -> Result<emission::Response, EmissionTransport>;
}

pub struct Core {
    memory: DefinitionMemory,
    definitions: Mutex<Arc<BTreeMap<String, Complex>>>,
    session: signal_herdr_nexus::Session,
    next_flow_id: AtomicI64,
}

impl Core {
    pub fn start(memory: DefinitionMemory, session: signal_herdr_nexus::Session) -> Result<Self, LoadFailure> {
        let definitions = validate_set(&memory.load()?)?;
        Ok(Self { memory, definitions: Mutex::new(Arc::new(definitions)), session, next_flow_id: AtomicI64::new(1) })
    }

    pub fn capture(&self) -> Arc<BTreeMap<String, Complex>> {
        Arc::clone(&self.definitions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    pub fn execute(&self, operation: Operation, emission: &mut impl EmissionPort) -> Outcome {
        match operation {
            Operation::Registering(repository) => self.register(repository, emission),
            Operation::Defining(complex) => self.define(complex),
            Operation::Describing(name) => self.describe(name),
            Operation::Listing => self.list(),
            Operation::Launching(request) => self.launch(request, emission),
        }
    }

    fn register(&self, repository: signal_belief_nexus::Repository, port: &mut impl EmissionPort) -> Outcome {
        match port.emit(emission::Query::Register(repository)) {
            Ok(emission::Response::Registered(emission::Registered_Data::Succeeded(state))) => Outcome::Registered(state),
            Ok(emission::Response::Registered(emission::Registered_Data::Refused(refusal))) => register_failure(RegisterFailure::Refused(refusal)),
            Ok(emission::Response::Registered(emission::Registered_Data::Unavailable)) | Err(EmissionTransport::Unavailable) => register_failure(RegisterFailure::Unavailable),
            Ok(emission::Response::Registered(emission::Registered_Data::Uncertain)) | Err(EmissionTransport::Uncertain) | Ok(_) => register_failure(RegisterFailure::Uncertain),
        }
    }

    fn define(&self, complex: Complex) -> Outcome {
        if let Err(rejection) = validate_complex(&complex) {
            return Outcome::Refused(RequestFailure::Invalid(rejection));
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
            Answer::Failed(refusal) => Outcome::Refused(RequestFailure::Memory(refusal)),
        }
    }

    fn describe(&self, name: ComplexName) -> Outcome {
        match self.capture().get(&name.0).cloned() {
            Some(complex) => Outcome::Described(complex),
            None => Outcome::Refused(RequestFailure::UnknownComplex(name)),
        }
    }

    fn list(&self) -> Outcome {
        Outcome::Listed(self.capture().values().map(|complex| complex.complex_name.clone()).collect())
    }

    fn launch(&self, request: LaunchRequest, port: &mut impl EmissionPort) -> Outcome {
        let definitions = self.capture();
        let Some(complex) = definitions.get(&request.complex_name.0) else {
            return Outcome::Refused(RequestFailure::UnknownComplex(request.complex_name));
        };
        let Some(member) = complex.member_vector.iter().find(|member| member.flow_name == request.flow_name) else {
            return Outcome::Refused(RequestFailure::UnknownMember(request.flow_name));
        };
        if member.harness != Harness::Codex {
            return Outcome::Refused(RequestFailure::UnsupportedHarness(UnsupportedHarness {
                flow_name: member.flow_name.clone(), harness: member.harness.clone(),
            }));
        }
        if !matches!(member.tools, Tools::HarnessDefault) {
            return Outcome::Refused(RequestFailure::UnsupportedTools(UnsupportedTools {
                flow_name: member.flow_name.clone(), harness: member.harness.clone(),
            }));
        }

        let flow_id = FlowId(self.next_flow_id.fetch_add(1, Ordering::Relaxed));
        let workspace_path = match &member.workspace {
            signal_flow_nexus_version_1::Workspace::Path(path) => std::path::PathBuf::from(path),
            signal_flow_nexus_version_1::Workspace::Temporary => self.memory.state().join("flow-workspaces").join(flow_id.0.to_string()),
        };
        if std::fs::create_dir_all(&workspace_path).is_err() {
            return furnish_failure(flow_id, FurnishFailure::Unavailable);
        }
        let belief_workspace = signal_belief_nexus::Workspace(signal_belief_nexus::WorkspacePath(workspace_path.to_string_lossy().into_owned()));
        let furnish = emission::Query::Furnish(emission::Furnish_Data {
            flow_id: flow_id.clone(),
            furnish_request: signal_belief_nexus::FurnishRequest {
                workspace: belief_workspace.clone(), harness: signal_belief_nexus::Harness::Codex,
                module_name_vector: member.beliefs.module_name_vector.clone(),
            },
        });
        let furnished = match port.emit(furnish) {
            Err(EmissionTransport::Unavailable) => return furnish_failure(flow_id, FurnishFailure::Unavailable),
            Err(EmissionTransport::Uncertain) => return furnish_failure(flow_id, FurnishFailure::Uncertain),
            Ok(emission::Response::Furnished(emission::Furnished_Data::Succeeded(value))) if value.flow_id == flow_id => value.furnished_workspace,
            Ok(emission::Response::Furnished(emission::Furnished_Data::Refused(value))) if value.flow_id == flow_id => {
                return furnish_failure(flow_id, FurnishFailure::Refused(value.refusal));
            }
            Ok(emission::Response::Furnished(emission::Furnished_Data::Unavailable(value))) if value == flow_id => {
                return furnish_failure(flow_id, FurnishFailure::Unavailable);
            }
            Ok(emission::Response::Furnished(emission::Furnished_Data::Uncertain(value))) if value == flow_id => {
                return furnish_failure(flow_id, FurnishFailure::Uncertain);
            }
            _ => return furnish_failure(flow_id, FurnishFailure::Uncertain),
        };
        if member.beliefs.missing == Missing::Refuse && !furnished.module_name_vector.is_empty() {
            return Outcome::Refused(RequestFailure::MissingBeliefs(MissingBeliefs {
                flow_id, module_name_vector: furnished.module_name_vector,
            }));
        }
        if furnished.workspace != belief_workspace {
            return furnish_failure(flow_id, FurnishFailure::Uncertain);
        }
        let root = (furnished.workspace.0).0;
        if !std::path::Path::new(&(furnished.furnished_paths.system_prompt_path.0)).is_file()
            || !std::path::Path::new(&(furnished.furnished_paths.skills_path.0)).is_dir()
        { return furnish_failure(flow_id, FurnishFailure::Uncertain); }

        let create = emission::Query::Open(emission::Open_Data::Create(emission::Open_Data_Create_Data {
            flow_id: flow_id.clone(), workspace: signal_herdr_nexus::Workspace {
                session: self.session.clone(), root: signal_herdr_nexus::Root(root), variable_vector: vec![],
            },
        }));
        let pane = match opened(port.emit(create), &flow_id) {
            Ok(signal_herdr_nexus::Outcome::Created(pane)) => pane,
            Ok(outcome) => return Outcome::Refused(RequestFailure::CreateFailed(CreateFailed { flow_id, outcome })),
            Err(outcome) => return Outcome::Refused(RequestFailure::CreateFailed(CreateFailed { flow_id, outcome })),
        };

        let start = emission::Query::Open(emission::Open_Data::Start(emission::Open_Data_Start_Data {
            flow_id: flow_id.clone(), pane: pane.clone(),
            command: signal_herdr_nexus::Command {
                agent_name: signal_herdr_nexus::AgentName(format!("flow-{}", flow_id.0)),
                agent_kind: signal_herdr_nexus::AgentKind::Codex,
                executable: signal_herdr_nexus::Executable::Configured,
                string_vector: vec!["--model".into(), member.model_name.0.clone()],
            },
            label: signal_herdr_nexus::Label(format!("{}:{}", request.complex_name.0, request.flow_name.0)),
        }));
        match opened(port.emit(start), &flow_id) {
            Ok(signal_herdr_nexus::Outcome::Done) => {}
            Ok(outcome) | Err(outcome) => return Outcome::Refused(RequestFailure::StartFailed(StartFailed { flow_id, pane, outcome })),
        }

        let prompt = emission::Query::Open(emission::Open_Data::Prompt(emission::Open_Data_Prompt_Data {
            flow_id: flow_id.clone(), pane: pane.clone(), text: signal_herdr_nexus::Text(request.brief.0),
        }));
        match opened(port.emit(prompt), &flow_id) {
            Ok(signal_herdr_nexus::Outcome::Done) => Outcome::Launched(flow_id),
            Ok(outcome) | Err(outcome) => Outcome::Refused(RequestFailure::PromptFailed(PromptFailed { flow_id, pane, outcome })),
        }
    }
}

fn register_failure(register_failure: RegisterFailure) -> Outcome {
    Outcome::Refused(RequestFailure::RegisterFailed(RegisterFailed(register_failure)))
}

fn furnish_failure(flow_id: FlowId, furnish_failure: FurnishFailure) -> Outcome {
    Outcome::Refused(RequestFailure::FurnishFailed(FurnishFailed { flow_id, furnish_failure }))
}

fn opened(result: Result<emission::Response, EmissionTransport>, flow_id: &FlowId) -> Result<signal_herdr_nexus::Outcome, signal_herdr_nexus::Outcome> {
    match result {
        Ok(emission::Response::Opened(value)) if &value.flow_id == flow_id => Ok(value.outcome),
        Err(EmissionTransport::Unavailable) => Err(signal_herdr_nexus::Outcome::Unavailable(signal_herdr_nexus::TransportRefusal {
            socket_path: signal_herdr_nexus::SocketPath("flow-translator.sock".into()),
            message: signal_herdr_nexus::Message("request was not completely submitted".into()),
        })),
        _ => Err(signal_herdr_nexus::Outcome::Uncertain),
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

    struct NoEmission;
    impl EmissionPort for NoEmission {
        fn emit(&mut self, _: emission::Query) -> Result<emission::Response, EmissionTransport> { panic!("definition operation emitted") }
    }

    struct SuccessfulLaunch { steps: Vec<&'static str>, missing: Vec<signal_belief_nexus::ModuleName> }
    impl EmissionPort for SuccessfulLaunch {
        fn emit(&mut self, query: emission::Query) -> Result<emission::Response, EmissionTransport> {
            match query {
                emission::Query::Register(_) => panic!("launch emitted Register"),
                emission::Query::Furnish(value) => {
                    self.steps.push("furnish");
                    let root = ((value.furnish_request.workspace.0).0).clone();
                    let prompt = std::path::Path::new(&root).join(".belief/current/instructions.md");
                    let skills = std::path::Path::new(&root).join(".agents/skills");
                    std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
                    std::fs::create_dir_all(&skills).unwrap();
                    std::fs::write(&prompt, "instructions").unwrap();
                    Ok(emission::Response::Furnished(emission::Furnished_Data::Succeeded(
                        emission::Furnished_Data_Succeeded_Data {
                            flow_id: value.flow_id,
                            furnished_workspace: signal_belief_nexus::FurnishedWorkspace {
                                workspace: value.furnish_request.workspace,
                                generation_id: signal_belief_nexus::GenerationId("one".into()),
                                publication_revision: signal_belief_nexus::PublicationRevision(1),
                                furnished_paths: signal_belief_nexus::FurnishedPaths {
                                    system_prompt_path: signal_belief_nexus::SystemPromptPath(prompt.to_string_lossy().into_owned()),
                                    skills_path: signal_belief_nexus::SkillsPath(skills.to_string_lossy().into_owned()),
                                }, found_vector: vec![], module_name_vector: self.missing.clone(),
                            },
                        },
                    )))
                }
                emission::Query::Open(emission::Open_Data::Create(value)) => {
                    self.steps.push("create");
                    Ok(emission::Response::Opened(emission::Opened_Data {
                        flow_id: value.flow_id,
                        outcome: signal_herdr_nexus::Outcome::Created(signal_herdr_nexus::Pane {
                            session: value.workspace.session, pane_id: signal_herdr_nexus::PaneId("w:p".into()),
                        }),
                    }))
                }
                emission::Query::Open(emission::Open_Data::Start(value)) => {
                    self.steps.push("start");
                    assert_eq!(value.command.agent_kind, signal_herdr_nexus::AgentKind::Codex);
                    assert_eq!(value.command.string_vector, ["--model", "configured"]);
                    Ok(emission::Response::Opened(emission::Opened_Data { flow_id: value.flow_id, outcome: signal_herdr_nexus::Outcome::Done }))
                }
                emission::Query::Open(emission::Open_Data::Prompt(value)) => {
                    self.steps.push("prompt");
                    assert_eq!(value.text.0, "begin");
                    Ok(emission::Response::Opened(emission::Opened_Data { flow_id: value.flow_id, outcome: signal_herdr_nexus::Outcome::Done }))
                }
            }
        }
    }

    struct RegisterPort { repository: Option<signal_belief_nexus::Repository> }
    impl EmissionPort for RegisterPort {
        fn emit(&mut self, query: emission::Query) -> Result<emission::Response, EmissionTransport> {
            let emission::Query::Register(repository) = query else { panic!("expected Register emission") };
            self.repository = Some(repository.clone());
            Ok(emission::Response::Registered(emission::Registered_Data::Succeeded(signal_belief_nexus::RepositoryState {
                repository, source_revision: signal_belief_nexus::SourceRevision("revision".into()),
            })))
        }
    }

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
    fn register_propagates_repository_and_typed_state() {
        let fixture = Fixture::new();
        let core = Core::start(DefinitionMemory::open(&fixture.0).unwrap(), signal_herdr_nexus::Session("test".into())).unwrap();
        let repository = signal_belief_nexus::Repository(signal_belief_nexus::RepositoryPath("/fixture/repository".into()));
        let expected = signal_belief_nexus::RepositoryState { repository: repository.clone(), source_revision: signal_belief_nexus::SourceRevision("revision".into()) };
        let mut port = RegisterPort { repository: None };
        assert_eq!(core.execute(Operation::Registering(repository.clone()), &mut port), Outcome::Registered(expected));
        assert_eq!(port.repository, Some(repository));
    }

    #[test]
    fn define_restart_describe_and_list() {
        let fixture = Fixture::new();
        let core = Core::start(DefinitionMemory::open(&fixture.0).unwrap(), signal_herdr_nexus::Session("test".into())).unwrap();
        let mut emission = NoEmission;
        let value = complex("one", "configured");
        assert_eq!(core.execute(Operation::Defining(value.clone()), &mut emission), Outcome::Defined(ComplexName("one".into())));
        drop(core);
        let restarted = Core::start(DefinitionMemory::open(&fixture.0).unwrap(), signal_herdr_nexus::Session("test".into())).unwrap();
        assert_eq!(restarted.execute(Operation::Describing(ComplexName("one".into())), &mut emission), Outcome::Described(value));
        assert_eq!(restarted.execute(Operation::Listing, &mut emission), Outcome::Listed(vec![ComplexName("one".into())]));
    }

    #[test]
    fn invalid_define_preserves_the_published_and_persisted_value() {
        let fixture = Fixture::new();
        let core = Core::start(DefinitionMemory::open(&fixture.0).unwrap(), signal_herdr_nexus::Session("test".into())).unwrap();
        let mut emission = NoEmission;
        let original = complex("one", "original");
        assert!(matches!(core.execute(Operation::Defining(original.clone()), &mut emission), Outcome::Defined(_)));
        let mut invalid = complex("one", "replacement");
        invalid.speaker = Speaker(FlowName("missing".into()));
        assert!(matches!(core.execute(Operation::Defining(invalid), &mut emission), Outcome::Refused(RequestFailure::Invalid(_))));
        drop(core);
        let restarted = Core::start(DefinitionMemory::open(&fixture.0).unwrap(), signal_herdr_nexus::Session("test".into())).unwrap();
        assert_eq!(restarted.execute(Operation::Describing(ComplexName("one".into())), &mut emission), Outcome::Described(original));
    }

    #[test]
    fn launch_furnishes_then_creates_starts_and_prompts() {
        let fixture = Fixture::new();
        let core = Core::start(DefinitionMemory::open(&fixture.0).unwrap(), signal_herdr_nexus::Session("test".into())).unwrap();
        let mut no_emission = NoEmission;
        let value = complex("one", "configured");
        assert!(matches!(core.execute(Operation::Defining(value.clone()), &mut no_emission), Outcome::Defined(_)));
        let mut launch = SuccessfulLaunch { steps: vec![], missing: vec![] };
        assert_eq!(core.execute(Operation::Launching(LaunchRequest {
            complex_name: value.complex_name, flow_name: value.member_vector[0].flow_name.clone(),
            brief: signal_flow_nexus_version_1::Brief("begin".into()),
        }), &mut launch), Outcome::Launched(FlowId(1)));
        assert_eq!(launch.steps, ["furnish", "create", "start", "prompt"]);
    }

    #[test]
    fn required_missing_belief_refuses_before_create() {
        let fixture = Fixture::new();
        let core = Core::start(DefinitionMemory::open(&fixture.0).unwrap(), signal_herdr_nexus::Session("test".into())).unwrap();
        let mut no_emission = NoEmission;
        let value = complex("one", "configured");
        assert!(matches!(core.execute(Operation::Defining(value.clone()), &mut no_emission), Outcome::Defined(_)));
        let missing = signal_belief_nexus::ModuleName { kind: signal_belief_nexus::Kind::Vision, subject: signal_belief_nexus::Subject("missing".into()) };
        let mut launch = SuccessfulLaunch { steps: vec![], missing: vec![missing] };
        assert!(matches!(core.execute(Operation::Launching(LaunchRequest {
            complex_name: value.complex_name, flow_name: value.member_vector[0].flow_name.clone(),
            brief: signal_flow_nexus_version_1::Brief("begin".into()),
        }), &mut launch), Outcome::Refused(RequestFailure::MissingBeliefs(_))));
        assert_eq!(launch.steps, ["furnish"]);
    }
}
