#!/usr/bin/env bash
set -euo pipefail
revision=6cf5e88dc0bf02b52d789587df7af63e3284b5f2
package=${FLOW_PACKAGE:-/nix/store/fx97zi3zql2rm1s5xpllx1dbdfd3axdc-flow-nexus-v1}
flow="$package/bin/flow-nexus"; pair="$package/bin/flow-pair"
[[ -x "$flow" && -x "$pair" ]]
base=${PROOF_BASE:-$(mktemp -d /tmp/flow-definition-failure.XXXXXX)}
metaflow="definition-failure-$$"; export XDG_RUNTIME_DIR="$base/run"
runtime="$XDG_RUNTIME_DIR/metaflow/$metaflow"; state="$base/state"; evidence="$base/evidence"
mkdir -p "$runtime" "$state" "$evidence"
place="{ $metaflow «$runtime» «$state» }"; pair_pid=
cleanup(){ local s=$?; set +e; [[ -n "$pair_pid" ]] && { kill "$pair_pid" 2>/dev/null; wait "$pair_pid" 2>/dev/null; }; chmod -R u+rwx "$base" 2>/dev/null; printf 'definition failure evidence %s status %s\n' "$base" "$s" >&2; exit "$s"; }
trap cleanup EXIT INT TERM
"$pair" "$place" >"$evidence/pair.log" 2>&1 & pair_pid=$!
for _ in $(seq 1 400); do [[ -S "$runtime/flow-translator.sock" ]] && break; kill -0 "$pair_pid" 2>/dev/null || { cat "$evidence/pair.log" >&2; exit 1; }; sleep .05; done
[[ -S "$runtime/flow-translator.sock" ]]
run(){ "$flow" "$metaflow" "$@"; }
new='Define.{ failnew [ { worker Codex model-new Medium Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
new2='Define.{ failrename [ { worker Codex model-rename Medium Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
old='Define.{ existing [ { worker Codex model-old Low Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
replacement='Define.{ existing [ { worker Codex model-replaced High Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
run "$old" | tee "$evidence/define-old.out"
mem="$state/flow-memory"; existing_file="$mem/definitions/6578697374696e67"; sha256sum "$existing_file" | tee "$evidence/existing-before.sha256"
chmod 0555 "$mem/definition-staging"; run "$new" | tee "$evidence/staging-write-refused.out"; grep -q 'Refused.Memory' "$evidence/staging-write-refused.out"
chmod 0755 "$mem/definition-staging"; run "Describe.existing" | tee "$evidence/prior-after-write.out"; grep -q model-old "$evidence/prior-after-write.out"
sha256sum -c "$evidence/existing-before.sha256"
chmod 0555 "$mem/definition-staging"; run "$replacement" | tee "$evidence/replacement-staging-write-refused.out"; grep -q 'Refused.Memory' "$evidence/replacement-staging-write-refused.out"
chmod 0755 "$mem/definition-staging"; run "Describe.existing" | tee "$evidence/prior-after-replacement-write.out"; grep -q model-old "$evidence/prior-after-replacement-write.out"
sha256sum -c "$evidence/existing-before.sha256"
chmod 0555 "$mem/definitions"; run "$new2" | tee "$evidence/new-rename-refused.out"; grep -q 'Refused.Memory' "$evidence/new-rename-refused.out"
chmod 0755 "$mem/definitions"; run Definitions | tee "$evidence/after-new-rename.out"; ! grep -q failrename "$evidence/after-new-rename.out"
chmod 0555 "$mem/definitions" 2>/dev/null || true
run "$replacement" | tee "$evidence/rename-refused.out"; grep -q 'Refused.Memory' "$evidence/rename-refused.out"
chmod 0755 "$mem/definitions"; run "Describe.existing" | tee "$evidence/prior-after-rename.out"; grep -q model-old "$evidence/prior-after-rename.out"
sha256sum -c "$evidence/existing-before.sha256"
run "$new" | tee "$evidence/recovered-new.out"; grep -q 'Defined.failnew' "$evidence/recovered-new.out"
kill "$pair_pid"; wait "$pair_pid" || true; pair_pid=
rm -f -- "$runtime"/*.sock
"$pair" "$place" >"$evidence/restart.log" 2>&1 & pair_pid=$!; for _ in $(seq 1 400); do [[ -S "$runtime/flow-translator.sock" ]] && break; sleep .05; done
run Definitions | tee "$evidence/restart-definitions.out"; grep -q 'failnew' "$evidence/restart-definitions.out"; grep -q 'existing' "$evidence/restart-definitions.out"
printf 'DEFINITION_STORAGE_FAILURE_OK revision=%s package=%s\n' "$revision" "$package" | tee "$evidence/result"
