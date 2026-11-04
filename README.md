# Flow Nexus version 1

Fresh version-1 scaffold. This repository starts from an orphan `version-1`
branch and contains no v0 implementation or dependency wiring.

## F1 boundary

The first contract is a user-defined `Complex` definition set. Definitions
are replaced atomically. Startup reads one canonical UTF-8-hex definition
filename, validates correspondence, duplicate names, and complete complexes
before the service becomes ready. A malformed definition refuses visibly.

A mutation that may have reached persistence but lost its outcome is
`MutationFailure::Uncertain`; its complex name remains reserved until the
restart recovery fence resolves it. Recovery is part of the F1 design; this
scaffold does not implement runtime I/O.

`Operation` and `Emission` are temporary Signal-root stand-ins for the missing
Emission root support in ethos-zero. They are marked in their own schema files;
ethos-zero is unchanged.

The `Tools` declaration records configuration. This scaffold does not enforce
tool policy and has no Launch or Stop operation. It does not copy the Herdr
server framework. Shared transport will use immutable translator revision
`2676fd0cb0ced3316dc151f73a9f59856e15e62d` when implementation begins.
