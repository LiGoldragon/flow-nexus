# Real three-Nexus first-launch proof

The current proof used fresh proof-owned Runtime, State, repository and
workspace paths. Its retained evidence is
`/tmp/first-launch-li-2856328.OwgS0N`.

- Flow source: `1673138c66c6424871c4d2ec034d5f317e44451e`
- Flow package: `/nix/store/rlgkpbw000fpdjks0l7byv3azif7prp6-flow-nexus-v1`
- Belief body: `f8b44e10cbf750bc87e45a50df5656028388680b`
- Belief package: `/nix/store/3v2wkqf3wj3miw1kq66v72fv5136wlwb-belief-nexus-v1`
- Herdr package: `/nix/store/51k5a966vki9d8ipah1p5kjj18mmbawv-herdr-nexus-v1`

Belief first answered `Resolved.{ [] [] }`. Its one configuration reply was
`Configured.{ RepositoryPolling.211 Nexus 24 Ready }`. The proof login then
registered the committed repository through the Belief CLI. No Flow Register
request was made. After the real Flow translator was live, its kernel identity
`{ 1001 2856518 }` was admitted to Belief and Herdr; both typed replies were
`Ready`. That peer performed the Furnish used by Launch.

The typed chain completed:

```text
Registered.{ /tmp/first-launch-li-2856328.OwgS0N/repository 37caac6e7346579599c859e094d0ac730b7b2a66 }
Defined.demo
Launched.1
ObservedPane.w1:p1
ObservedAssistant.N15097A11016Q20226Z19990
```

The definition selected `gpt-6.1-sol` at `Medium` effort from the immutable
Flow candidate. Source inspection and its immutable check establish the exact
invocation override; the live assistant receipt does not observe provider-side
effort consumption. The joined assistant nonce was absent from the Brief,
repository and workspace before Launch; only its two halves were submitted.

Flow accepted Belief's returned system-prompt and skills paths before opening
the pane. The proof additionally checked that Codex's generated configuration
named the stable system-prompt path and that its skill-discovery path resolved
to the stable skills path:

```text
ConsumedFurnishedPaths.{
  /tmp/first-launch-li-2856328.OwgS0N/workspace/.belief/current/instructions.md
  /tmp/first-launch-li-2856328.OwgS0N/workspace/.belief/current/skills
  /tmp/first-launch-li-2856328.OwgS0N/workspace/.agents/skills
}
```

Cleanup closed `w1:p1` and stopped the three owned pairs and Herdr server; all
recorded PIDs were absent afterward. The retained directory is evidence, not a
running deployment. This run proves integrated owner-login Register,
configured-Flow-peer Furnish, and live harness adoption for the pinned
candidate. It does not prove restart continuity or deployment into an existing
metaflow.

The older retained proof at `/tmp/real-three-nexus-proof-1793811173` remains
historical evidence for source `77530fd`; it used the temporary Flow Register
bridge. It is not evidence for this ownership split or the effort candidate.
