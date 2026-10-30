#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RequestId(pub i64);
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Phase {
    Reserved,
    Prepared,
    Released,
    Cleanup,
    RecoveryRequired,
    Complete,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Flow {
    pub flow_id: flow_nexus_types::FlowId,
    pub address: flow_nexus_types::Address,
    pub session_id_option: std::option::Option<flow_nexus_types::SessionId>,
    pub state: flow_nexus_types::State,
    pub flow_id_option: std::option::Option<flow_nexus_types::FlowId>,
    pub transcript_path_option: std::option::Option<flow_nexus_types::TranscriptPath>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct LaunchAttempt {
    pub request_id: RequestId,
    pub address: flow_nexus_types::Address,
    pub declaration: flow_nexus_types::Declaration,
    pub remembering: flow_nexus_types::Remembering,
    pub brief: flow_nexus_types::Brief,
    pub phase: Phase,
    pub flow_id_option: std::option::Option<flow_nexus_types::FlowId>,
    pub capsule_option: std::option::Option<flow_nexus_types::Capsule>,
    pub process_binding_option: std::option::Option<flow_nexus_types::ProcessBinding>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct EndAttempt {
    pub request_id: RequestId,
    pub flow_id: flow_nexus_types::FlowId,
    pub phase: Phase,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Queue {
    pub address: flow_nexus_types::Address,
    pub request_vector: std::vec::Vec<flow_nexus_types::Request>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Answer {
    Succeeded,
    Failed,
}
