#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Start_Data {
    pub address: flow_nexus_types::Address,
    pub remembering: flow_nexus_types::Remembering,
    pub brief: flow_nexus_types::Brief,
    pub declaration: flow_nexus_types::Declaration,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Deliver_Data {
    pub flow_id: flow_nexus_types::FlowId,
    pub request: flow_nexus_types::Request,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Enqueue_Data {
    pub address: flow_nexus_types::Address,
    pub request: flow_nexus_types::Request,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Operation {
    Configure(flow_nexus_types::Configuration),
    Declare(flow_nexus_types::Declaration),
    Threshold(flow_nexus_types::Threshold),
    RecordHook(flow_nexus_types::RecordHook),
    Observe(flow_nexus_types::Address),
    Debug,
    Reconcile(flow_nexus_types::FlowId),
    Start(Start_Data),
    Stop(flow_nexus_types::FlowId),
    Deliver(Deliver_Data),
    Enqueue(Enqueue_Data),
    Replace(flow_nexus_types::FlowId),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Failed_Data {
    ConfigurationRefused,
    DeclarationRefused,
    ThresholdRefused,
    HookRefused,
    ObservationRefused,
    DebugRefused,
    ReconciliationRefused,
    CapsuleRefused,
    HarnessRefused,
    StoreRefused,
    RecoveryRequired,
    Unavailable,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Outcome {
    Configured,
    Declared,
    ThresholdSet,
    HookRecorded,
    Observed(std::vec::Vec<flow_nexus_types::Observation>),
    Debugged(flow_nexus_types::evidence::DebugProjection),
    Reconciled,
    Started(flow_nexus_types::FlowId),
    Stopped,
    Delivered,
    Enqueued,
    Replaced(flow_nexus_types::FlowId),
    Failed(Failed_Data),
}
