#!/usr/bin/env bash
# Exact-revision CLI acceptance for one-definition replacement and restart restore.
set -euo pipefail

revision=1673138c66c6424871c4d2ec034d5f317e44451e
source="git+file://$(git rev-parse --show-toplevel)?rev=$revision"
package=${FLOW_PACKAGE:-}
if [[ -z "$package" ]]; then
  package=$(nix build --max-jobs 0 --option fallback false \
    "$source#default" --no-link --print-out-paths -L)
fi
flow="$package/bin/flow-nexus"
pair="$package/bin/flow-pair"
[[ -x "$flow" && -x "$pair" ]] || {
  printf 'Flow package lacks required executables: %s\n' "$package" >&2
  exit 64
}

base=${PROOF_BASE:-$(mktemp -d /tmp/flow-atomic-define.XXXXXX)}
metaflow="atomic-define-$$"
export XDG_RUNTIME_DIR="$base/run"
runtime="$XDG_RUNTIME_DIR/metaflow/$metaflow"
state="$base/state"
evidence="$base/evidence"
mkdir -p "$runtime" "$state" "$evidence"
place="{ $metaflow «$runtime» «$state» }"
pair_pid=

cleanup() {
  local status=$?
  set +e
  if [[ -n "$pair_pid" ]]; then
    kill "$pair_pid" >/dev/null 2>&1 || true
    wait "$pair_pid" >/dev/null 2>&1 || true
  fi
  printf 'atomic Define evidence retained at %s (status %s)\n' "$base" "$status" >&2
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

wait_for_socket() {
  local pid=$1
  for _ in $(seq 1 400); do
    kill -0 "$pid" 2>/dev/null || { cat "$evidence/pair.log" >&2; return 1; }
    [[ -S "$runtime/flow-translator.sock" ]] && return 0
    sleep .05
  done
  printf 'Flow translator socket did not appear\n' >&2
  cat "$evidence/pair.log" >&2
  return 1
}

start_pair() {
  : >"$evidence/pair.log"
  "$pair" "$place" >"$evidence/pair.log" 2>&1 &
  pair_pid=$!
  wait_for_socket "$pair_pid"
}

stop_pair() {
  kill "$pair_pid"
  wait "$pair_pid" || true
  pair_pid=
  mapfile -t owned_sockets < <(find "$runtime" -maxdepth 1 -type s -print | sort)
  printf '%s\n' "${owned_sockets[@]}" >"$evidence/restart-runtime-deletions"
  for socket in "${owned_sockets[@]}"; do
    [[ "$socket" == "$runtime/"* && -S "$socket" ]] || {
      printf 'restart cleanup path is outside the owned runtime: %s\n' "$socket" >&2
      return 1
    }
  done
  ((${#owned_sockets[@]} == 0)) || rm -f -- "${owned_sockets[@]}"
}

expect() {
  local name=$1 expected=$2
  shift 2
  printf '$' >"$evidence/$name.command"
  printf ' %q' "$@" >>"$evidence/$name.command"
  printf '\n' >>"$evidence/$name.command"
  local actual
  actual=$("$@")
  printf '%s\n' "$actual" | tee "$evidence/$name.out"
  [[ "$(tr -d '[:space:]' <<<"$actual")" == "$(tr -d '[:space:]' <<<"$expected")" ]] || {
    printf 'expected: %s\nactual:   %s\n' "$expected" "$actual" >&2
    return 1
  }
}

alpha_one='Define.{ alpha [ { worker Codex model-one Low Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
alpha_two='Define.{ alpha [ { worker Codex model-two High Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
alpha_invalid='Define.{ alpha [ { worker Codex model-invalid Medium Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] absent }'
beta='Define.{ beta [ { worker Codex model-beta Medium Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
alpha_two_value='{ alpha [ { worker Codex model-two High Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'
beta_value='{ beta [ { worker Codex model-beta Medium Ephemeral Temporary { [] Refuse } HarnessDefault [] } ] worker }'

printf '%s\n' "$revision" >"$evidence/revision"
printf '%s\n' "$source" >"$evidence/source"
printf '%s\n' "$package" >"$evidence/package"
printf '%s\n' "$state" >"$evidence/state"
printf '%s\n' 'Controlled process restart only; crash-boundary durability is untested.' >"$evidence/limitations"

start_pair
expect define-alpha-one Defined.alpha "$flow" "$metaflow" "$alpha_one"
expect define-beta Defined.beta "$flow" "$metaflow" "$beta"
expect list-before-replacement 'Listed.[ alpha beta ]' "$flow" "$metaflow" Definitions

expect replace-alpha Defined.alpha "$flow" "$metaflow" "$alpha_two"
expect describe-replaced-alpha "Described.$alpha_two_value" "$flow" "$metaflow" Describe.alpha
expect describe-preserved-beta "Described.$beta_value" "$flow" "$metaflow" Describe.beta

expect refuse-invalid-alpha Refused.Invalid.UnknownSpeaker.absent "$flow" "$metaflow" "$alpha_invalid"
expect describe-alpha-after-refusal "Described.$alpha_two_value" "$flow" "$metaflow" Describe.alpha
expect list-after-refusal 'Listed.[ alpha beta ]' "$flow" "$metaflow" Definitions

# This is a controlled process restart, not a crash-boundary durability fixture.
stop_pair
start_pair
expect list-after-restart 'Listed.[ alpha beta ]' "$flow" "$metaflow" Definitions
expect describe-alpha-after-restart "Described.$alpha_two_value" "$flow" "$metaflow" Describe.alpha
expect describe-beta-after-restart "Described.$beta_value" "$flow" "$metaflow" Describe.beta

printf 'ATOMIC_DEFINE_ACCEPTANCE_OK revision=%s package=%s state=%s\n' \
  "$revision" "$package" "$state" | tee "$evidence/result"
