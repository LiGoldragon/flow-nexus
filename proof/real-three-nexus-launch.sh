#!/usr/bin/env bash
# DRAFT, NOT YET RUN: awaiting a compatible Belief body package.
# Intended real first-launch proof: Flow -> Belief Furnish -> Herdr Create/Start/Prompt -> Codex response.
set -euo pipefail

flow_package=${1:?usage: real-three-nexus-launch.sh FLOW_PACKAGE BELIEF_PACKAGE HERDR_PACKAGE [HERDR]}
belief_package=${2:?usage: real-three-nexus-launch.sh FLOW_PACKAGE BELIEF_PACKAGE HERDR_PACKAGE [HERDR]}
herdr_package=${3:?usage: real-three-nexus-launch.sh FLOW_PACKAGE BELIEF_PACKAGE HERDR_PACKAGE [HERDR]}
herdr=${4:-/home/li/.nix-profile/bin/herdr}

for executable in \
  "$flow_package/bin/flow-pair" "$flow_package/bin/flow-nexus" \
  "$belief_package/bin/belief-pair" "$belief_package/bin/belief-nexus" "$belief_package/bin/belief-nexus-meta" \
  "$herdr_package/bin/herdr-pair" "$herdr_package/bin/herdr-nexus-meta" \
  "$herdr"; do
  [[ -x "$executable" ]] || { printf 'required executable is absent: %s\n' "$executable" >&2; exit 64; }
done

tag="first-launch-${USER}-$$"
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
mkdir -p "$runtime" "$state" "$workspace" "$repository/knowledge" "$evidence"
place="{ $metaflow «$runtime» «$state» }"
uid=$(id -u)
pair_pids=()
herdr_server_pid=
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
printf '%s\n' \
  '---' \
  'name: launch-proof' \
  'description: Marks the real three-Nexus launch fixture.' \
  '---' \
  '' \
  'FIRST_LAUNCH_BELIEF_MARKER' >"$repository/knowledge/launchProof.md"
git -C "$repository" init -b main >/dev/null
git -C "$repository" config user.email proof@flow.invalid
git -C "$repository" config user.name proof
git -C "$repository" add knowledge/launchProof.md
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
registered=$(capture belief-register "$belief_package/bin/belief-nexus" "$metaflow" "Register.«$repository»")
[[ "$registered" == Registered.* ]]

"$flow_package/bin/flow-pair" "$place" >"$evidence/flow-pair.log" 2>&1 &
flow_pair_pid=$!; pair_pids+=("$flow_pair_pid")
wait_for_socket "$flow_pair_pid" "$runtime/flow-translator.sock" "$evidence/flow-pair.log"
flow_translator_pid=$(child_matching "$flow_pair_pid" '/flow-nexus-translator')
flow_translator_uid=$(ps -o uid= -p "$flow_translator_pid" | tr -d ' ')
[[ "$flow_translator_uid" == "$uid" ]]
printf 'FlowTranslator.{ %s %s }\n' "$flow_translator_pid" "$flow_translator_uid" | tee "$evidence/flow-translator-identity.out"

# Herdr captures and verifies this live PID/UID when it accepts the stable peer binding.
herdr_peer=$(capture herdr-peer "$herdr_meta" "$metaflow" \
  "Configure.Peer.{ flow-translator { $flow_translator_uid $flow_translator_pid } }")
[[ "$herdr_peer" == Configured.* ]]

# Obtain a coherent Belief revision, then atomically replace OrdinaryPeers by CAS.
belief_meta="$belief_package/bin/belief-nexus-meta"
revision_witness=$(capture belief-revision "$belief_meta" "$metaflow" 'Configure.{ None RepositoryPolling.100 }')
[[ "$revision_witness" == Configured.* ]]
revision=$(sed -E 's/^Configured\.\{.* ([0-9]+) (Ready|Awaiting\..*) \}$/\1/' <<<"$revision_witness")
[[ "$revision" =~ ^[0-9]+$ ]] || { printf 'could not decode Belief revision: %s\n' "$revision_witness" >&2; exit 1; }
belief_peers=$(capture belief-peers "$belief_meta" "$metaflow" \
  "Configure.{ Some.$revision OrdinaryPeers.[ { flow-translator { $flow_translator_uid $flow_translator_pid } } ] }")
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

flow="$flow_package/bin/flow-nexus"
definition="Define.{ demo [ { worker Codex gpt-6.1-sol Medium Ephemeral Path.«$workspace» { [ { Knowledge launchProof } ] Refuse } HarnessDefault [] } ] worker }"
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
  pane=$(grep -oE 'w[0-9]+:p[0-9]+' "$evidence/panes.out" | head -1 || true)
  [[ -n "$pane" ]] && grep -q 'demo:worker' "$evidence/panes.out" && break
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

# Furnishing was accepted from Belief's returned paths; retain the concrete stable artifacts used by Start.
test -f "$workspace/.belief/current/instructions.md"
test -d "$workspace/.agents/skills"
printf 'FurnishedPaths.{ %s %s }\n' \
  "$workspace/.belief/current/instructions.md" "$workspace/.agents/skills" | tee "$evidence/furnished-paths.out"

printf 'REAL_THREE_NEXUS_LAUNCH_OK base=%s flow=%s pane=%s\n' "$base" "$launched" "$pane"
