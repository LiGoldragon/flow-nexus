//! Shared Flow v1 vocabulary. Wire crates depend on this library, never each other.
#![forbid(unsafe_code)]
#[path = "../../../ethos/generated/library.rs"]
mod generated;
pub use generated::*;
#[path = "../../../ethos/generated/evidence.rs"]
pub mod evidence;
#[path = "../../../ethos/generated/memory.rs"]
pub mod memory;
