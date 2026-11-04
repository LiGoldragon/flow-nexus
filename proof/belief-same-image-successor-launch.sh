#!/usr/bin/env bash
# Belief same-image publication successor launch proof.
# Uses immutable Flow, the accepted Belief successor, accepted Herdr Nexus, and Herdr CLI packages.
# Flow 776a4dcaa0896a8a7d300bb2cd3bc65241feec47; signal 31c7d96fbbfb6288061558328fea84cf7695a028.
# Belief 748925ff894e40d7ed049bb27eb95ddb105ff5f4; ordinary signal 579cf898b4ebd633457c936858b1166f16e88716.
set -euo pipefail

belief_package=/nix/store/ixd0xyskn22wqn55wvq93vhspzrjkpw2-belief-nexus-v1
herdr_package=/nix/store/51k5a966vki9d8ipah1p5kjj18mmbawv-herdr-nexus-v1
herdr=/nix/store/9x03bz0q0a978zrzcshrlmmb4fvcmdb6-herdr-0.8.2/bin/herdr
flow_package=/nix/store/gfhqrcy1bhga3fj6a8ks7b1mpsi3lxzm-flow-nexus-v1
flow_pair=/nix/store/jmh63gx9n3j468pq6qajb6gimk8ah1p8-flow-nexus-engines-1.0.0/libexec/flow-pair-engine
flow_daemon=/nix/store/zg2aa2wxgqbrigbizl8dq5vwbhjdlkkk-flow-nexus-daemon-1.0.0/bin/flow-nexus-daemon
flow_translator=/nix/store/wflqlapzcwynbwm5k0df8lsdw8jzvpdn-flow-nexus-translator-1.0.0/bin/flow-nexus-translator
flow="$flow_package/bin/flow-nexus"

for executable in \
  "$flow_pair" "$flow_daemon" "$flow_translator" "$flow" \
  "$belief_package/bin/belief-pair" "$belief_package/bin/belief-nexus" "$belief_package/bin/belief-nexus-meta" \
  "$herdr_package/bin/herdr-pair" "$herdr_package/bin/herdr-nexus-meta" \
  "$herdr"; do
  [[ -x "$executable" ]] || { printf 'required executable is absent: %s\n' "$executable" >&2; exit 64; }
done

tag="belief-same-image-${USER}-$$"
base=${PROOF_BASE:-}
if [[ -n "$base" ]]; then
  mkdir "$base"
else
  base=$(mktemp -d "/tmp/${tag}.XXXXXX")
fi
metaflow="launch$$"
export XDG_RUNTIME_DIR="$base/run"
runtime="$XDG_RUNTIME_DIR/metaflow/$metaflow"
state="$base/state/$metaflow"
workspace="$base/workspace"
repository="$base/repository"
evidence="$base/evidence"
mkdir -p "$runtime" "$state" "$workspace" "$repository/spirit" "$evidence"
exec > >(tee "$evidence/run-transcript.txt") 2>&1
sha256sum "$0" | tee "$evidence/executed-script.sha256"
cat >"$evidence/immutable-pins.txt" <<'PINS'
FlowSource 776a4dcaa0896a8a7d300bb2cd3bc65241feec47
FlowSignal 31c7d96fbbfb6288061558328fea84cf7695a028
FlowPackage /nix/store/gfhqrcy1bhga3fj6a8ks7b1mpsi3lxzm-flow-nexus-v1
BeliefBody 748925ff894e40d7ed049bb27eb95ddb105ff5f4
BeliefSignal 579cf898b4ebd633457c936858b1166f16e88716
BeliefPackage /nix/store/ixd0xyskn22wqn55wvq93vhspzrjkpw2-belief-nexus-v1
HerdrPackage /nix/store/51k5a966vki9d8ipah1p5kjj18mmbawv-herdr-nexus-v1
HerdrCli /nix/store/9x03bz0q0a978zrzcshrlmmb4fvcmdb6-herdr-0.8.2/bin/herdr
PINS
place="{ $metaflow «$runtime» «$state» }"
uid=$(id -u)
pair_pids=()
herdr_server_pid=
flow_translator_pid=
pane=

cleanup() {
  local status=$?
  set +e
  printf 'cleanup owns base=%s metaflow=%s pairs=%s server=%s pane=%s\n' \
    "$base" "$metaflow" "${pair_pids[*]:-}" "${herdr_server_pid:-}" "${pane:-}" >&2
  if [[ -n "$pane" ]]; then
    "$herdr" --session "$metaflow" pane close "$pane" >"$evidence/cleanup-pane.out" 2>&1 || true
  fi
  "$herdr" --session "$metaflow" workspace close w1 >"$evidence/cleanup-workspace.out" 2>&1 || true
  "$herdr" --session "$metaflow" server stop >"$evidence/cleanup-server.out" 2>&1 || true
  for pid in "${pair_pids[@]:-}"; do
    [[ -n "$pid" ]] && kill "$pid" >/dev/null 2>&1 || true
  done
  [[ -n "$herdr_server_pid" ]] && kill "$herdr_server_pid" >/dev/null 2>&1 || true
  wait >/dev/null 2>&1 || true
  : >"$evidence/cleanup-processes.out"
  for pid in "${pair_pids[@]:-}" "${herdr_server_pid:-}" "${flow_translator_pid:-}"; do
    [[ -n "$pid" ]] || continue
    if kill -0 "$pid" 2>/dev/null; then
      printf 'Live.%s\n' "$pid" >>"$evidence/cleanup-processes.out"
    else
      printf 'Gone.%s\n' "$pid" >>"$evidence/cleanup-processes.out"
    fi
  done
  find "$runtime" -type s -print 2>/dev/null | sort >"$evidence/residual-sockets.out"
  printf '%s\n' "$status" >"$evidence/exit-code.txt"
  if [[ -n ${KEEP_PROOF:-} || $status -ne 0 ]]; then
    printf 'retaining owned evidence: %s\n' "$base" >&2
  else
    printf 'deleting owned proof directory: %s\n' "$base" >&2
    rm -rf "$base"
  fi
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

capture() {
  local name=$1
  shift
  printf '$' >"$evidence/$name.command"
  printf ' %q' "$@" >>"$evidence/$name.command"
  printf '\n' >>"$evidence/$name.command"
  "$@" | tee "$evidence/$name.out"
}

wait_for_socket() {
  local pid=$1 path=$2 log=$3
  for _ in $(seq 1 400); do
    kill -0 "$pid" 2>/dev/null || { cat "$log" >&2; return 1; }
    [[ -S "$path" ]] && return 0
    sleep .05
  done
  printf 'socket did not appear: %s\n' "$path" >&2
  cat "$log" >&2
  return 1
}

child_matching() {
  local parent=$1 pattern=$2 result=
  for _ in $(seq 1 200); do
    result=$(ps -o pid=,args= --ppid "$parent" | awk -v pattern="$pattern" 'index($0, pattern) { print $1 }')
    if [[ $(wc -w <<<"$result") -eq 1 ]]; then printf '%s\n' "$result"; return 0; fi
    sleep .05
  done
  printf 'one child matching %s was not found beneath %s\n' "$pattern" "$parent" >&2
  ps -o pid=,uid=,args= --ppid "$parent" >&2 || true
  return 1
}

# One committed belief module is enough to witness a real furnished bundle.
printf '%s\n' 'FIRST_LAUNCH_BELIEF_MARKER' >"$repository/spirit/core.md"
git -C "$repository" init -b main >/dev/null
git -C "$repository" config user.email proof@flow.invalid
git -C "$repository" config user.name proof
git -C "$repository" add spirit/core.md
git -C "$repository" commit -m 'launch proof belief' >/dev/null
git -C "$workspace" init -b main >/dev/null

"$herdr_package/bin/herdr-pair" "$place" >"$evidence/herdr-pair.log" 2>&1 &
herdr_pair_pid=$!; pair_pids+=("$herdr_pair_pid")
wait_for_socket "$herdr_pair_pid" "$runtime/herdr-meta-translator.sock" "$evidence/herdr-pair.log"

# The Herdr pair begins from typed bootstrap and becomes ready through its owned settings.
herdr_meta="$herdr_package/bin/herdr-nexus-meta"
capture herdr-path "$herdr_meta" "$metaflow" "Configure.HerdrPath.$herdr" >/dev/null
capture herdr-call-timeout "$herdr_meta" "$metaflow" 'Configure.CallTimeout.70000' >/dev/null
capture herdr-output-limit "$herdr_meta" "$metaflow" 'Configure.OutputLimit.8388608' >/dev/null
capture herdr-text-budget "$herdr_meta" "$metaflow" 'Configure.TextBudget.{ 1048576 64 }' >/dev/null
capture herdr-start-timeout "$herdr_meta" "$metaflow" 'Configure.StartTimeout.70000' >/dev/null
capture herdr-polling "$herdr_meta" "$metaflow" 'Configure.Polling.100' >/dev/null
capture herdr-frame-capacity "$herdr_meta" "$metaflow" 'Configure.FrameCapacity.8388608' >/dev/null
capture herdr-subscriber-capacity "$herdr_meta" "$metaflow" 'Configure.SubscriberCapacity.64' >/dev/null
capture herdr-transport-timeout "$herdr_meta" "$metaflow" 'Configure.TransportTimeout.70000' >"$evidence/herdr-ready.txt"
grep -q Ready "$evidence/herdr-ready.txt"

"$belief_package/bin/belief-pair" "$place" >"$evidence/belief-pair.log" 2>&1 &
belief_pair_pid=$!; pair_pids+=("$belief_pair_pid")
wait_for_socket "$belief_pair_pid" "$runtime/belief-meta-translator.sock" "$evidence/belief-pair.log"
belief_ready=
for _ in $(seq 1 200); do
  belief_ready=$(capture belief-resolve "$belief_package/bin/belief-nexus" "$metaflow" 'Resolve.[]' 2>/dev/null || true)
  [[ "$belief_ready" == Resolved.* ]] && break
  sleep .05
done
[[ "$belief_ready" == 'Resolved.{ [] [] }' ]]

# Readiness is a typed configuration result, not the appearance of a socket.
belief_meta="$belief_package/bin/belief-nexus-meta"
belief_polling=$(capture belief-polling "$belief_meta" "$metaflow" \
  'Configure.{ None RepositoryPolling.211 }')
[[ "$belief_polling" == Configured.*Ready* ]]

# The login owns repository registration. The configured Flow peer will only Furnish.
registered=$(capture belief-register "$belief_package/bin/belief-nexus" "$metaflow" "Register.«$repository»")
[[ "$registered" == Registered.* ]]

# The package's immutable pair engine accepts proof-local Codex trust as its final setting.
"$flow_pair" "$place" "$flow_daemon" "$flow_translator" 70000 8388608 25 1048576 64 "$workspace" >"$evidence/flow-pair.log" 2>&1 &
flow_pair_pid=$!; pair_pids+=("$flow_pair_pid")
wait_for_socket "$flow_pair_pid" "$runtime/flow-translator.sock" "$evidence/flow-pair.log"
flow_translator_pid=$(child_matching "$flow_pair_pid" '/flow-nexus-translator')
flow_translator_uid=$(ps -o uid= -p "$flow_translator_pid" | tr -d ' ')
[[ "$flow_translator_uid" == "$uid" ]]
printf 'FlowTranslator.{ %s %s }\n' "$flow_translator_pid" "$flow_translator_uid" | tee "$evidence/flow-translator-identity.out"

# Herdr captures and verifies this live PID/UID when it accepts the stable peer binding.
herdr_peer=$(capture herdr-peer "$herdr_meta" "$metaflow" \
  "Configure.Peer.{ flow { $flow_translator_uid $flow_translator_pid } }")
[[ "$herdr_peer" == Configured.* ]]

# Atomically replace the one OrdinaryPeers field. The final Belief body authorizes this live peer.
belief_peers=$(capture belief-peers "$belief_meta" "$metaflow" \
  "Configure.{ None OrdinaryPeers.[ { flow { $flow_translator_uid $flow_translator_pid } } ] }")
[[ "$belief_peers" == Configured.*Ready* ]]

# The real Herdr backend keeps the normal login credential environment. Only its owned session/runtime is isolated.
"$herdr" --session "$metaflow" server >"$evidence/herdr-server.log" 2>&1 &
herdr_server_pid=$!
for _ in $(seq 1 200); do
  "$herdr" --session "$metaflow" status server >/dev/null 2>&1 && break
  kill -0 "$herdr_server_pid" 2>/dev/null || { cat "$evidence/herdr-server.log" >&2; exit 1; }
  sleep .05
done
"$herdr" --session "$metaflow" status server >"$evidence/herdr-server-status.out"

definition="Define.{ demo [ { worker Codex gpt-6.1-sol Medium Ephemeral Path.«$workspace» { [ { Spirit core } ] Refuse } HarnessDefault [] } ] worker }"
defined=$(capture flow-define "$flow" "$metaflow" "$definition")
[[ "$defined" == 'Defined.demo' ]]

# The joined target is computed, never written into the Brief, bundle, or launch argv.
half_one="N${RANDOM}A${RANDOM}"
half_two="Q${RANDOM}Z${RANDOM}"
target="${half_one}${half_two}"
brief="Return only the concatenation of these two halves, with no separator: $half_one and $half_two"
[[ "$brief" != *"$target"* ]]
if grep -R -F "$target" "$repository" "$workspace" >/dev/null 2>&1; then
  printf 'nonce target already exists in the belief bundle or workspace\n' >&2
  exit 1
fi
launch="Launch.{ demo worker «$brief» }"
launched=$(capture flow-launch "$flow" "$metaflow" "$launch")
[[ "$launched" == 'Launched.1' ]]
printf '%s\n' "$launched" | tee "$evidence/launched.out"

# Flow deliberately returns only FlowId. Observe the created pane independently and retain its exact identity.
for _ in $(seq 1 200); do
  "$herdr" --session "$metaflow" pane list >"$evidence/panes.out" 2>&1 || true
  pane=$(python3 - "$evidence/panes.out" "$workspace" <<'PY'
import json, sys
try:
    body = json.load(open(sys.argv[1]))
    panes = body.get("result", {}).get("panes", [])
    matches = [p["pane_id"] for p in panes if p.get("cwd") == sys.argv[2] and p.get("agent") == "codex"]
    print(matches[0] if len(matches) == 1 else "")
except (OSError, ValueError, KeyError, TypeError):
    print("")
PY
  )
  [[ -n "$pane" ]] && break
  pane=
  sleep .1
done
[[ -n "$pane" ]] || { cat "$evidence/panes.out" >&2; exit 1; }
printf 'ObservedPane.%s\n' "$pane" | tee "$evidence/observed-pane.out"

for _ in $(seq 1 600); do
  "$herdr" --session "$metaflow" pane read --source recent-unwrapped --lines 200 "$pane" >"$evidence/pane-output.txt" 2>&1 || true
  grep -F -q "$target" "$evidence/pane-output.txt" && break
  sleep .2
done
grep -F -q "$target" "$evidence/pane-output.txt"
printf 'ObservedAssistant.%s\n' "$target" | tee "$evidence/observed-assistant.out"

# Furnishing was accepted from Belief's returned paths. Verify the stable paths
# that Flow checked and the Codex configuration/discovery paths consumed by Start.
system_prompt="$workspace/.belief/current/instructions.md"
skills="$workspace/.belief/current/skills"
skill_discovery="$workspace/.agents/skills"
test -f "$system_prompt"
test -d "$skills"
grep -F -x "model_instructions_file = \"$system_prompt\"" "$workspace/.codex/config.toml" >/dev/null
[[ "$(realpath "$skill_discovery")" == "$(realpath "$skills")" ]]
printf 'ConsumedFurnishedPaths.{ %s %s %s }\n' \
  "$system_prompt" "$skills" "$skill_discovery" | tee "$evidence/furnished-paths.out"

printf 'BELIEF_SAME_IMAGE_SUCCESSOR_LAUNCH_OK base=%s flow=%s pane=%s\n' "$base" "$launched" "$pane"
