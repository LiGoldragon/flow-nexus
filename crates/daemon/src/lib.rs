//! Ordinary binary Flow core server.

use flow_nexus_core::{memory::DefinitionMemory, Core, EmissionPort, EmissionTransport, Operation, Outcome};
use flow_nexus_emission::generated::emission;
use flow_nexus_wire as wire;
use shared_translator::{ByteViewable, Restorable, Signal, Signalizable};
use std::os::unix::net::UnixStream;

pub fn run(bootstrap: &wire::CoreBootstrap) -> Result<(), Box<dyn std::error::Error>> {
    let admission = wire::validate_admission(&bootstrap.admission)?;
    let runtime = shared_translator::placement::runtime_path(&bootstrap.place);
    let state = shared_translator::placement::state_path(&bootstrap.place);
    if !runtime.is_dir() || !state.is_dir() { return Err("Flow Place directories must exist".into()); }
    let core = Core::start(
        DefinitionMemory::open(&state)?,
        signal_herdr_nexus::Session((bootstrap.place.metaflow_name.0).clone()),
    )?;
    let mut emission = SocketEmission { bootstrap };
    let peer = shared_translator::unix::LivePeer::capture(bootstrap.peer_identity.clone())?;
    let listener = wire::BoundUnixListener::bind(runtime.join(wire::CORE_SOCKET))?;
    loop {
        let (stream, _) = shared_translator::unix::accept_with_retry(listener.listener(), admission.retry_delay)?;
        if !shared_translator::unix::peer_credentials(&stream).is_ok_and(|value| peer.matches(&value)) { continue; }
        let _ = serve_call(stream, admission, &core, &mut emission);
    }
}

fn serve_call(
    stream: UnixStream,
    admission: shared_translator::unix::Admission,
    core: &Core,
    emission: &mut impl EmissionPort,
) -> Result<(), Box<dyn std::error::Error>> {
    serve_request(stream, admission, |query| execute(core, emission, query))
}

fn serve_request(
    stream: UnixStream,
    admission: shared_translator::unix::Admission,
    handle: impl FnOnce(signal_flow_nexus_version_1::Query) -> signal_flow_nexus_version_1::Response,
) -> Result<(), Box<dyn std::error::Error>> {
    let response_stream = stream.try_clone()?;
    let mut request_stream = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    let frame = shared_translator::unix::read_frame(&mut request_stream, admission.frame_capacity)?;
    if frame.lane != wire::PUBLIC_SIGNAL_LANE || frame.kind != wire::REQUEST_KIND {
        return Err("wrong Flow signal lane".into());
    }
    drop(request_stream);
    let call = Signal::<wire::Call>::from(frame.payload).restore()?;
    let response = handle(call.input);
    let signal = response.signalize()?;
    let mut response_stream = shared_translator::unix::DeadlineStream::new(response_stream, admission.deadline)?;
    shared_translator::unix::write_frame(&mut response_stream, &shared_translator::unix::Frame {
        lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::ANSWER_KIND, payload: signal.bytes().to_vec(),
    }, admission.frame_capacity)?;
    Ok(())
}

fn execute(core: &Core, emission: &mut impl EmissionPort, query: signal_flow_nexus_version_1::Query) -> signal_flow_nexus_version_1::Response {
    let outcome = match query {
        signal_flow_nexus_version_1::Query::Register(value) => core.execute(Operation::Registering(value), emission),
        signal_flow_nexus_version_1::Query::Define(value) => core.execute(Operation::Defining(value), emission),
        signal_flow_nexus_version_1::Query::Describe(name) => core.execute(Operation::Describing(name), emission),
        signal_flow_nexus_version_1::Query::Definitions => core.execute(Operation::Listing, emission),
        signal_flow_nexus_version_1::Query::Launch(value) => core.execute(Operation::Launching(value), emission),
    };
    match outcome {
        Outcome::Registered(value) => signal_flow_nexus_version_1::Response::Registered(value),
        Outcome::Defined(value) => signal_flow_nexus_version_1::Response::Defined(value),
        Outcome::Described(value) => signal_flow_nexus_version_1::Response::Described(value),
        Outcome::Listed(value) => signal_flow_nexus_version_1::Response::Listed(value),
        Outcome::Refused(value) => signal_flow_nexus_version_1::Response::Refused(value),
        Outcome::Launched(value) => signal_flow_nexus_version_1::Response::Launched(value),
    }
}

struct SocketEmission<'a> { bootstrap: &'a wire::CoreBootstrap }

impl EmissionPort for SocketEmission<'_> {
    fn emit(&mut self, query: emission::Query) -> Result<emission::Response, EmissionTransport> {
        let admission = wire::validate_admission(&self.bootstrap.admission).map_err(|_| EmissionTransport::Unavailable)?;
        let path = shared_translator::placement::runtime_path(&self.bootstrap.place).join(wire::TRANSLATOR_SOCKET);
        let stream = UnixStream::connect(path).map_err(|_| EmissionTransport::Unavailable)?;
        let read = stream.try_clone().map_err(|_| EmissionTransport::Unavailable)?;
        let mut write = shared_translator::unix::DeadlineStream::new(stream, admission.deadline).map_err(|_| EmissionTransport::Unavailable)?;
        let signal = query.signalize().map_err(|_| EmissionTransport::Unavailable)?;
        let request = shared_translator::unix::Frame {
            lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::REQUEST_KIND, payload: signal.bytes().to_vec(),
        };
        let complete_bytes = request.payload.len().checked_add(shared_translator::unix::HEADER_BYTES)
            .ok_or(EmissionTransport::Unavailable)?;
        if let Err(failure) = shared_translator::unix::write_frame_witness(&mut write, &request, admission.frame_capacity) {
            return Err(if failure.written < complete_bytes {
                EmissionTransport::Unavailable
            } else {
                EmissionTransport::Uncertain
            });
        }
        drop(write);
        let frame = shared_translator::unix::read_frame_after_idle(read, admission.deadline, admission.frame_capacity)
            .map_err(|_| EmissionTransport::Uncertain)?.ok_or(EmissionTransport::Uncertain)?;
        if frame.lane != wire::PUBLIC_SIGNAL_LANE || frame.kind != wire::ANSWER_KIND { return Err(EmissionTransport::Uncertain); }
        Signal::<emission::Response>::from(frame.payload).restore().map_err(|_| EmissionTransport::Uncertain)
    }
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    use shared_translator::{Caller, ProcessId, UserId};
    use std::time::Duration;

    fn admission() -> shared_translator::unix::Admission {
        shared_translator::unix::Admission::new(Duration::from_millis(20), 8192, Duration::from_millis(1)).unwrap()
    }

    #[test]
    fn external_operation_may_outlive_frame_progress_deadline() {
        let (client, server) = UnixStream::pair().unwrap();
        let server_thread = std::thread::spawn(move || {
            serve_request(server, admission(), |_| {
                std::thread::sleep(Duration::from_millis(80));
                signal_flow_nexus_version_1::Response::Listed(vec![])
            }).unwrap();
        });
        let read = client.try_clone().unwrap();
        let call = wire::Call {
            caller: Caller { process_id: ProcessId(1), user_id: UserId(1) },
            input: signal_flow_nexus_version_1::Query::Definitions,
        }.signalize().unwrap();
        let mut write = shared_translator::unix::DeadlineStream::new(client, admission().deadline).unwrap();
        shared_translator::unix::write_frame(&mut write, &shared_translator::unix::Frame {
            lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::REQUEST_KIND, payload: call.bytes().to_vec(),
        }, admission().frame_capacity).unwrap();
        drop(write);
        let response = shared_translator::unix::read_frame_after_idle(read, admission().deadline, admission().frame_capacity)
            .unwrap().unwrap();
        assert_eq!(response.kind, wire::ANSWER_KIND);
        assert_eq!(Signal::<signal_flow_nexus_version_1::Response>::from(response.payload).restore().unwrap(),
            signal_flow_nexus_version_1::Response::Listed(vec![]));
        server_thread.join().unwrap();
    }
}
