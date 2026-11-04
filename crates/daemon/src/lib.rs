//! Ordinary binary Flow core server.

use flow_nexus_core::{memory::DefinitionMemory, Core, Operation, Outcome};
use flow_nexus_wire as wire;
use shared_translator::{ByteViewable, Restorable, Signal, Signalizable};
use std::os::unix::net::UnixStream;

pub fn run(bootstrap: &wire::CoreBootstrap) -> Result<(), Box<dyn std::error::Error>> {
    let admission = wire::validate_admission(&bootstrap.admission)?;
    let runtime = shared_translator::placement::runtime_path(&bootstrap.place);
    let state = shared_translator::placement::state_path(&bootstrap.place);
    if !runtime.is_dir() || !state.is_dir() { return Err("Flow Place directories must exist".into()); }
    let core = Core::start(DefinitionMemory::open(&state)?)?;
    let peer = shared_translator::unix::LivePeer::capture(bootstrap.peer_identity.clone())?;
    let listener = wire::BoundUnixListener::bind(runtime.join(wire::CORE_SOCKET))?;
    loop {
        let (stream, _) = shared_translator::unix::accept_with_retry(listener.listener(), admission.retry_delay)?;
        if !shared_translator::unix::peer_credentials(&stream).is_ok_and(|value| peer.matches(&value)) { continue; }
        let _ = serve_call(stream, admission, &core);
    }
}

fn serve_call(
    stream: UnixStream,
    admission: shared_translator::unix::Admission,
    core: &Core,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut bounded = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    let frame = shared_translator::unix::read_frame(&mut bounded, admission.frame_capacity)?;
    if frame.lane != wire::PUBLIC_SIGNAL_LANE || frame.kind != wire::REQUEST_KIND {
        return Err("wrong Flow signal lane".into());
    }
    let call = Signal::<wire::Call>::from(frame.payload).restore()?;
    let response = execute(core, call.input);
    let signal = response.signalize()?;
    shared_translator::unix::write_frame(&mut bounded, &shared_translator::unix::Frame {
        lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::ANSWER_KIND, payload: signal.bytes().to_vec(),
    }, admission.frame_capacity)?;
    Ok(())
}

fn execute(core: &Core, query: signal_flow_nexus_version_1::Query) -> signal_flow_nexus_version_1::Response {
    let outcome = match query {
        signal_flow_nexus_version_1::Query::Define(value) => core.execute(Operation::Defining(value)),
        signal_flow_nexus_version_1::Query::Describe(name) => core.execute(Operation::Describing(name)),
        signal_flow_nexus_version_1::Query::Definitions => core.execute(Operation::Listing),
    };
    match outcome {
        Outcome::Defined(value) => signal_flow_nexus_version_1::Response::Defined(value),
        Outcome::Described(value) => signal_flow_nexus_version_1::Response::Described(value),
        Outcome::Listed(value) => signal_flow_nexus_version_1::Response::Listed(value),
        Outcome::Refused(value) => signal_flow_nexus_version_1::Response::Refused(value),
    }
}
