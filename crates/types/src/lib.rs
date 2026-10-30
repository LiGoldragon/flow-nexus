//! Shared Flow v1 vocabulary. Wire crates depend on this library, never each other.
#![forbid(unsafe_code)]
#[path = "../../../ethos/generated/library.rs"]
mod generated;
pub use generated::*;
