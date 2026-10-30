//! Flow v1 Operation and Memory anatomy. Runtime effects are not implemented yet.
#![forbid(unsafe_code)]
#[path = "../../../ethos/generated/operation.rs"]
pub mod operation;
pub use flow_nexus_types::memory;

pub mod memory_actor;
pub mod state;
pub mod store;

pub mod coherence;

pub mod reservation;
