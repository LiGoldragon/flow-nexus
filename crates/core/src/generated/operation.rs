#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Operation {
    Defining(signal_flow_nexus_version_1::Complex),
    Undefining(signal_flow_nexus_version_1::ComplexName),
    Describing,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Outcome {
    Defined(signal_flow_nexus_version_1::ComplexName),
    Undefined(signal_flow_nexus_version_1::ComplexName),
    Described(signal_flow_nexus_version_1::Snapshot),
    Refused(signal_flow_nexus_version_1::MutationFailure),
}
