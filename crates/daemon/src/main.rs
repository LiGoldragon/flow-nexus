use shared_translator::{FrameCapacity, FrameReading, Restorable, Signal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = std::io::stdin();
    let frame = stdin.read_frame(FrameCapacity::default())?;
    let bootstrap = Signal::<flow_nexus_wire::CoreBootstrap>::from(Vec::<u8>::from(frame)).restore()?;
    drop(stdin);
    nix::unistd::close(0)?;
    flow_nexus_daemon::run(&bootstrap)
}
