extends Node
## Automatic LAN match for `just lan-selftest` (two headless instances).
##
## Orders the own tanks around every 40 ticks (a move plus a queued
## waypoint, once a patrol: prompt 031), so both players send commands; both sides hold fire, so combat does not end the match first.
## Once the simulation reached TICKS and every state hash up to
## there was confirmed by the other peer, the client (player 1) surrenders
## (prompt 027); passes (exit code 0) when both see the decided match on their
## result screen, the host as victory and the client as defeat, and the
## result tick is confirmed. Fails (exit code 1) on a desync, a lost
## connection or after TIMEOUT seconds.
##
## Teams (prompt 037, `just lan-selftest`: `--lan-computers=2` on
## `twin_meadow`): the two humans play together against two passive
## computer players the host runs; every unit starts with hold fire. The
## client surrenders first: the match must go on (its ally still stands);
## then the host surrenders and both must see the computers' team win.

const TICKS := 400
const ORDER_INTERVAL := 40
## Wall-clock limits in milliseconds (`--fixed-fps` decouples frame time).
const TIMEOUT_MS := 120000
## Keep the connection serviced this long after passing, so the other peer
## still receives our last hash report.
const LINGER_MS := 1000
const TARGETS: Array[Vector2] = [
	Vector2(-60, 40), Vector2(50, -30), Vector2(0, 80), Vector2(-40, -60),
]

var sim_host: SimHost
var main: Node3D

var _start_ms := Time.get_ticks_msec()
var _last_order_tick := -1
var _orders := 0
var _passed_ms := -1
var _surrendered := false
var _host_surrendered := false


func _ready() -> void:
	# The tanks would fight on their own (prompt 017) and decide the match
	# before the surrender this test checks: both sides hold fire.
	sim_host.set_hold_fire(_own_units(), true)


func _process(_delta: float) -> void:
	var tick := sim_host.current_tick()
	var status := sim_host.net_status()
	if _passed_ms >= 0:
		if Time.get_ticks_msec() - _passed_ms >= LINGER_MS:
			get_tree().quit(0)
			set_process(false)
		return
	if tick >= TICKS and status["verified_tick"] >= TICKS and not _surrendered:
		_surrendered = true
		print("LAN self-test: tick %d confirmed by all, %d orders, input delay %d" % [
			status["verified_tick"], _orders, status["input_delay"],
		])
		if sim_host.local_player() == 1:
			sim_host.surrender()
	var result := sim_host.match_result()
	if _teams() and _surrendered and not _host_surrendered and sim_host.local_player() == 0 \
			and result["players"][1]["defeated"] and status["verified_tick"] >= int(result["players"][1]["defeat_tick"]):
		if result["decided"]:
			_fail("the match ended with the first surrender: %s" % result)
			return
		_host_surrendered = true
		print("LAN self-test: player 1 surrendered, the match goes on; the host surrenders too")
		sim_host.surrender()
	if _surrendered and result["decided"] and status["verified_tick"] >= int(result["tick"]):
		_check_result(result)
		return
	if status["state"] == "halted":
		_fail(status["message"])
	elif Time.get_ticks_msec() - _start_ms > TIMEOUT_MS:
		_fail("timed out at tick %d (confirmed %d)" % [tick, status["verified_tick"]])
	elif tick / ORDER_INTERVAL != _last_order_tick / ORDER_INTERVAL and tick > 0 and not _surrendered:
		_last_order_tick = tick
		_order_own_units()


## Both peers must agree: player 0 wins because player 1 surrendered, and
## the result screen says so.
func _check_result(result: Dictionary) -> void:
	var expected := "VICTORY" if sim_host.local_player() == 0 and not _teams() else "DEFEAT"
	var winner := 2 if _teams() else 0
	if not main.game_menu.result_panel.visible:
		return  # shown in the game menu's next _process
	var shown: String = main.game_menu.result_title.text
	if result["winner"] != winner or result["players"][1]["reason"] != "surrendered":
		_fail("unexpected result %s" % result)
	elif _teams() and (result["players"][0]["reason"] != "surrendered" or sim_host.match_players().size() != 4):
		_fail("unexpected team result %s" % result)
	elif shown != expected:
		_fail("result screen shows '%s', expected %s" % [shown, expected])
	else:
		print("LAN self-test PASSED: player %d, %s at tick %d, confirmed %d, hash %s" % [
			sim_host.local_player(), expected, result["tick"], sim_host.net_status()["verified_tick"],
			sim_host.state_hash(),
		])
		_passed_ms = Time.get_ticks_msec()


## A team match (more than two players).
func _teams() -> bool:
	return sim_host.match_players().size() > 2


func _own_units() -> PackedInt64Array:
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var own := PackedInt64Array()
	for i in ids.size():
		if owners[i] == sim_host.local_player():
			own.append(ids[i])
	return own


func _order_own_units() -> void:
	var own := _own_units()
	var target := TARGETS[(_orders + sim_host.local_player()) % TARGETS.size()]
	sim_host.move_units(own, target.x, target.y)
	# Waypoint chains and a patrol go over the network too (prompt 031).
	var next := TARGETS[(_orders + 1 + sim_host.local_player()) % TARGETS.size()]
	if _orders == 4:
		sim_host.patrol_units(own, next.x, next.y)
	else:
		sim_host.queue_move(own, next.x, next.y)
	_orders += 1


func _fail(reason: String) -> void:
	print("LAN self-test FAILED: player %d: %s" % [sim_host.local_player(), reason])
	get_tree().quit(1)
	set_process(false)
