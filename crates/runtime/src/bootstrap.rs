//! Initial domain configuration and allocation state have one atomic birth.
use crate::store::{MemoryStore, Readable, StoreFailure, Writable};
use flow_nexus_types::{
    evidence::{InitializationOutcome, Mutation, MutationBatch, Record, RequestId},
    memory::{ConfigurationRecord, RequestAllocator},
};
pub trait Initializable {
    type Configuration;
    type Outcome;
    type Failure;
    fn initialize(
        &mut self,
        configuration: &Self::Configuration,
    ) -> Result<Self::Outcome, Self::Failure>;
}
impl Initializable for MemoryStore {
    type Configuration = ConfigurationRecord;
    type Outcome = InitializationOutcome;
    type Failure = StoreFailure;
    fn initialize(
        &mut self,
        configuration: &ConfigurationRecord,
    ) -> Result<InitializationOutcome, StoreFailure> {
        if !self.snapshot()?.records.is_empty() {
            return Ok(InitializationOutcome::ExistingState);
        }
        match self.mutate(&MutationBatch(vec![
            Mutation::Put(Record::ConfigurationRecord(configuration.clone())),
            Mutation::Put(Record::RequestAllocator(RequestAllocator {
                request_id: RequestId(0),
            })),
        ])) {
            Ok(()) => Ok(InitializationOutcome::Initialized),
            Err(StoreFailure::Integrity(error)) => Ok(InitializationOutcome::Refused(error)),
            Err(error) => Err(error),
        }
    }
}
