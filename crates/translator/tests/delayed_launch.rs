use flow_nexus_wire as wire;
use shared_translator::{ByteViewable, Restorable, Signal, Signalizable};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::{Duration, Instant};

fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !path.exists() {
        assert!(Instant::now() < deadline, "socket did not appear: {}", path.display());
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn read_plain<T>(stream: &mut UnixStream) -> T
where
    T: rkyv::Archive,
    T::Archived: for<'a> rkyv::bytecheck::CheckBytes<rkyv::rancor::Strategy<rkyv::validation::Validator<rkyv::validation::archive::ArchiveValidator<'a>, rkyv::validation::shared::SharedValidator>, rkyv::rancor::Error>>
        + rkyv::Deserialize<T, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    let mut prefix = [0; 4];
    stream.read_exact(&mut prefix).unwrap();
    let mut body = vec![0; u32::from_be_bytes(prefix) as usize];
    stream.read_exact(&mut body).unwrap();
    Signal::<T>::from(body).restore().unwrap()
}

fn write_plain<T: shared_translator::Signalizable>(stream: &mut UnixStream, value: T) {
    let signal = value.signalize().unwrap();
    stream.write_all(&(signal.bytes().len() as u32).to_be_bytes()).unwrap();
    stream.write_all(signal.bytes()).unwrap();
}

#[test]
fn define_and_launch_survive_quiet_external_execution() {
    let root = std::env::temp_dir().join(format!("flow-full-delay-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let runtime = root.join("runtime"); let state = root.join("state"); let workspace = root.join("workspace");
    std::fs::create_dir_all(&runtime).unwrap(); std::fs::create_dir_all(&state).unwrap(); std::fs::create_dir_all(&workspace).unwrap();
    let place = shared_translator::Place { metaflow_name: shared_translator::MetaflowName("fixture".into()), runtime: shared_translator::Runtime(runtime.to_string_lossy().into_owned()), state: shared_translator::State(state.to_string_lossy().into_owned()) };
    let admission_value = wire::AdmissionValue { frame_deadline_milliseconds: 20, frame_capacity_bytes: 65536, retry_delay_milliseconds: 1 };
    let admission = wire::validate_admission(&admission_value).unwrap();
    let caller = shared_translator::PeerIdentity { user_id: shared_translator::UserId(i64::from(nix::unistd::Uid::current().as_raw())), process_id: shared_translator::ProcessId(i64::from(std::process::id())) };

    let belief = UnixListener::bind(runtime.join("belief.sock")).unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = belief.accept().unwrap();
        let call: signal_belief_nexus::Call = read_plain(&mut stream);
        let signal_belief_nexus::Query::Furnish(request) = call.query else { panic!("expected Furnish") };
        std::thread::sleep(Duration::from_millis(80));
        let root = ((request.workspace.0).0).clone();
        let prompt = Path::new(&root).join(".belief/current/instructions.md");
        let skills = Path::new(&root).join(".agents/skills");
        std::fs::create_dir_all(prompt.parent().unwrap()).unwrap(); std::fs::create_dir_all(&skills).unwrap(); std::fs::write(&prompt, "fixture").unwrap();
        write_plain(&mut stream, signal_belief_nexus::Response::Furnished(signal_belief_nexus::FurnishedWorkspace { workspace: request.workspace, generation_id: signal_belief_nexus::GenerationId("one".into()), publication_revision: signal_belief_nexus::PublicationRevision(1), furnished_paths: signal_belief_nexus::FurnishedPaths { system_prompt_path: signal_belief_nexus::SystemPromptPath(prompt.to_string_lossy().into_owned()), skills_path: signal_belief_nexus::SkillsPath(skills.to_string_lossy().into_owned()) }, found_vector: vec![], module_name_vector: vec![] }));
    });

    let herdr = UnixListener::bind(runtime.join("herdr.sock")).unwrap();
    std::thread::spawn(move || {
        for step in 0..3 {
            let (stream, _) = herdr.accept().unwrap();
            let read = stream.try_clone().unwrap();
            let mut bounded = shared_translator::unix::DeadlineStream::new(read, Duration::from_secs(1)).unwrap();
            let frame = shared_translator::unix::read_frame(&mut bounded, 65536).unwrap();
            let call = Signal::<herdr_nexus_wire::public_call::Query>::from(frame.payload).restore().unwrap();
            let herdr_nexus_wire::public_call::Query::Call(value) = call;
            let outcome = match (step, value.input.0) {
                (0, signal_herdr_nexus::Query::Create(workspace)) => signal_herdr_nexus::Response::Created(signal_herdr_nexus::Outcome::Created(signal_herdr_nexus::Pane { session: workspace.session, pane_id: signal_herdr_nexus::PaneId("w:p".into()) })),
                (1, signal_herdr_nexus::Query::Start(_)) => signal_herdr_nexus::Response::Started(signal_herdr_nexus::Outcome::Done),
                (2, signal_herdr_nexus::Query::Prompt(_)) => signal_herdr_nexus::Response::Prompted(signal_herdr_nexus::Outcome::Done),
                _ => panic!("unexpected Herdr step"),
            };
            let response = herdr_nexus_wire::public_call::Response::Answered(herdr_nexus_wire::public_payload::Output(outcome)).signalize().unwrap();
            let mut bounded = shared_translator::unix::DeadlineStream::new(stream, Duration::from_secs(1)).unwrap();
            shared_translator::unix::write_frame(&mut bounded, &shared_translator::unix::Frame { lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::ANSWER_KIND, payload: response.bytes().to_vec() }, 65536).unwrap();
        }
    });

    let core_bootstrap = wire::CoreBootstrap { place: place.clone(), admission: admission_value.clone(), peer_identity: caller.clone() };
    let translator_bootstrap = wire::TranslatorBootstrap { place: place.clone(), admission: admission_value, text_budget: wire::TextBudget { node_limit: 65536, depth_limit: 64 }, peer_identity: caller };
    std::thread::spawn(move || flow_nexus_daemon::run(&core_bootstrap).unwrap());
    std::thread::spawn(move || flow_nexus_translator::run(&translator_bootstrap).unwrap());
    wait_for(&runtime.join(wire::CORE_SOCKET)); wait_for(&runtime.join(wire::TRANSLATOR_SOCKET));

    let define = format!("Define.{{ demo [ {{ worker Codex configured Medium Ephemeral Path.«{}» {{ [] Refuse }} HarnessDefault [] }} ] worker }}", workspace.display());
    assert_eq!(flow_nexus_translator::request(&place, admission, &define).unwrap(), "Defined.demo");
    let started = Instant::now();
    assert_eq!(flow_nexus_translator::request(&place, admission, "Launch.{ demo worker begin }").unwrap(), "Launched.1");
    assert!(started.elapsed() >= Duration::from_millis(80));
}
