# Initial anatomy checkpoint

Order: `primary-74ls`. Implementation brief: `flows/4c98c9/proposals/flow-v1-boundary-one.md` in Primary. Latest identity ruling: Psyche core Secondary delivery 20843.

This repository started empty. No Flow v0 source, contract, Capsule adapter or fixture was read or reused. The shared generator and foundational libraries are permitted dependencies.

## Present

- Ethos Library, Operation and Memory declarations, with generated Rust.
- Shared type library and runtime-anatomy library; neither is an executable.
- Nexus-minted FlowId independent of optional harness SessionId.
- Separate private launch/end attempt records and public lifecycle states.
- A Nix build entry point for the anatomy; generation used ethos-zero 19.0.0 at revision `c9b48dd291c62d8d071bf7abf9e3f44b7f19ff8c`, built through Nix.

## Still required

- Primary review of the draft schema and capability traits before runtime bodies.
- Complete configured harness/model/prompt and namespace anatomy.
- Standard meta/ordinary first-Configure behavior.
- Binary socket transport, process-incarnation authority, real Memory transactions, Capsule adapter, recovery, launch and end.
- Implied Memory/Operation/Signal root traits: the current generator output supplies data types but no such interactions.
- A typed meta Debug projection; the query exists and must return Unavailable until implemented.
- Real isolated harness proof and independent acceptance tests at immutable revisions.

Cross-metaflow runtime messaging follows standalone v1 proof. Send and Refresh are later boundaries. No runtime operation is presently implemented; no success receipt is emitted by this checkpoint. v0 remains unchanged. The three public remotes have been provisioned and their initial root commits pushed non-force. Nix compilation passed for the initial main anatomy with and without Datom, and both wire crates with Datom. This does not prove the revised configuration/evidence schemas, the newer Input/Output naming and implied traits, runtime behavior or the complete infrastructure closure.

## Initial build witnesses

- Main without Datom: `/nix/store/kjs9wl0mhgs82brc4rgz0g3mxr45hjwh-flow-nexus-anatomy-build-1.0.0`
- Main with Datom: `/nix/store/m1vv5s79yblas0riyq500rsa1yb86i6w-flow-nexus-anatomy-build-1.0.0`
- Ordinary wire with Datom: `/nix/store/0w08dz83jzyzy3w6sp60nh63d0612lxy-signal-flow-nexus-anatomy-build-1.0.0`
- Meta wire with Datom: `/nix/store/mc1wpfajxmrkpa619vsnyccawgf4nzjs-meta-signal-flow-nexus-anatomy-build-1.0.0`

These are compilation witnesses, not independent scenario tests. New Translator and implied-trait drafts are not implemented. The pinned generator emits the older Query/Response projection; adopting the new Input/Output/root-capability contract requires the upstream approved generator.

## Corrected anatomy checkpoint

The shared Library now contains complete draft configuration and a separate evidence module. Memory declares durable Flow/Permaflow/binding/attempt/queue rows, rather than treating every helper scalar as a stored row. Launch and end have separate evidence-bearing phases; creation intent and locator precede external effects. Ordinary Configure uses the same shared Configuration as meta. Debug is a typed private projection. There are still no runtime bodies.

Nix compiled the corrected main anatomy without and with Datom at `/nix/store/iznw4cj8mgp4lzci317hf91lqydpb97d-flow-nexus-anatomy-build-1.0.0` and `/nix/store/dz1dws13kc7b4y5mqppm6f83aig0sjnl-flow-nexus-anatomy-build-1.0.0`. Corrected ordinary/meta Datom builds are `/nix/store/zl1x25bb1wr8i6anq68k3paykrgg2rv2-signal-flow-nexus-anatomy-build-1.0.0` and `/nix/store/vjv3wq6mvh7czvk4rnrbx9mxklsg1ya8-meta-signal-flow-nexus-anatomy-build-1.0.0`.

The separate schema test repository preserves two failures against the earlier immutable anatomy: supplied bare-string newtype forms in Launch and End conflict with the current structural derive projection. Compilation and self-roundtrip tests do not settle that interface. Primary is deciding the expected shape/upstream action. Complete typed meta batch mutations and authenticated entry bootstrap remain under review. Translator remains a design-only draft.

## Internal Memory implementation slice

The runtime library now implements owner-batch candidate validation, a typed durable Memory table with atomic commits, and a component-owned Kameo Memory actor/port. Typed replies preserve committed/rejected/storage-failure outcomes; actor transport failure is separately MutationUncertain and never causes retry. The source exposes no socket or executable and launches nothing.

Completed rules include duplicate batch keys, referential/history/predecessor checks, address/reservation ownership, captured snapshot consistency, queue ordering, monotonic allocator validation, typed meta Verified-state refusal, and verification invalidation on target changes. Unresolved active rows are conservatively protected until the genuine reconciliation path exists. The verification fields carry fingerprints/boot/namespace; producing and validating live evidence is not yet implemented, and no stored state alone grants caller authority.

The reservation/admission allocator operations, authenticated Operation/Signal actor entry, OwnerLauncher kernel ancestry classification, default seeding, live reconciliation, and launch/end effects remain separate work. Translator stays unbuilt. Source generation still uses the older upstream roots without claiming Input/Output/implied-trait conformance.

Nix compiled this internal slice without/with Datom at `/nix/store/0zvs0qzc632aijbmqc2430r3ilfisrki-flow-nexus-anatomy-build-1.0.0` and `/nix/store/hcx1m1na2ispxj5jwczj5sxa3bgl791w-flow-nexus-anatomy-build-1.0.0`. This is compile evidence; independent Memory/actor scenarios are next.

## Identity integrity correction

Source audit and separate immutable-target tests found that the first Memory slice accepted a FlowId reserved at two addresses, private RequestId shared by launch/end attempts, and a completed launch with no Flow row. The corrected validator retains FlowId/address claims across historical and pending rows, refuses repeated launch identities, checks completed launch references, and rejects launch/end request reuse. Independent tests failed on cd4a184 before this correction; their corrected-target results are recorded in the separate test repository.

Primary21445/21446/21460/21473 decisions are now consumed: exact revision-bound verification, monotonic allocators, complete typed owner meta reads/writes, and OwnerLauncher policy. The policy is not an implemented credential/ancestry mechanism. Transparent Datom newtype form is settled by vision, not a pending design choice; the shared derive bug remains an upstream textual blocker. No Translator is built.
