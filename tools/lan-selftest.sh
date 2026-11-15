#!/usr/bin/env bash
# Two headless Godot instances play a LAN match over localhost (prompt 025):
# one hosts, the other finds it with the LAN search, joins and gets ready;
# both order their tanks around until tick 400 is confirmed by both.
# Exit code 0 only if both instances pass. Logs: build/lan-selftest/.
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

run --lan-host --lan-name=host > "$LOGS/host.log" 2>&1 &
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
if [[ $HOST_CODE -eq 0 && $CLIENT_CODE -eq 0 ]]; then
    echo "LAN self-test PASSED"
    exit 0
fi
echo "LAN self-test FAILED"
exit 1
