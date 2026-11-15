extends Node
## Automatic LAN match for `just lan-selftest` (two headless instances).
##
## Orders the own tanks around every 40 ticks, so both players send
## commands. Passes (exit code 0) once the simulation reached TICKS and every
## state hash up to there was confirmed by the other peer; fails (exit code 1)
## on a desync, a lost connection or after TIMEOUT seconds.

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

var _start_ms := Time.get_ticks_msec()
var _last_order_tick := -1
var _orders := 0
var _passed_ms := -1


func _process(_delta: float) -> void:
	var tick := sim_host.current_tick()
	var status := sim_host.net_status()
	if _passed_ms >= 0:
		if Time.get_ticks_msec() - _passed_ms >= LINGER_MS:
			get_tree().quit(0)
			set_process(false)
		return
	if tick >= TICKS and status["verified_tick"] >= TICKS:
		print("LAN self-test PASSED: player %d, tick %d, confirmed %d, %d orders, input delay %d, hash %s" % [
			sim_host.local_player(), tick, status["verified_tick"], _orders,
			status["input_delay"], sim_host.state_hash(),
		])
		_passed_ms = Time.get_ticks_msec()
		return
	if status["state"] == "halted":
		_fail(status["message"])
	elif Time.get_ticks_msec() - _start_ms > TIMEOUT_MS:
		_fail("timed out at tick %d (confirmed %d)" % [tick, status["verified_tick"]])
	elif tick / ORDER_INTERVAL != _last_order_tick / ORDER_INTERVAL and tick > 0:
		_last_order_tick = tick
		_order_own_units()


func _order_own_units() -> void:
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var own := PackedInt64Array()
	for i in ids.size():
		if owners[i] == sim_host.local_player():
			own.append(ids[i])
	var target := TARGETS[(_orders + sim_host.local_player()) % TARGETS.size()]
	sim_host.move_units(own, target.x, target.y)
	_orders += 1


func _fail(reason: String) -> void:
	print("LAN self-test FAILED: player %d: %s" % [sim_host.local_player(), reason])
	get_tree().quit(1)
	set_process(false)
