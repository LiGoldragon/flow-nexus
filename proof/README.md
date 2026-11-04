# Real three-Nexus first-launch proof

This proof was run with immutable packages. Its retained evidence is
`/tmp/real-three-nexus-proof-1793811173`.

- Flow source: `77530fd90204bcc3204c1275b6e765bf0dddfc95`
- Belief: `/nix/store/3v2wkqf3wj3miw1kq66v72fv5136wlwb-belief-nexus-v1`
- Herdr: `/nix/store/51k5a966vki9d8ipah1p5kjj18mmbawv-herdr-nexus-v1`

The typed deployment explicitly trusted only the proof-owned furnished
workspace. The temporary Flow `Register` bridge registered one committed
Spirit module; the same configured Flow peer then asked Belief to furnish the
workspace.

The submission chain completed:

```text
Registered.{ /tmp/real-three-nexus-proof-1793811173/repository 27014790ec1a94df3e8dcc285a8b5195d84b37f2 }
Defined.demo
Launched.1
ObservedPane.w1:p1
```

The separate assistant receipt witness was:

```text
ObservedAssistant.N28501A17348Q27967Z5895
```

That joined nonce was absent from the Brief, repository, workspace, and Launch
arguments before submission. Its two halves appeared separately in the Brief.

Belief furnished these paths:

```text
/tmp/real-three-nexus-proof-1793811173/workspace/.belief/current/instructions.md
/tmp/real-three-nexus-proof-1793811173/workspace/.agents/skills
```

Cleanup closed the exact owned pane `w1:p1` and stopped the three owned pairs.
The evidence directory remains for inspection. The Register bridge is a
temporary first-launch scaffold until Login-owned Register and peer-owned
Furnish replace it.

Earlier attempts found three distinct causes before this proof: stale package
autodetection selected incompatible Herdr and Belief packages; then a legacy
pane-list parser missed a successful submission. The reproduced script pins
every store path and parses the current pane output.
