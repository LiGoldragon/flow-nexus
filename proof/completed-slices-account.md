# Completed v1 slices

These three bounded items are complete. The broad v1 feature remains open.

## Member effort argv

- Source: `1673138c66c6424871c4d2ec034d5f317e44451e`.
- Source-owner check: immutable `git+file` remote Nix check.
- Independent check: exact Low/Medium/High argv oracle review and a source-clean
  checkout Nix check.
- Detailed account: `proof/effort-argv-account.md`.
- Limit: neither check observes provider-side effort consumption.

## Controlled atomic Define

- Fixture: `3203390acf2cbc85f3ab4b84ee692d61a984f726`.
- Tested package: `/nix/store/rlgkpbw000fpdjks0l7byv3azif7prp6-flow-nexus-v1`,
  built from source `1673138c66c6424871c4d2ec034d5f317e44451e`.
- Replacing one definition preserved the other definition.
- `Refused.Invalid.UnknownSpeaker` preserved the prior definition.
- A controlled process restart read the committed definition set.
- Limit: crash and power-loss durability were not tested.

## Temporary Flow Register scaffold removal

- Flow source: `6cf5e88dc0bf02b52d789587df7af63e3284b5f2`.
- Signal contract `2.0.0`: `dfc1245adf3e8551e151e2499ea930d0913135aa`.
- Checked package: `/nix/store/fx97zi3zql2rm1s5xpllx1dbdfd3axdc-flow-nexus-v1`.
- Published branch tip: `6f382fb60b8e178d645bfbac483c430a8d7f4075`.
- Independent static and artifact inspection passed.
- The bridge-free live proof at `/tmp/first-launch-li-2891525.igpFVY`
  recorded owner-login Register, actual Flow peer `{ 1001 2891679 }`, assistant
  nonce `N20433A13109Q1488Z12897`, returned-path consumption, and owned cleanup.
- Limit: the live proof had no independent live rerun.

## Remaining v1 work

The outer shell, durable lifecycle, subscription consumer, and broader
acceptance remain. Lifecycle bead `flow-nexus-version-1-4xf` remains open.
The Belief filtered-subscription migration awaits a repaired immutable handoff.
