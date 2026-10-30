#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ConfigurationRecord {
    pub configuration: crate::Configuration,
    pub meta_configure_occurred: crate::evidence::MetaConfigureOccurred,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct AddressReservation {
    pub address: crate::Address,
    pub flow_id: crate::FlowId,
    pub request_id: crate::evidence::RequestId,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ProcessBinding {
    pub flow_id: crate::FlowId,
    pub process_incarnation: crate::evidence::ProcessIncarnation,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct LaunchAttempt {
    pub request_id: crate::evidence::RequestId,
    pub flow_id: crate::FlowId,
    pub address: crate::Address,
    pub resolved_launch: crate::evidence::ResolvedLaunch,
    pub resource_evidence: crate::evidence::ResourceEvidence,
    pub creation_intent: crate::evidence::CreationIntent,
    pub launch_phase: crate::evidence::LaunchPhase,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct EndAttempt {
    pub request_id: crate::evidence::RequestId,
    pub flow_id: crate::FlowId,
    pub end_phase: crate::evidence::EndPhase,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Permaflow {
    pub address: crate::Address,
    pub flow_id_vector: std::vec::Vec<crate::FlowId>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Flow {
    pub flow_id: crate::FlowId,
    pub address: crate::Address,
    pub session_id_option: std::option::Option<crate::SessionId>,
    pub state: crate::State,
    pub flow_id_option: std::option::Option<crate::FlowId>,
    pub transcript_path_option: std::option::Option<crate::TranscriptPath>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Queue {
    pub address: crate::Address,
    pub admitted_request_vector: std::vec::Vec<crate::evidence::AdmittedRequest>,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Verification {
    pub verification_target: crate::evidence::VerificationTarget,
    pub content_fingerprint: crate::evidence::ContentFingerprint,
    pub host_boot_identity: crate::evidence::HostBootIdentity,
    pub pid_namespace_identity: crate::evidence::PidNamespaceIdentity,
    pub verification_state: crate::evidence::VerificationState,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct RequestAllocator {
    pub request_id: crate::evidence::RequestId,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct ArrivalAllocator {
    pub address: crate::Address,
    pub arrival_ordinal: crate::evidence::ArrivalOrdinal,
}
