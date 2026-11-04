#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Furnish_Data {
    pub flow_id: signal_flow_nexus_version_1::FlowId,
    pub furnish_request: signal_belief_nexus::FurnishRequest,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Open_Data_Create_Data {
    pub flow_id: signal_flow_nexus_version_1::FlowId,
    pub workspace: signal_herdr_nexus::Workspace,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Open_Data_Start_Data {
    pub flow_id: signal_flow_nexus_version_1::FlowId,
    pub pane: signal_herdr_nexus::Pane,
    pub command: signal_herdr_nexus::Command,
    pub label: signal_herdr_nexus::Label,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Open_Data_Prompt_Data {
    pub flow_id: signal_flow_nexus_version_1::FlowId,
    pub pane: signal_herdr_nexus::Pane,
    pub text: signal_herdr_nexus::Text,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Open_Data {
    Create(Open_Data_Create_Data),
    Start(Open_Data_Start_Data),
    Prompt(Open_Data_Prompt_Data),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Query {
    Register(signal_belief_nexus::Repository),
    Furnish(Furnish_Data),
    Open(Open_Data),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Registered_Data {
    Succeeded(signal_belief_nexus::RepositoryState),
    Refused(signal_belief_nexus::Refusal),
    Unavailable,
    Uncertain,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Furnished_Data_Succeeded_Data {
    pub flow_id: signal_flow_nexus_version_1::FlowId,
    pub furnished_workspace: signal_belief_nexus::FurnishedWorkspace,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Furnished_Data_Refused_Data {
    pub flow_id: signal_flow_nexus_version_1::FlowId,
    pub refusal: signal_belief_nexus::Refusal,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Furnished_Data {
    Succeeded(Furnished_Data_Succeeded_Data),
    Refused(Furnished_Data_Refused_Data),
    Unavailable(signal_flow_nexus_version_1::FlowId),
    Uncertain(signal_flow_nexus_version_1::FlowId),
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Opened_Data {
    pub flow_id: signal_flow_nexus_version_1::FlowId,
    pub outcome: signal_herdr_nexus::Outcome,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Response {
    Registered(Registered_Data),
    Furnished(Furnished_Data),
    Opened(Opened_Data),
}
