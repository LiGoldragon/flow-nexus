# Real three-Nexus first-launch proof — draft

**NOT YET RUN.** The prepared script is blocked on a compatible Belief body
artifact. No launch success or runtime evidence is claimed here. The intended
Flow input is commit `820cb0be365c5f170591a0f1ded7cec3ab43ce10`, package
`/nix/store/sr454yspprplsizs5iylnlrhrff2q364-flow-nexus-v1`.

`real-three-nexus-launch.sh` starts the installed Flow, Belief, and Herdr pairs
at one typed `Place`. It registers one committed belief module, binds the live
Flow translator PID and UID as an ordinary peer in both downstream Nexuses,
defines one Codex member, and issues one public `Launch`.

A successful run records `Launched.1`, the exact Herdr pane labelled by Flow,
the furnished paths, and a fresh concatenated nonce read from the real Codex
pane. The concatenated nonce is absent from the Brief and belief bundle. The
script retains evidence on failure and removes only its owned pane, Herdr
session, pair processes, and temporary directory after success.

```sh
KEEP_PROOF=1 ./proof/real-three-nexus-launch.sh \
  /nix/store/...-flow-nexus-v1 \
  /nix/store/...-belief-nexus-v1 \
  /nix/store/...-herdr-nexus-v1
```
