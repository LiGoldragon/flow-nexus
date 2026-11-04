//! Text edge and paired ordinary transport for Flow.

use datom_codec::{Actualizing, Budget, Potential};
use flow_nexus_wire as wire;
use protos::ReaderBudget;
use shared_translator::{ByteViewable, Caller, Restorable, Signal, Signalizable};
use signal_flow_nexus_version_1::{Query, Response};
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
        let _ = serve_text(stream, credentials, bootstrap, admission, &core_peer);
    }
}

fn serve_text(
    stream: UnixStream,
    credentials: shared_translator::unix::PeerCredentials,
    bootstrap: &wire::TranslatorBootstrap,
    admission: shared_translator::unix::Admission,
    core_peer: &shared_translator::unix::LivePeer,
) -> Result<(), Box<dyn std::error::Error>> {
    if !core_peer.is_live() { return Err("paired core ended".into()); }
    let mut client = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    let frame = shared_translator::unix::read_frame(&mut client, admission.frame_capacity)?;
    if frame.lane != wire::PUBLIC_TEXT_LANE || frame.kind != wire::REQUEST_KIND { return Err("wrong Flow text lane".into()); }
    let text = std::str::from_utf8(&frame.payload)?;
    let query = decode(text, &mut budget(&bootstrap.text_budget)?)?;
    let call = wire::Call { caller: Caller { process_id: credentials.process_id, user_id: credentials.user_id }, input: query };
    let response = binary_call(bootstrap, admission, call)?;
    shared_translator::unix::write_frame(&mut client, &shared_translator::unix::Frame {
        lane: wire::PUBLIC_TEXT_LANE, kind: wire::ANSWER_KIND, payload: encode(&response).into_bytes(),
    }, admission.frame_capacity)?;
    Ok(())
}

fn binary_call(
    bootstrap: &wire::TranslatorBootstrap,
    admission: shared_translator::unix::Admission,
    call: wire::Call,
) -> Result<Response, Box<dyn std::error::Error>> {
    let path = shared_translator::placement::runtime_path(&bootstrap.place).join(wire::CORE_SOCKET);
    let stream = UnixStream::connect(path)?;
    let mut bounded = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    let signal = call.signalize()?;
    shared_translator::unix::write_frame(&mut bounded, &shared_translator::unix::Frame {
        lane: wire::PUBLIC_SIGNAL_LANE, kind: wire::REQUEST_KIND, payload: signal.bytes().to_vec(),
    }, admission.frame_capacity)?;
    let frame = shared_translator::unix::read_frame(&mut bounded, admission.frame_capacity)?;
    if frame.lane != wire::PUBLIC_SIGNAL_LANE || frame.kind != wire::ANSWER_KIND { return Err("wrong Flow answer lane".into()); }
    Ok(Signal::<Response>::from(frame.payload).restore()?)
}

pub fn request(place: &shared_translator::Place, admission: shared_translator::unix::Admission, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let stream = UnixStream::connect(shared_translator::placement::runtime_path(place).join(wire::TRANSLATOR_SOCKET))?;
    let mut bounded = shared_translator::unix::DeadlineStream::new(stream, admission.deadline)?;
    shared_translator::unix::write_frame(&mut bounded, &shared_translator::unix::Frame {
        lane: wire::PUBLIC_TEXT_LANE, kind: wire::REQUEST_KIND, payload: text.as_bytes().to_vec(),
    }, admission.frame_capacity)?;
    let frame = shared_translator::unix::read_frame(&mut bounded, admission.frame_capacity)?;
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
