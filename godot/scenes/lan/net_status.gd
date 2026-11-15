extends Label
## Status line of a LAN match (prompt 025): player, input delay, round trip,
## confirmed tick; "waiting" while remote frames are missing; why the match
## stopped (lost connection, desync). On a desync the replay is saved at once
## as `user://replays/desync-p<player>-<date-time>.msreplay`; compare the
## replays of all peers with `just replay`.
##
## Added by `main.gd` when `SimHost.is_networked()`. With `--lan-self-test`
## it also adds `lan_self_test.gd`.

const LanMatch := preload("res://scenes/lan/lan_match.gd")
const LanSelfTest := preload("res://scenes/lan/lan_self_test.gd")

var main: Node3D

var _halt_handled := false


func _ready() -> void:
	position = Vector2(12, 12)
	add_theme_color_override("font_shadow_color", Color.BLACK)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_focus_own_units()
	if LanMatch.self_test:
		var test := LanSelfTest.new()
		test.sim_host = main.sim_host
		test.main = main
		add_child(test)


func _process(_delta: float) -> void:
	var host: SimHost = main.sim_host
	var status := host.net_status()
	var line := "LAN · player %d of %d · input delay %d ticks · RTT %s · confirmed tick %d" % [
		host.local_player(), status["players"], status["input_delay"],
		"%d ms" % status["rtt_ms"] if status["rtt_ms"] >= 0 else "?",
		status["verified_tick"],
	]
	match status["state"]:
		"waiting":
			text = line + "\nWaiting for the other players …"
			modulate = Color(1, 0.9, 0.5)
		"halted":
			if not _halt_handled:
				_halt_handled = true
				_on_halt(status)
		_:
			text = line
			modulate = Color.WHITE


## Every player starts looking at their own tanks (main.gd frames player 0's).
func _focus_own_units() -> void:
	var host: SimHost = main.sim_host
	var owners := host.unit_owners()
	var positions := host.current_positions()
	var sum := Vector3.ZERO
	var count := 0
	for i in owners.size():
		if owners[i] == host.local_player():
			sum += positions[i]
			count += 1
	if count > 0:
		main.camera_rig.snap_to(sum / count, 140.0)


func _on_halt(status: Dictionary) -> void:
	var message: String = status["message"]
	if main.sim_host.match_result()["decided"] and not status["desync"]:
		# The match is over (prompt 027); the others leaving is expected.
		text = "Match over. %s" % message
		modulate = Color(0.8, 0.8, 0.8)
		return
	var replay := ""
	if status["desync"]:
		replay = main.save_replay("desync-p%d-" % main.sim_host.local_player())
	text = "Match stopped: %s%s" % [message, "\nReplay: %s" % replay if replay != "" else ""]
	modulate = Color(1, 0.45, 0.4)
	push_warning("LAN match stopped: %s" % message)
