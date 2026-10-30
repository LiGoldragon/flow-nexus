//! Six-character candidates and configured identity-space accounting.
use crate::state::RecordState;
use flow_nexus_types::{FlowId, IdentityPolicy, evidence::Record};
use std::{collections::BTreeSet, future::Future};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntropyFailure {
    Interrupted,
    Unavailable,
    OperatingSystem(i32),
    InvalidReturn,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MintFailure {
    InvalidAlphabet,
    Cancelled,
    Entropy(EntropyFailure),
    EmptyRead,
    InvalidReadLength,
}
pub trait EntropyReadable {
    fn read_entropy(&mut self, target: &mut [u8]) -> Result<usize, EntropyFailure>;
}
pub trait CancellationObservable {
    fn is_cancelled(&self) -> bool;
}
pub trait Alphabetic {
    fn alphabet(&self) -> Result<&[u8], MintFailure>;
}
impl Alphabetic for IdentityPolicy {
    fn alphabet(&self) -> Result<&[u8], MintFailure> {
        let bytes = self.identity_alphabet.0.as_bytes();
        if bytes.len() < 2
            || bytes.iter().enumerate().any(|(index, value)| {
                !value.is_ascii_alphanumeric() || bytes[..index].contains(value)
            })
        {
            return Err(MintFailure::InvalidAlphabet);
        }
        Ok(bytes)
    }
}
pub struct MintContext<'a, E, C> {
    pub policy: &'a IdentityPolicy,
    pub entropy: &'a mut E,
    pub cancellation: &'a C,
}
pub trait CandidateMintable {
    type Identity;
    type Failure;
    fn mint(&mut self) -> impl Future<Output = Result<Self::Identity, Self::Failure>> + Send;
}
impl<E: EntropyReadable + Send, C: CancellationObservable + Sync> CandidateMintable
    for MintContext<'_, E, C>
{
    type Identity = FlowId;
    type Failure = MintFailure;
    async fn mint(&mut self) -> Result<FlowId, MintFailure> {
        let alphabet = self.policy.alphabet()?;
        let limit = 256 - (256 % alphabet.len());
        let mut identity = String::with_capacity(6);
        while identity.len() < 6 {
            if self.cancellation.is_cancelled() {
                return Err(MintFailure::Cancelled);
            }
            let mut byte = [0];
            match self.entropy.read_entropy(&mut byte) {
                Err(EntropyFailure::Interrupted) => {
                    tokio::task::yield_now().await;
                    continue;
                }
                Err(error) => return Err(MintFailure::Entropy(error)),
                Ok(0) => return Err(MintFailure::EmptyRead),
                Ok(1) => {}
                Ok(_) => return Err(MintFailure::InvalidReadLength),
            }
            if usize::from(byte[0]) < limit {
                identity.push(char::from(alphabet[usize::from(byte[0]) % alphabet.len()]));
            } else {
                tokio::task::yield_now().await;
            }
        }
        if self.cancellation.is_cancelled() {
            return Err(MintFailure::Cancelled);
        }
        Ok(FlowId(identity))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityCapacity(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccupiedIdentities(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentitySpace {
    pub capacity: IdentityCapacity,
    pub occupied: OccupiedIdentities,
}
pub trait Exhaustible {
    fn exhausted(&self) -> bool;
}
impl Exhaustible for IdentitySpace {
    fn exhausted(&self) -> bool {
        self.occupied.0 >= self.capacity.0
    }
}
pub trait IdentityCountable {
    type Policy;
    type Space;
    type Failure;
    fn identity_space(&self, policy: &Self::Policy) -> Result<Self::Space, Self::Failure>;
}
impl IdentityCountable for RecordState {
    type Policy = IdentityPolicy;
    type Space = IdentitySpace;
    type Failure = MintFailure;
    fn identity_space(&self, policy: &IdentityPolicy) -> Result<IdentitySpace, MintFailure> {
        let alphabet = policy.alphabet()?;
        // Validation bounds the alphabet to the62 distinct ASCII alphanumeric symbols.
        let capacity = (alphabet.len() as u64)
            .checked_pow(6)
            .ok_or(MintFailure::InvalidAlphabet)?;
        let mut identities = BTreeSet::new();
        for record in &self.records {
            match record {
                Record::Flow(row) => {
                    identities.insert(&row.flow_id.0);
                }
                Record::LaunchAttempt(row) => {
                    identities.insert(&row.flow_id.0);
                }
                Record::AddressReservation(row) => {
                    identities.insert(&row.flow_id.0);
                }
                Record::Permaflow(row) => {
                    identities.extend(row.flow_id_vector.iter().map(|id| &id.0));
                }
                _ => {}
            }
        }
        let occupied = identities
            .into_iter()
            .filter(|id| {
                id.len() == 6 && id.as_bytes().iter().all(|value| alphabet.contains(value))
            })
            .count() as u64;
        Ok(IdentitySpace {
            capacity: IdentityCapacity(capacity),
            occupied: OccupiedIdentities(occupied),
        })
    }
}

// The platform crate owns the sole unsafe syscall boundary; runtime stays safe.
impl EntropyReadable for flow_nexus_platform::LinuxEntropy {
    fn read_entropy(&mut self, target: &mut [u8]) -> Result<usize, EntropyFailure> {
        use flow_nexus_platform::{EntropyError, OsEntropyReadable};
        OsEntropyReadable::read_entropy(self, target)
            .map(|count| count.0)
            .map_err(|error| match error {
                EntropyError::Interrupted => EntropyFailure::Interrupted,
                EntropyError::Unavailable => EntropyFailure::Unavailable,
                EntropyError::OperatingSystem(errno) => EntropyFailure::OperatingSystem(errno.0),
                EntropyError::InvalidReturn => EntropyFailure::InvalidReturn,
            })
    }
}
