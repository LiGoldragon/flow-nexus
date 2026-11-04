//! Typed bootstrap and ordinary call wire shared by the Flow pair.

use shared_translator::{Caller, PeerIdentity, Place};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

pub const PUBLIC_TEXT_LANE: u8 = 1;
pub const PUBLIC_SIGNAL_LANE: u8 = 3;
pub const REQUEST_KIND: u8 = 0;
pub const ANSWER_KIND: u8 = 1;
pub const FAILED_KIND: u8 = 2;
pub const CORE_SOCKET: &str = "flow.sock";
pub const TRANSLATOR_SOCKET: &str = "flow-translator.sock";

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AdmissionValue {
    pub frame_deadline_milliseconds: i64,
    pub frame_capacity_bytes: i64,
    pub retry_delay_milliseconds: i64,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TextBudget {
    pub node_limit: i64,
    pub depth_limit: i64,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CoreBootstrap {
    pub place: Place,
    pub admission: AdmissionValue,
    pub codex_deployment: CodexDeployment,
    pub peer_identity: PeerIdentity,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CodexDeployment {
    pub trusted_path_vector: Vec<String>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TranslatorBootstrap {
    pub place: Place,
    pub admission: AdmissionValue,
    pub text_budget: TextBudget,
    pub peer_identity: PeerIdentity,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub caller: Caller,
    pub input: signal_flow_nexus_version_1::Query,
}

pub fn validate_admission(value: &AdmissionValue) -> Result<shared_translator::unix::Admission, &'static str> {
    let deadline = u64::try_from(value.frame_deadline_milliseconds).ok().filter(|value| *value > 0)
        .map(std::time::Duration::from_millis).ok_or("frame deadline")?;
    let frame_capacity = usize::try_from(value.frame_capacity_bytes).map_err(|_| "frame capacity")?;
    let retry = u64::try_from(value.retry_delay_milliseconds).ok().filter(|value| *value > 0)
        .map(std::time::Duration::from_millis).ok_or("retry delay")?;
    shared_translator::unix::Admission::new(deadline, frame_capacity, retry)
}

/// A listener whose pathname is removed only while it still names this listener.
pub struct BoundUnixListener {
    listener: UnixListener,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl BoundUnixListener {
    pub fn bind(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let listener = UnixListener::bind(&path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        Ok(Self { listener, path, device: metadata.dev(), inode: metadata.ino() })
    }

    pub fn listener(&self) -> &UnixListener { &self.listener }
}

impl Drop for BoundUnixListener {
    fn drop(&mut self) {
        if std::fs::symlink_metadata(&self.path)
            .is_ok_and(|metadata| metadata.dev() == self.device && metadata.ino() == self.inode)
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bound_listener_removes_only_its_own_path() {
        let directory = std::env::temp_dir().join(format!(
            "flow-nexus-wire-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("flow.sock");
        let listener = BoundUnixListener::bind(&path).unwrap();
        assert!(path.exists());
        drop(listener);
        assert!(!path.exists());

        let listener = BoundUnixListener::bind(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"replacement").unwrap();
        drop(listener);
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&directory).unwrap();
    }
}
