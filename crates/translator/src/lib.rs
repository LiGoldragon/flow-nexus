//! F1 canonical definition persistence and startup recovery.

use datom_codec::{Actualizing, Budget, Potential};
use flow_nexus_emission::{Query, Response};
use signal_flow_nexus_version_1::{
    Complex, ComplexName, DefinitionSet, Message, PersistenceFailure,
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct DefinitionStore {
    canonical: PathBuf,
    staging: PathBuf,
    next_stage: AtomicU64,
}

impl DefinitionStore {
    pub fn open(root: &Path) -> std::io::Result<Self> {
        let canonical = root.join("definitions");
        let staging = root.join("definition-staging");
        std::fs::create_dir_all(&canonical)?;
        std::fs::create_dir_all(&staging)?;
        Ok(Self { canonical, staging, next_stage: AtomicU64::new(1) })
    }

    /// Loads every canonical file. Any malformed entry, filename mismatch,
    /// duplicate, or incomplete complex refuses the whole recovered set.
    pub fn load(&self, budget: &mut Budget) -> Result<DefinitionSet, String> {
        let mut paths = std::fs::read_dir(&self.canonical)
            .map_err(|error| format!("read canonical definitions: {error}"))?
            .map(|entry| entry.map(|entry| entry.path()).map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut definitions = Vec::with_capacity(paths.len());
        for path in paths {
            if !path.is_file() { return Err(format!("canonical entry is not a file: {}", path.display())); }
            let filename = path.file_name().and_then(|name| name.to_str())
                .ok_or_else(|| format!("canonical filename is not UTF-8: {}", path.display()))?;
            let decoded_name = decode_name(filename)?;
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            let mut potential = Potential::<Complex>::from(text.as_str());
            let complex = potential.actualize(budget)
                .map_err(|error| format!("decode {}: {error:?}", path.display()))?;
            if complex.complex_name.0 != decoded_name {
                return Err(format!("filename/name mismatch at {}", path.display()));
            }
            definitions.push(complex);
        }
        let set = DefinitionSet(definitions);
        flow_nexus_core::validate_set(&set).map_err(|error| format!("invalid recovered definition: {error:?}"))?;
        Ok(set)
    }

    /// Executes exactly one persistence emission. Refusal is precommit: once
    /// rename or unlink commits, this returns the corresponding success.
    pub fn execute(&self, query: Query) -> Response {
        match query {
            Query::Persist(complex) => {
                let name = complex.complex_name.clone();
                match self.persist(&complex) {
                    Ok(()) => Response::Persisted(name),
                    Err(error) => Response::Refused(failure(name, error)),
                }
            }
            Query::Remove(name) => match self.remove(&name) {
                Ok(()) => Response::Removed(name),
                Err(error) => Response::Refused(failure(name, error)),
            },
        }
    }

    fn persist(&self, complex: &Complex) -> std::io::Result<()> {
        let filename = encode_name(&complex.complex_name.0);
        let stage_id = self.next_stage.fetch_add(1, Ordering::Relaxed);
        let stage = self.staging.join(format!("{}-{stage_id}", std::process::id()));
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&stage)?;
        let text = translator::encode(complex);
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        // Atomic rename is the commit boundary. No fallible work follows it.
        std::fs::rename(stage, self.canonical.join(filename))
    }

    fn remove(&self, name: &ComplexName) -> std::io::Result<()> {
        // Unlink is the commit boundary. Only the chosen canonical name is touched.
        std::fs::remove_file(self.canonical.join(encode_name(&name.0)))
    }
}

fn failure(complex_name: ComplexName, error: std::io::Error) -> PersistenceFailure {
    PersistenceFailure { complex_name, message: Message(error.to_string()) }
}

pub fn encode_name(name: &str) -> String {
    let mut encoded = String::with_capacity(name.len() * 2);
    for byte in name.as_bytes() { use std::fmt::Write as _; write!(&mut encoded, "{byte:02x}").expect("String write"); }
    encoded
}

pub fn decode_name(encoded: &str) -> Result<String, String> {
    if encoded.len() % 2 != 0 || encoded.bytes().any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase()) {
        return Err("definition filename is not canonical lowercase UTF-8 hex".into());
    }
    let bytes = encoded.as_bytes().chunks_exact(2).map(|pair| {
        let text = std::str::from_utf8(pair).expect("ASCII hex pair");
        u8::from_str_radix(text, 16).map_err(|_| "invalid hex pair".to_owned())
    }).collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| "definition filename does not encode UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_flow_nexus_version_1::{
        Beliefs, Continuity, Effort, FlowName, Harness, Member, Missing, ModelName, Speaker,
        Tools, Workspace,
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("flow-f1-store-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir(&path).unwrap(); Self(path)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    fn complex(name: &str) -> Complex {
        let flow = FlowName("member".into());
        Complex { complex_name: ComplexName(name.into()), member_vector: vec![Member {
            flow_name: flow.clone(), harness: Harness::Codex, model_name: ModelName("configured".into()),
            effort: Effort::Medium, continuity: Continuity::Ephemeral, workspace: Workspace::Temporary,
            beliefs: Beliefs { module_name_vector: vec![], missing: Missing::Refuse }, tools: Tools::HarnessDefault,
            route_vector: vec![],
        }], speaker: Speaker(flow) }
    }
    fn budget() -> Budget { datom_codec::Budget { remaining: 1_000_000, reader: protos::ReaderBudget { remaining: 1_000_000 }, depth: 0, maximum_depth: 64 } }

    #[test]
    fn canonical_name_round_trips_unicode_and_rejects_noncanonical() {
        let name = "méta flow";
        assert_eq!(decode_name(&encode_name(name)).unwrap(), name);
        assert!(decode_name("ABC0").is_err());
        assert!(decode_name("f").is_err());
    }

    #[test]
    fn persist_then_load_validates_filename_correspondence() {
        let fixture = Fixture::new();
        let store = DefinitionStore::open(&fixture.0).unwrap();
        assert!(matches!(store.execute(Query::Persist(complex("one"))), Response::Persisted(_)));
        assert_eq!(store.load(&mut budget()).unwrap(), DefinitionSet(vec![complex("one")]));
        std::fs::rename(store.canonical.join(encode_name("one")), store.canonical.join(encode_name("other"))).unwrap();
        assert!(store.load(&mut budget()).unwrap_err().contains("mismatch"));
    }

    #[test]
    fn precommit_stage_failure_preserves_old_canonical_value() {
        let fixture = Fixture::new();
        let store = DefinitionStore::open(&fixture.0).unwrap();
        assert!(matches!(store.execute(Query::Persist(complex("one"))), Response::Persisted(_)));
        std::fs::remove_dir(&store.staging).unwrap();
        std::fs::write(&store.staging, b"not a directory").unwrap();
        let mut replacement = complex("one"); replacement.member_vector[0].model_name = ModelName("replacement".into());
        assert!(matches!(store.execute(Query::Persist(replacement)), Response::Refused(_)));
        assert_eq!(store.load(&mut budget()).unwrap(), DefinitionSet(vec![complex("one")]));
    }

    #[test]
    fn removal_unlinks_only_the_selected_name() {
        let fixture = Fixture::new();
        let store = DefinitionStore::open(&fixture.0).unwrap();
        store.execute(Query::Persist(complex("one"))); store.execute(Query::Persist(complex("two")));
        assert!(matches!(store.execute(Query::Remove(ComplexName("one".into()))), Response::Removed(_)));
        assert_eq!(store.load(&mut budget()).unwrap(), DefinitionSet(vec![complex("two")]));
    }

    #[test]
    fn invalid_canonical_entry_is_never_ignored_but_staging_is_separate() {
        let fixture = Fixture::new();
        let store = DefinitionStore::open(&fixture.0).unwrap();
        std::fs::write(store.staging.join("abandoned"), b"partial").unwrap();
        assert_eq!(store.load(&mut budget()).unwrap(), DefinitionSet(vec![]));
        std::fs::write(store.canonical.join("zz"), b"invalid").unwrap();
        assert!(store.load(&mut budget()).is_err());
    }
}
