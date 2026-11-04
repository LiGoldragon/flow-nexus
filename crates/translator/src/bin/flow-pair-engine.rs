use flow_nexus_wire::{AdmissionValue, CoreBootstrap, TextBudget, TranslatorBootstrap};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::waitpid;
use nix::unistd::{Pid, Uid};
use shared_translator::{FrameCapacity, FrameWriting, PeerIdentity, ProcessId, Signalizable, UserId};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn integer(value: Option<String>, name: &str) -> Result<i64, Box<dyn std::error::Error>> { value.ok_or_else(|| name.to_owned())?.parse().map_err(Into::into) }
fn stop(child: &mut Child) { let pid = Pid::from_raw(child.id() as i32); let _ = kill(pid, Signal::SIGTERM); let _ = waitpid(pid, None); }

struct SocketWitness { path: PathBuf, device: u64, inode: u64 }
impl SocketWitness {
    fn capture(path: &Path) -> Option<Self> {
        let metadata = std::fs::symlink_metadata(path).ok()?;
        Some(Self { path: path.to_owned(), device: metadata.dev(), inode: metadata.ino() })
    }
    fn remove_owned(&self) {
        if std::fs::symlink_metadata(&self.path)
            .is_ok_and(|metadata| metadata.dev() == self.device && metadata.ino() == self.inode)
        { let _ = std::fs::remove_file(&self.path); }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let place_text = args.next().ok_or("PLACE")?;
    let core_executable = args.next().ok_or("CORE_EXECUTABLE")?;
    let translator_executable = args.next().ok_or("TRANSLATOR_EXECUTABLE")?;
    let admission = AdmissionValue { frame_deadline_milliseconds: integer(args.next(), "DEADLINE")?, frame_capacity_bytes: integer(args.next(), "CAPACITY")?, retry_delay_milliseconds: integer(args.next(), "RETRY")? };
    let validated_admission = flow_nexus_wire::validate_admission(&admission)?;
    let text_budget = TextBudget { node_limit: integer(args.next(), "NODES")?, depth_limit: integer(args.next(), "DEPTH")? };
    if text_budget.node_limit <= 0 || text_budget.depth_limit <= 0 { return Err("text budget must be positive".into()); }
    let trusted_path_vector = args.map(PathBuf::from).collect::<Vec<_>>();
    if trusted_path_vector.iter().any(|path| !path.is_absolute()) { return Err("trusted Codex paths must be absolute".into()); }
    let mut decode_budget = datom_codec::Budget { remaining: text_budget.node_limit, reader: protos::ReaderBudget { remaining: text_budget.node_limit as usize }, depth: 0, maximum_depth: text_budget.depth_limit };
    let place = shared_translator::decode::<shared_translator::Place>(&place_text, &mut decode_budget).map_err(|error| format!("{error:?}"))?;
    if !shared_translator::placement::runtime_path(&place).is_dir() || !shared_translator::placement::state_path(&place).is_dir() { return Err("Place directories must exist".into()); }
    let mut core = Command::new(core_executable).stdin(Stdio::piped()).spawn()?;
    let mut translator = match Command::new(translator_executable).stdin(Stdio::piped()).spawn() { Ok(value) => value, Err(error) => { stop(&mut core); return Err(error.into()); } };
    let user_id = UserId(i64::from(Uid::current().as_raw()));
    let capacity = FrameCapacity::default();
    let sent = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut input = core.stdin.take().ok_or("core stdin")?;
        input.write_frame(&CoreBootstrap {
            place: place.clone(), admission: admission.clone(),
            codex_deployment: flow_nexus_wire::CodexDeployment {
                trusted_path_vector: trusted_path_vector.iter().map(|path| path.to_string_lossy().into_owned()).collect(),
            },
            peer_identity: PeerIdentity { user_id: user_id.clone(), process_id: ProcessId(i64::from(translator.id())) },
        }.signalize()?, capacity)?;
        input.flush()?; drop(input);
        let mut input = translator.stdin.take().ok_or("translator stdin")?;
        input.write_frame(&TranslatorBootstrap { place: place.clone(), admission, text_budget, peer_identity: PeerIdentity { user_id, process_id: ProcessId(i64::from(core.id())) } }.signalize()?, capacity)?;
        input.flush()?; drop(input); Ok(())
    })();
    if let Err(error) = sent { stop(&mut core); stop(&mut translator); return Err(error); }
    let runtime = shared_translator::placement::runtime_path(&place);
    let core_socket = runtime.join(flow_nexus_wire::CORE_SOCKET);
    let translator_socket = runtime.join(flow_nexus_wire::TRANSLATOR_SOCKET);
    let readiness_deadline = Instant::now().checked_add(validated_admission.deadline).ok_or("readiness deadline")?;
    let (core_socket, translator_socket) = loop {
        if let (Some(core_socket), Some(translator_socket)) = (
            SocketWitness::capture(&core_socket), SocketWitness::capture(&translator_socket),
        ) { break (core_socket, translator_socket); }
        if Instant::now() >= readiness_deadline {
            stop(&mut core); stop(&mut translator);
            if let Some(value) = SocketWitness::capture(&core_socket) { value.remove_owned(); }
            if let Some(value) = SocketWitness::capture(&translator_socket) { value.remove_owned(); }
            return Err("Flow pair readiness timed out".into());
        }
        std::thread::sleep(validated_admission.retry_delay.min(Duration::from_millis(25)));
    };
    let core_pid = Pid::from_raw(core.id() as i32); let translator_pid = Pid::from_raw(translator.id() as i32);
    let ended = waitpid(Pid::from_raw(-1), None)?;
    let survivor = if ended.pid() == Some(core_pid) { translator_pid } else { core_pid };
    let _ = kill(survivor, Signal::SIGTERM); let _ = waitpid(survivor, None);
    core_socket.remove_owned(); translator_socket.remove_owned();
    Err(format!("paired Flow process ended: {ended:?}").into())
}
