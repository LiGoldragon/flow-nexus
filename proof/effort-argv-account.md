# Member.Effort Codex argv evidence

Source revision `1673138c66c6424871c4d2ec034d5f317e44451e` adds the
invocation-only `model_reasoning_effort` setting. Its production helper
tests cover exact Low/Medium/High argv, preserve the configured `--model`,
and append trust configuration only for trusted workspaces.

Independent review confirmed those vectors and semantics. The source owner's
immutable remote check was:
`nix build --max-jobs 0 --option fallback false git+file://$PWD?rev=1673138c66c6424871c4d2ec034d5f317e44451e#checks.x86_64-linux.test --no-link -L`.
My independent check used `nix build .#checks.x86_64-linux.test --no-link`
from the repository workspace; it passed with an unrelated Beads warning.
These are source-revision checks, not a separately exported archive or live
Codex deployment. The first direct-Cargo runner was invalid under the remote
build guard; `9e993cf` corrected it to the supported Nix check and passed
syntax validation only. The correction was pushed with `version-1` through
`origin` to `9e993cf`; no main or deployment branch was changed.
