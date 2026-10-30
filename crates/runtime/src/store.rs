//! Typed durable rows, committed as one transaction by the single owning Memory actor.
use crate::state::{Keyed, Mutatable, RecordState, Validatable};
use flow_nexus_types::evidence::{Invariant, MutationBatch, ReadSelector, Record, RecordKey};
use std::{fmt, path::PathBuf};

#[derive(Clone)]
pub struct TableName(pub &'static str);
#[derive(Clone)]
pub struct StoreLayout {
    pub path: PathBuf,
    pub table_name: TableName,
    pub schema: memory::Schema,
}
pub struct MemoryStore {
    layout: StoreLayout,
    kernel: memory::Memory,
    table: memory::Table<&'static [u8], Record>,
}
#[derive(Debug)]
pub enum StoreFailure {
    Closed,
    Kernel(memory::Error),
    Archive(rkyv::rancor::Error),
    CorruptKey,
    Integrity(Invariant),
    FileSystem(std::io::Error),
}
impl fmt::Display for StoreFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, formatter)
    }
}
impl std::error::Error for StoreFailure {}
impl From<memory::Error> for StoreFailure {
    fn from(value: memory::Error) -> Self {
        Self::Kernel(value)
    }
}
pub trait Openable: Sized {
    type Layout;
    type Failure;
    fn open(layout: Self::Layout) -> Result<Self, Self::Failure>;
}
pub trait Readable {
    type State;
    type Selector;
    type Record;
    type Failure;
    fn snapshot(&self) -> Result<Self::State, Self::Failure>;
    fn read(&self, selector: &Self::Selector) -> Result<Vec<Self::Record>, Self::Failure>;
}
pub trait Writable {
    type Batch;
    type Failure;
    fn mutate(&mut self, batch: &Self::Batch) -> Result<(), Self::Failure>;
}
trait ArchivableKey {
    fn archive_key(&self) -> Result<Vec<u8>, StoreFailure>;
}
impl ArchivableKey for RecordKey {
    fn archive_key(&self) -> Result<Vec<u8>, StoreFailure> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(StoreFailure::Archive)
    }
}
impl Openable for MemoryStore {
    type Layout = StoreLayout;
    type Failure = StoreFailure;
    fn open(layout: StoreLayout) -> Result<Self, StoreFailure> {
        let kernel = memory::Memory::open_with_schema(&layout.path, &layout.schema)?;
        let table = memory::Table::new(layout.table_name.0);
        kernel.write(|transaction| table.ensure(transaction))?;
        let mut layout = layout;
        layout.path = std::fs::canonicalize(&layout.path).map_err(StoreFailure::FileSystem)?;
        let store = Self {
            layout,
            kernel,
            table,
        };
        let state = store.snapshot()?;
        state.validate(&state).map_err(StoreFailure::Integrity)?;
        Ok(store)
    }
}
impl Readable for MemoryStore {
    type State = RecordState;
    type Selector = ReadSelector;
    type Record = Record;
    type Failure = StoreFailure;
    fn snapshot(&self) -> Result<RecordState, StoreFailure> {
        let rows = self
            .kernel
            .read(|transaction| self.table.iter(transaction))?;
        let mut records = Vec::with_capacity(rows.len());
        // The dependency returns tuple entries at this contact point; domain rows remain named types.
        for (key, record) in rows {
            if key != record.key().archive_key()? {
                return Err(StoreFailure::CorruptKey);
            }
            records.push(record);
        }
        Ok(RecordState { records })
    }
    fn read(&self, selector: &ReadSelector) -> Result<Vec<Record>, StoreFailure> {
        let snapshot = self.snapshot()?;
        Ok(match selector {
            ReadSelector::All => snapshot.records,
            ReadSelector::ByKey(key) => snapshot
                .records
                .into_iter()
                .filter(|row| row.key() == *key)
                .collect(),
        })
    }
}
impl Writable for MemoryStore {
    type Batch = MutationBatch;
    type Failure = StoreFailure;
    fn mutate(&mut self, batch: &MutationBatch) -> Result<(), StoreFailure> {
        let old = self.snapshot()?;
        let candidate = old.mutate(batch).map_err(StoreFailure::Integrity)?;
        let mut removals = Vec::with_capacity(old.records.len());
        for record in &old.records {
            removals.push(record.key().archive_key()?);
        }
        let mut insertions = Vec::with_capacity(candidate.records.len());
        for record in candidate.records {
            insertions.push(StoredRow {
                key: record.key().archive_key()?,
                record,
            });
        }
        // No await or competing writer exists between snapshot validation and this atomic commit.
        self.kernel.write(|transaction| {
            for key in &removals {
                self.table.remove(transaction, key.as_slice())?;
            }
            for row in &insertions {
                self.table
                    .insert(transaction, row.key.as_slice(), &row.record)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
struct StoredRow {
    key: Vec<u8>,
    record: Record,
}

pub trait LayoutBearing {
    type Layout;
    fn layout(&self) -> Self::Layout;
}
impl LayoutBearing for MemoryStore {
    type Layout = StoreLayout;
    fn layout(&self) -> StoreLayout {
        self.layout.clone()
    }
}
