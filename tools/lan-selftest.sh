#!/usr/bin/env bash
# Two headless Godot instances play a LAN match over localhost (prompt 025):
# one hosts, the other finds it with the LAN search, joins and gets ready;
# both order their tanks around until tick 400 is confirmed by both. Since
# prompt 037 it is a 2-against-2 on twin_meadow: the two humans against two
# computer players that the host runs (LAN_COMPUTERS=0: the old 1 against 1).
# Since prompt 190 a second pair plays a cooperative mission (LAN_MISSION,
# default two_bridges; empty: skipped). Exit code 0 only if every instance
# passes. Logs: build/lan-selftest/.
#
# Ports can be changed with LAN_PORT and LAN_DISCOVERY_PORT. Needs bash
# (Git Bash on Windows).
set -uo pipefail

GODOT="${GODOT:-godot}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${LAN_PORT:-47820}"
DISCOVERY="${LAN_DISCOVERY_PORT:-47821}"
LOGS="$ROOT/build/lan-selftest"
mkdir -p "$LOGS"

run() {
    "$GODOT" --path "$ROOT/godot" --headless --fixed-fps 60 res://scenes/lan/lan_lobby.tscn -- \
        --lan-self-test --lan-port="$PORT" --lan-discovery-port="$DISCOVERY" "$@"
}

COMPUTERS="${LAN_COMPUTERS:-2}"
TEAM_ARGS=()
if [[ "$COMPUTERS" -gt 0 ]]; then
    TEAM_ARGS=(--map=twin_meadow --lan-computers="$COMPUTERS")
fi
run --lan-host --lan-name=host ${TEAM_ARGS[@]+"${TEAM_ARGS[@]}"} > "$LOGS/host.log" 2>&1 &
HOST=$!
sleep 1
run --lan-join --lan-name=client > "$LOGS/client.log" 2>&1 &
CLIENT=$!

# Watchdog: a hanging instance (e.g. a script error) must not block forever.
( for _ in $(seq "${LAN_TIMEOUT:-180}"); do sleep 1; done; kill "$HOST" "$CLIENT" 2>/dev/null ) &
WATCHDOG=$!

wait "$HOST"; HOST_CODE=$?
wait "$CLIENT"; CLIENT_CODE=$?
{ kill "$WATCHDOG" && wait "$WATCHDOG"; } 2>/dev/null

grep -h "LAN self-test\|ERROR" "$LOGS/host.log" "$LOGS/client.log"
echo "host exit code $HOST_CODE, client exit code $CLIENT_CODE (logs in $LOGS)"

# Prompt 190: a cooperative mission, the client in the second seat.
MISSION_CODE=0
if [[ -n "${LAN_MISSION:-two_bridges}" ]]; then
    run --lan-host --lan-name=host --lan-mission="${LAN_MISSION:-two_bridges}" > "$LOGS/mission-host.log" 2>&1 &
    HOST=$!
    sleep 1
    run --lan-join --lan-name=client > "$LOGS/mission-client.log" 2>&1 &
    CLIENT=$!
    ( for _ in $(seq "${LAN_TIMEOUT:-180}"); do sleep 1; done; kill "$HOST" "$CLIENT" 2>/dev/null ) &
    WATCHDOG=$!
    wait "$HOST"; MH=$?
    wait "$CLIENT"; MC=$?
    { kill "$WATCHDOG" && wait "$WATCHDOG"; } 2>/dev/null
    grep -h "LAN self-test\|ERROR" "$LOGS/mission-host.log" "$LOGS/mission-client.log"
    echo "mission: host exit code $MH, client exit code $MC"
    if [[ $MH -ne 0 || $MC -ne 0 ]]; then MISSION_CODE=1; fi
fi
if [[ $HOST_CODE -eq 0 && $CLIENT_CODE -eq 0 && $MISSION_CODE -eq 0 ]]; then
    echo "LAN self-test PASSED"
    exit 0
fi
echo "LAN self-test FAILED"
exit 1
