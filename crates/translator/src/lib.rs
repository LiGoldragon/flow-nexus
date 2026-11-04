//! Text edge and paired ordinary transport for Flow.

use datom_codec::{Actualizing, Budget, Potential};
use flow_nexus_emission::generated::emission;
use flow_nexus_wire as wire;
use protos::ReaderBudget;
use shared_translator::{ByteViewable, Caller, Restorable, Signal, Signalizable};
use signal_flow_nexus_version_1::{Query, Response};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub fn decode(text: &str, budget: &mut Budget) -> Result<Query, String> {
    Potential::<Query>::from(text).actualize(budget).map_err(|error| format!("{error:?}"))
}
pub fn encode(response: &Response) -> String { shared_translator::encode(response) }

fn budget(value: &wire::TextBudget) -> Result<Budget, &'static str> {
    let nodes = usize::try_from(value.node_limit).ok().filter(|value| *value > 0).ok_or("node limit")?;
    if value.depth_limit <= 0 { return Err("depth limit"); }
    Ok(Budget { remaining: value.node_limit, reader: ReaderBudget { remaining: nodes }, depth: 0, maximum_depth: value.depth_limit })
}

pub fn run(bootstrap: &wire::TranslatorBootstrap) -> Result<(), Box<dyn std::error::Error>> {
    let admission = wire::validate_admission(&bootstrap.admission)?;
    let runtime = shared_translator::placement::runtime_path(&bootstrap.place);
    if !runtime.is_dir() { return Err("Flow runtime directory must exist".into()); }
    let core_peer = shared_translator::unix::LivePeer::capture(bootstrap.peer_identity.clone())?;
    let listener = wire::BoundUnixListener::bind(runtime.join(wire::TRANSLATOR_SOCKET))?;
    let login = i64::from(nix::unistd::Uid::current().as_raw());
    loop {
        let (stream, _) = shared_translator::unix::accept_with_retry(listener.listener(), admission.retry_delay)?;
        let credentials = match shared_translator::unix::peer_credentials(&stream) {
            Ok(value) if value.user_id.0 == login => value,
            _ => continue,
        };
        let _ = serve_connection(stream, credentials, bootstrap, admission, &core_peer);
    }
}

fn serve_connection(
    stream: UnixStream,
    credentials: shared_translator::unix::PeerCredentials,
    bootstrap: &wire::TranslatorBootstrap,
    admission: shared_translator::unix::Admission,
    core_peer: &shared_translator::unix::LivePeer,
) -> Result<(), Box<dyn std::error::Error>> {
    if !core_peer.is_live() { return Err("paired core ended".into()); }
    let response_stream = stream.try_clone()?;
    let mut client = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    let frame = shared_translator::unix::read_frame(&mut client, admission.frame_capacity)?;
    if frame.kind != wire::REQUEST_KIND { return Err("wrong Flow request kind".into()); }
    let response = match frame.lane {
        wire::PUBLIC_TEXT_LANE => {
            let text = std::str::from_utf8(&frame.payload)?;
            let query = decode(text, &mut budget(&bootstrap.text_budget)?)?;
            let call = wire::Call { caller: Caller { process_id: credentials.process_id, user_id: credentials.user_id }, input: query };
            shared_translator::unix::Frame {
                lane: wire::PUBLIC_TEXT_LANE, kind: wire::ANSWER_KIND,
                payload: encode(&binary_call(bootstrap, admission, call)?).into_bytes(),
            }
        }
        wire::PUBLIC_SIGNAL_LANE if core_peer.matches(&credentials) => {
            let query = Signal::<emission::Query>::from(frame.payload).restore()?;
            let response = execute_emission(bootstrap, admission, query);
            let archived = response.signalize()?;
            shared_translator::unix::Frame {
                lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::ANSWER_KIND, payload: archived.bytes().to_vec(),
            }
        }
        _ => return Err("wrong Flow lane or peer".into()),
    };
    let mut client = shared_translator::unix::DeadlineStream::new(response_stream, admission.deadline)?;
    shared_translator::unix::write_frame(&mut client, &response, admission.frame_capacity)?;
    Ok(())
}

fn execute_emission(
    bootstrap: &wire::TranslatorBootstrap,
    admission: shared_translator::unix::Admission,
    query: emission::Query,
) -> emission::Response {
    match query {
        emission::Query::Furnish(value) => {
            let call = signal_belief_nexus::Call { caller: own_caller(), query: signal_belief_nexus::Query::Furnish(value.furnish_request) };
            match belief_call(bootstrap, admission, call) {
                External::Answered(signal_belief_nexus::Response::Furnished(workspace)) => emission::Response::Furnished(emission::Furnished_Data::Succeeded(emission::Furnished_Data_Succeeded_Data { flow_id: value.flow_id, furnished_workspace: workspace })),
                External::Answered(signal_belief_nexus::Response::Refused(refusal)) => emission::Response::Furnished(emission::Furnished_Data::Refused(emission::Furnished_Data_Refused_Data { flow_id: value.flow_id, refusal })),
                External::Unavailable => emission::Response::Furnished(emission::Furnished_Data::Unavailable(value.flow_id)),
                _ => emission::Response::Furnished(emission::Furnished_Data::Uncertain(value.flow_id)),
            }
        }
        emission::Query::Open(value) => {
            let (flow_id, query, expected) = match value {
                emission::Open_Data::Create(value) => (value.flow_id, signal_herdr_nexus::Query::Create(value.workspace), HerdrAnswer::Created),
                emission::Open_Data::Start(value) => (value.flow_id, signal_herdr_nexus::Query::Start(signal_herdr_nexus::Start_Data { pane: value.pane, command: value.command, label: value.label }), HerdrAnswer::Started),
                emission::Open_Data::Prompt(value) => (value.flow_id, signal_herdr_nexus::Query::Prompt(signal_herdr_nexus::Prompt_Data { pane: value.pane, text: value.text }), HerdrAnswer::Prompted),
            };
            let outcome = match herdr_call(bootstrap, admission, query) {
                External::Answered(herdr_nexus_wire::public_call::Response::Answered(value)) => match (expected, value.0) {
                    (HerdrAnswer::Created, signal_herdr_nexus::Response::Created(outcome))
                    | (HerdrAnswer::Started, signal_herdr_nexus::Response::Started(outcome))
                    | (HerdrAnswer::Prompted, signal_herdr_nexus::Response::Prompted(outcome)) => outcome,
                    _ => signal_herdr_nexus::Outcome::Uncertain,
                },
                External::Unavailable => signal_herdr_nexus::Outcome::Unavailable(signal_herdr_nexus::TransportRefusal {
                    socket_path: signal_herdr_nexus::SocketPath(herdr_path(bootstrap).to_string_lossy().into_owned()),
                    message: signal_herdr_nexus::Message("request was not completely submitted".into()),
                }),
                _ => signal_herdr_nexus::Outcome::Uncertain,
            };
            emission::Response::Opened(emission::Opened_Data { flow_id, outcome })
        }
    }
}

#[derive(Clone, Copy)]
enum HerdrAnswer { Created, Started, Prompted }
enum External<T> { Answered(T), Unavailable, Uncertain }

fn own_caller() -> Caller {
    Caller {
        process_id: shared_translator::ProcessId(i64::from(std::process::id())),
        user_id: shared_translator::UserId(i64::from(nix::unistd::Uid::current().as_raw())),
    }
}

fn belief_path(bootstrap: &wire::TranslatorBootstrap) -> std::path::PathBuf {
    shared_translator::placement::runtime_path(&bootstrap.place).join("belief.sock")
}
fn herdr_path(bootstrap: &wire::TranslatorBootstrap) -> std::path::PathBuf {
    shared_translator::placement::runtime_path(&bootstrap.place).join("herdr.sock")
}

fn belief_call(
    bootstrap: &wire::TranslatorBootstrap,
    admission: shared_translator::unix::Admission,
    call: signal_belief_nexus::Call,
) -> External<signal_belief_nexus::Response> {
    let signal = match call.signalize() { Ok(value) => value, Err(_) => return External::Unavailable };
    let stream = match UnixStream::connect(belief_path(bootstrap)) { Ok(value) => value, Err(_) => return External::Unavailable };
    let read = match stream.try_clone() { Ok(value) => value, Err(_) => return External::Unavailable };
    let mut write = match shared_translator::unix::DeadlineStream::new(stream, admission.deadline) { Ok(value) => value, Err(_) => return External::Unavailable };
    let bytes = signal.bytes();
    let Ok(length) = u32::try_from(bytes.len()) else { return External::Unavailable };
    if bytes.len() > admission.frame_capacity { return External::Unavailable; }
    if write.write_all(&length.to_be_bytes()).and_then(|()| write.write_all(bytes)).and_then(|()| write.flush()).is_err() { return External::Unavailable; }
    let Some(body) = read_signal_after_idle(read, admission) else { return External::Uncertain };
    match Signal::<signal_belief_nexus::Response>::from(body).restore() { Ok(value) => External::Answered(value), Err(_) => External::Uncertain }
}

fn herdr_call(
    bootstrap: &wire::TranslatorBootstrap,
    admission: shared_translator::unix::Admission,
    query: signal_herdr_nexus::Query,
) -> External<herdr_nexus_wire::public_call::Response> {
    let call = herdr_nexus_wire::public_call::Query::Call(herdr_nexus_wire::public_call::Call_Data {
        caller: own_caller(), input: herdr_nexus_wire::public_payload::Input(query),
    });
    let signal = match call.signalize() { Ok(value) => value, Err(_) => return External::Unavailable };
    let stream = match UnixStream::connect(herdr_path(bootstrap)) { Ok(value) => value, Err(_) => return External::Unavailable };
    let read = match stream.try_clone() { Ok(value) => value, Err(_) => return External::Unavailable };
    let mut write = match shared_translator::unix::DeadlineStream::new(stream, admission.deadline) { Ok(value) => value, Err(_) => return External::Unavailable };
    let request = shared_translator::unix::Frame {
        lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::REQUEST_KIND, payload: signal.bytes().to_vec(),
    };
    let Some(complete_bytes) = request.payload.len().checked_add(shared_translator::unix::HEADER_BYTES) else { return External::Unavailable };
    if let Err(failure) = shared_translator::unix::write_frame_witness(&mut write, &request, admission.frame_capacity) {
        return if failure.written < complete_bytes { External::Unavailable } else { External::Uncertain };
    }
    let frame = match shared_translator::unix::read_frame_after_idle(read, admission.deadline, admission.frame_capacity) {
        Ok(Some(value)) if value.lane == wire::PUBLIC_SIGNAL_LANE && value.kind == wire::ANSWER_KIND => value,
        _ => return External::Uncertain,
    };
    match Signal::<herdr_nexus_wire::public_call::Response>::from(frame.payload).restore() { Ok(value) => External::Answered(value), Err(_) => External::Uncertain }
}

fn read_signal_after_idle(stream: UnixStream, admission: shared_translator::unix::Admission) -> Option<Vec<u8>> {
    let mut stream = stream;
    stream.set_read_timeout(None).ok()?;
    let mut prefix = [0_u8; 4];
    if stream.read(&mut prefix[..1]).ok()? != 1 { return None; }
    let mut progressing = shared_translator::unix::DeadlineStream::new(stream, admission.deadline).ok()?;
    progressing.read_exact(&mut prefix[1..]).ok()?;
    let length = usize::try_from(u32::from_be_bytes(prefix)).ok()?;
    if length > admission.frame_capacity { return None; }
    let mut body = vec![0_u8; length];
    progressing.read_exact(&mut body).ok()?;
    Some(body)
}

fn binary_call(
    bootstrap: &wire::TranslatorBootstrap,
    admission: shared_translator::unix::Admission,
    call: wire::Call,
) -> Result<Response, Box<dyn std::error::Error>> {
    let path = shared_translator::placement::runtime_path(&bootstrap.place).join(wire::CORE_SOCKET);
    let stream = UnixStream::connect(path)?;
    let read = stream.try_clone()?;
    let mut bounded = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    let signal = call.signalize()?;
    shared_translator::unix::write_frame(&mut bounded, &shared_translator::unix::Frame {
        lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::REQUEST_KIND, payload: signal.bytes().to_vec(),
    }, admission.frame_capacity)?;
    drop(bounded);
    let frame = shared_translator::unix::read_frame_after_idle(read, admission.deadline, admission.frame_capacity)?
        .ok_or("Flow core closed before answering")?;
    if frame.lane != wire::PUBLIC_SIGNAL_LANE || frame.kind != wire::ANSWER_KIND { return Err("wrong Flow answer lane".into()); }
    Ok(Signal::<Response>::from(frame.payload).restore()?)
}

pub fn request(place: &shared_translator::Place, admission: shared_translator::unix::Admission, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let stream = UnixStream::connect(shared_translator::placement::runtime_path(place).join(wire::TRANSLATOR_SOCKET))?;
    let read = stream.try_clone()?;
    let mut bounded = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    shared_translator::unix::write_frame(&mut bounded, &shared_translator::unix::Frame {
        lane: wire::PUBLIC_TEXT_LANE, kind: wire::REQUEST_KIND, payload: text.as_bytes().to_vec(),
    }, admission.frame_capacity)?;
    drop(bounded);
    let frame = shared_translator::unix::read_frame_after_idle(read, admission.deadline, admission.frame_capacity)?
        .ok_or("Flow translator closed before answering")?;
    if frame.lane != wire::PUBLIC_TEXT_LANE || frame.kind != wire::ANSWER_KIND { return Err("wrong Flow text answer".into()); }
    Ok(String::from_utf8(frame.payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_flow_nexus_version_1::{ComplexName, Query, Response};
    fn fixture_budget() -> Budget { Budget { remaining: 4096, reader: ReaderBudget { remaining: 4096 }, depth: 0, maximum_depth: 32 } }
    #[test]
    fn public_text_round_trip() {
        assert_eq!(decode("Describe.main", &mut fixture_budget()).unwrap(), Query::Describe(ComplexName("main".into())));
        assert_eq!(encode(&Response::Listed(vec![])), "Listed.[]");
    }
}
