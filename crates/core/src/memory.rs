//! Binary typed definition Memory owned by the Flow Nexus core.

use crate::{generated::memory::Answer, validate_complex};
use signal::{ByteViewable, Restorable, Signal, Signalizable};
use signal_flow_nexus_version_1::{
    Complex, ComplexName, DefinitionRejection, DefinitionSet, MemoryRefusal, Message,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct DefinitionMemory {
    state: PathBuf,
    canonical: PathBuf,
    staging: PathBuf,
    sequence: AtomicU64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadFailure {
    Unavailable,
    Corrupt,
    Invalid(DefinitionRejection),
}
impl std::fmt::Display for LoadFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(formatter, "Flow Memory load failed: {self:?}") }
}
impl std::error::Error for LoadFailure {}

impl DefinitionMemory {
    /// Opens the definition Memory beneath the already-resolved `Place.State`.
    pub fn open(state: &Path) -> Result<Self, LoadFailure> {
        let root = state.join("flow-memory");
        let canonical = root.join("definitions");
        let staging = root.join("definition-staging");
        std::fs::create_dir_all(&canonical).map_err(|_| LoadFailure::Unavailable)?;
        std::fs::create_dir_all(&staging).map_err(|_| LoadFailure::Unavailable)?;
        Ok(Self { state: state.to_owned(), canonical, staging, sequence: AtomicU64::new(1) })
    }

    pub fn state(&self) -> &Path { &self.state }

    /// Restores the complete definition set. One malformed canonical file
    /// refuses the whole load; staging artifacts are outside this namespace.
    pub fn load(&self) -> Result<DefinitionSet, LoadFailure> {
        let mut paths = std::fs::read_dir(&self.canonical)
            .map_err(|_| LoadFailure::Unavailable)?
            .map(|entry| entry.map(|entry| entry.path()).map_err(|_| LoadFailure::Unavailable))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut definitions = BTreeMap::new();
        for path in paths {
            if !path.is_file() { return Err(LoadFailure::Corrupt); }
            let encoded = path.file_name().and_then(|name| name.to_str()).ok_or(LoadFailure::Corrupt)?;
            let expected_name = decode_name(encoded).ok_or(LoadFailure::Corrupt)?;
            let bytes = std::fs::read(&path).map_err(|_| LoadFailure::Unavailable)?;
            let complex = Signal::<Complex>::from(bytes).restore().map_err(|_| LoadFailure::Corrupt)?;
            if complex.complex_name.0 != expected_name { return Err(LoadFailure::Corrupt); }
            validate_complex(&complex).map_err(LoadFailure::Invalid)?;
            if definitions.insert(expected_name, complex.clone()).is_some() {
                return Err(LoadFailure::Invalid(DefinitionRejection::DuplicateComplex(complex.complex_name)));
            }
        }
        Ok(DefinitionSet(definitions.into_values().collect()))
    }

    /// Atomically replaces exactly one named definition. Every failure is
    /// before the rename commit boundary, so the returned Answer is truthful.
    pub fn store(&self, complex: &Complex) -> Answer {
        if let Err(rejection) = validate_complex(complex) {
            return failed(&complex.complex_name, format!("invalid definition: {rejection:?}"));
        }
        match self.store_validated(complex) {
            Ok(()) => Answer::Succeeded,
            Err(message) => failed(&complex.complex_name, message),
        }
    }

    fn store_validated(&self, complex: &Complex) -> Result<(), String> {
        let archived = complex.signalize().map_err(|error| format!("archive definition: {error:?}"))?;
        let number = self.sequence.fetch_add(1, Ordering::Relaxed);
        let stage = self.staging.join(format!("{}-{number}", std::process::id()));
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&stage)
            .map_err(|error| format!("create staging file: {error}"))?;
        file.write_all(archived.bytes()).and_then(|()| file.sync_all())
            .map_err(|error| format!("write staging file: {error}"))?;
        drop(file);
        // Rename is the commit boundary. No fallible operation follows it.
        std::fs::rename(stage, self.canonical.join(encode_name(&complex.complex_name.0)))
            .map_err(|error| format!("commit definition: {error}"))
    }
}

fn failed(name: &ComplexName, message: String) -> Answer {
    Answer::Failed(MemoryRefusal { complex_name: name.clone(), message: Message(message) })
}

fn encode_name(name: &str) -> String {
    let mut encoded = String::with_capacity(name.len() * 2);
    for byte in name.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing into String is infallible");
    }
    encoded
}

fn decode_name(encoded: &str) -> Option<String> {
    if encoded.is_empty() || encoded.len() % 2 != 0 || encoded.bytes().any(|byte| {
        !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase()
    }) { return None; }
    let bytes = encoded.as_bytes().chunks_exact(2).map(|pair| {
        std::str::from_utf8(pair).ok().and_then(|text| u8::from_str_radix(text, 16).ok())
    }).collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_flow_nexus_version_1::{
        Beliefs, Continuity, Effort, FlowName, Harness, Member, Missing, ModelName, Speaker,
        Tools, Workspace,
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(1);
            let path = std::env::temp_dir().join(format!(
                "flow-binary-memory-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    fn complex(name: &str, model: &str) -> Complex {
        let flow_name = FlowName(format!("{name}-member"));
        Complex {
            complex_name: ComplexName(name.into()),
            member_vector: vec![Member {
                flow_name: flow_name.clone(), harness: Harness::Codex,
                model_name: ModelName(model.into()), effort: Effort::Medium,
                continuity: Continuity::Ephemeral, workspace: Workspace::Temporary,
                beliefs: Beliefs { module_name_vector: vec![], missing: Missing::Refuse },
                tools: Tools::HarnessDefault, route_vector: vec![],
            }],
            speaker: Speaker(flow_name),
        }
    }

    #[test]
    fn define_then_restart_restores_the_typed_definition() {
        let fixture = Fixture::new();
        let memory = DefinitionMemory::open(&fixture.0).unwrap();
        assert_eq!(memory.store(&complex("one", "configured")), Answer::Succeeded);
        drop(memory);
        let restarted = DefinitionMemory::open(&fixture.0).unwrap();
        assert_eq!(restarted.load().unwrap(), DefinitionSet(vec![complex("one", "configured")]));
    }

    #[test]
    fn invalid_define_preserves_the_previous_definition() {
        let fixture = Fixture::new();
        let memory = DefinitionMemory::open(&fixture.0).unwrap();
        let original = complex("one", "original");
        assert_eq!(memory.store(&original), Answer::Succeeded);
        let mut invalid = complex("one", "replacement");
        invalid.speaker = Speaker(FlowName("absent".into()));
        assert!(matches!(memory.store(&invalid), Answer::Failed(_)));
        assert_eq!(memory.load().unwrap(), DefinitionSet(vec![original]));
    }

    #[test]
    fn malformed_canonical_file_refuses_the_complete_load() {
        let fixture = Fixture::new();
        let memory = DefinitionMemory::open(&fixture.0).unwrap();
        std::fs::write(memory.canonical.join(encode_name("one")), b"not a signal").unwrap();
        assert_eq!(memory.load(), Err(LoadFailure::Corrupt));
    }
}
