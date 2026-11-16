extends Control
## LAN lobby (prompt 025): host a game, search the LAN, join, get ready and
## start together. Opened from the main menu ("LAN game", prompt 027) or
## directly with `just lan`; Back returns to the main menu. The host picks
## the map.
##
## All networking happens in `LanLobby` (Rust, `net::lan`). When the match
## starts, the lobby is handed to `main.tscn` through `lan_match.gd`.
##
## Command line (after `--`), for tests and quick starts:
##   --lan-host                 host right away, start when everyone is ready
##   --lan-join[=ip[:port]]     join the address, or the first game found
##   --lan-name=<name>          player name
##   --lan-port=<port>          game port (default 47810)
##   --lan-discovery-port=<p>   search port (default 47811)
##   --lan-self-test            play automatically, quit with exit code 0/1
##   --map=<name>              host: play on data/maps/<name>/
##   --lan-computers=<n>       host: the last n slots get passive computer
##                             players on one team, the humans on the other
##                             (prompt 037; the self-test plays 2 against 2)
##   --mode=<battle|base>      host: the mode
##
## Slots (prompt 037): the host sets who plays in each slot (open, closed,
## a computer and its difficulty), teams and factions; a client changes
## its own team and faction. The number of slots is the map's start
## positions. Computer players run on the host.

const LanMatch := preload("res://scenes/lan/lan_match.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const SlotList := preload("res://scenes/menu/slot_list.gd")
const MAIN_SCENE := "res://scenes/main.tscn"
const SEED := 42
## Milliseconds between search queries while the search is on.
const SEARCH_INTERVAL_MS := 1000
## Automatic modes give up after this many milliseconds. Wall-clock time,
## because tests run with `--fixed-fps`, where frame time is not real time.
const AUTO_TIMEOUT_MS := 60000

var _lobby := LanLobby.new()
var _args := {}
var _next_search_ms := 0
var _searching := false
var _auto_start_ms := Time.get_ticks_msec()

var _name: LineEdit
var _port: SpinBox
var _discovery_port: SpinBox
var _address: LineEdit
var _games: ItemList
var _players: Label
var _ready_box: CheckBox
var _start: Button
var _leave: Button
var _status: Label
var _setup: Control
var _map: OptionButton
var _maps: PackedStringArray = []
var _mode: OptionButton
var _shared_vision: CheckBox
var _slot_list: SlotList
var _slot_hint: Label
## Returns to the main menu (prompt 027).
var back_button: Button


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		var parts := arg.split("=", true, 1)
		_args[parts[0]] = parts[1] if parts.size() > 1 else ""
	LanMatch.self_test = _args.has("--lan-self-test")
	LanMatch.kind = "LAN"
	Settings.ensure_loaded()
	theme = MenuTheme.get_theme()
	add_child(MenuTheme.background())
	_build_ui()
	if _args.has("--lan-host"):
		_on_host()
	elif _args.has("--lan-join"):
		if _args["--lan-join"] == "":
			_on_search()
		else:
			_address.text = _args["--lan-join"]
			_on_join_address()


func _process(_delta: float) -> void:
	_lobby.poll()
	if _searching and Time.get_ticks_msec() >= _next_search_ms:
		_lobby.search(int(_discovery_port.value))
		_next_search_ms = Time.get_ticks_msec() + SEARCH_INTERVAL_MS
	_update_games()
	_update_lobby()
	if _is_auto():
		_auto_step()


func _build_ui() -> void:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(560, 0)
	center.add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)

	box.add_child(MenuTheme.title("LAN game", MenuTheme.HEADING_SIZE))

	_setup = VBoxContainer.new()
	box.add_child(_setup)
	_name = LineEdit.new()
	_name.placeholder_text = "Your name"
	_name.text = _args.get("--lan-name", "Player")
	_setup.add_child(_labeled("Name", _name))

	_port = _spin(_args.get("--lan-port", str(LanLobby.DEFAULT_GAME_PORT)))
	_discovery_port = _spin(_args.get("--lan-discovery-port", str(LanLobby.DEFAULT_DISCOVERY_PORT)))
	var ports := HBoxContainer.new()
	ports.add_child(_labeled("Game port", _port))
	ports.add_child(_labeled("Search port", _discovery_port))
	_setup.add_child(ports)

	_map = OptionButton.new()
	_maps = MatchSetup.available_maps()
	for map_name in _maps:
		_map.add_item(map_name)
	if _args.has("--map") and _args["--map"] in _maps:
		_map.select(Array(_maps).find(_args["--map"]))
	_setup.add_child(_labeled("Map (host)", _map))
	_mode = OptionButton.new()
	for id in MatchSetup.MODES:
		_mode.add_item(MatchSetup.MODES[id])
	_mode.select(MatchSetup.MODES.keys().find(_args.get("--mode", "battle")))
	_setup.add_child(_labeled("Mode (host)", _mode))
	_shared_vision = CheckBox.new()
	_shared_vision.text = "Allies share what they see"
	_shared_vision.button_pressed = true
	_setup.add_child(_shared_vision)

	var host := Button.new()
	host.text = "Host game"
	host.pressed.connect(_on_host)
	_setup.add_child(host)

	var search := Button.new()
	search.text = "Search LAN"
	search.pressed.connect(_on_search)
	_setup.add_child(search)
	_games = ItemList.new()
	_games.custom_minimum_size = Vector2(0, 110)
	_games.item_activated.connect(func(_index: int) -> void: _on_join_selected())
	_setup.add_child(_games)
	var join := Button.new()
	join.text = "Join selected game"
	join.pressed.connect(_on_join_selected)
	_setup.add_child(join)

	_address = LineEdit.new()
	_address.placeholder_text = "192.168.1.20 or 192.168.1.20:47810"
	_address.text_submitted.connect(func(_text: String) -> void: _on_join_address())
	var by_address := Button.new()
	by_address.text = "Join address"
	by_address.pressed.connect(_on_join_address)
	var address_row := HBoxContainer.new()
	_address.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	address_row.add_child(_address)
	address_row.add_child(by_address)
	_setup.add_child(address_row)

	_players = Label.new()
	box.add_child(_players)
	_slot_list = SlotList.new()
	_slot_list.allow_open = true
	_slot_list.kind_changed.connect(func(index: int, kind: String, level: String) -> void:
		_lobby.set_slot_kind(index, kind, level))
	_slot_list.side_changed.connect(func(index: int, team: int, faction: String) -> void:
		_lobby.set_slot_side(index, team, faction))
	box.add_child(_slot_list)
	_slot_hint = MenuTheme.hint("")
	box.add_child(_slot_hint)
	_ready_box = CheckBox.new()
	_ready_box.text = "Ready"
	_ready_box.toggled.connect(func(on: bool) -> void: _lobby.set_ready(on))
	box.add_child(_ready_box)
	_start = Button.new()
	_start.text = "Start match"
	_start.pressed.connect(_on_start)
	box.add_child(_start)
	_leave = Button.new()
	_leave.text = "Leave"
	_leave.pressed.connect(_on_leave)
	box.add_child(_leave)
	_status = Label.new()
	_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_status)
	back_button = Button.new()
	back_button.text = "Back to main menu"
	back_button.pressed.connect(_on_back)
	box.add_child(back_button)


func _labeled(text: String, control: Control) -> Control:
	var row := HBoxContainer.new()
	var label := Label.new()
	label.text = text
	label.custom_minimum_size = Vector2(90, 0)
	row.add_child(label)
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(control)
	return row


func _spin(value: String) -> SpinBox:
	var spin := SpinBox.new()
	spin.min_value = 1
	spin.max_value = 65535
	spin.value = int(value)
	return spin


func _on_host() -> void:
	_searching = false
	if _lobby.host(_name.text, int(_port.value), int(_discovery_port.value)):
		print("LAN: hosting on port %d" % _lobby.port())
		_apply_match_settings()


## Host: map, mode, shared vision and one slot per start position of the
## map, with the teams the map suggests (prompt 037).
func _apply_match_settings() -> void:
	var map_name := _selected_map()
	var starts := MatchSetup.map_starts(map_name)
	var mode: String = MatchSetup.MODES.keys()[_mode.selected]
	_lobby.set_match(map_name, mode, _shared_vision.button_pressed, starts.size())
	for s in starts:
		_lobby.set_slot_side(s["player"], s["team"], "")
	if _args.has("--lan-computers"):
		var computers := int(_args["--lan-computers"])
		var slots := starts.size()
		for i in slots:
			var computer := i >= slots - computers
			if computer:
				_lobby.set_slot_kind(i, "computer", "passive")
			_lobby.set_slot_side(i, 1 if computer else 0, "")
		# Nobody fires on their own: the test ends by surrender.
		_lobby.hold_fire_at_start = LanMatch.self_test


func _selected_map() -> String:
	if _args.has("--map") and _args["--map"] in _maps:
		return _args["--map"]
	return _maps[_map.selected]


func _on_search() -> void:
	_searching = true
	_next_search_ms = 0


func _on_join_selected() -> void:
	var selected := _games.get_selected_items()
	var games := _lobby.found_games()
	var index := selected[0] if not selected.is_empty() else 0
	if index < games.size():
		_join(games[index]["address"])


func _on_join_address() -> void:
	var address := _address.text.strip_edges()
	if not address.is_empty():
		_join(address)


func _join(address: String) -> void:
	_searching = false
	_ready_box.button_pressed = false
	if _lobby.join(_name.text, address):
		print("LAN: joining %s" % address)


func _on_start() -> void:
	var map_dir := ""
	if _args.has("--map"):
		map_dir = GamePaths.map_dir(_args["--map"])
	else:
		map_dir = MatchSetup.map_dir(_maps[_map.selected])
	_lobby.start_match(SEED, map_dir)


func _on_back() -> void:
	set_process(false)
	_lobby.leave()
	get_tree().change_scene_to_file(MatchSetup.MENU_SCENE)


func _on_leave() -> void:
	_lobby.leave()
	_ready_box.button_pressed = false


func _update_games() -> void:
	var games := _lobby.found_games()
	if games.size() == _games.item_count and not _searching:
		return
	var selected := _games.get_selected_items()
	_games.clear()
	for game in games:
		var compatible: bool = game["compatible"]
		_games.add_item("%s  –  %s  (%d/%d)%s" % [
			game["name"], game["address"], game["players"], game["max_players"],
			"" if compatible else "  incompatible version",
		])
	if not selected.is_empty() and selected[0] < _games.item_count:
		_games.select(selected[0])


func _update_lobby() -> void:
	var state := _lobby.state()
	var in_game := state == "connecting" or state == "lobby"
	_setup.visible = not in_game
	_ready_box.visible = state == "lobby" and not _lobby.is_host()
	_start.visible = state == "lobby" and _lobby.is_host()
	_start.disabled = not _lobby.can_start()
	_leave.visible = in_game
	back_button.visible = not in_game
	var lines: PackedStringArray = []
	for player in _lobby.players():
		lines.append("Player %d: %s%s%s" % [
			player["id"], player["name"],
			"  (you)" if player["local"] else "",
			"  – ready" if player["ready"] else "",
		])
	_players.text = "\n".join(lines)
	_players.visible = false
	_slot_list.visible = state == "lobby"
	_slot_list.host = _lobby.is_host()
	if state == "lobby":
		_slot_list.show_slots(_lobby.slots())
		var settings := _lobby.match_settings()
		_slot_hint.text = "Map: %s, mode: %s%s. %s" % [
			settings.get("map", ""), MatchSetup.MODES.get(settings.get("mode", ""), ""),
			", allies share their sight" if settings.get("shared_vision", false) else "",
			"Set who plays in each slot, teams and factions." if _lobby.is_host() else "Choose your team and faction.",
		]
	_slot_hint.visible = state == "lobby"
	var status: String
	match state:
		"idle":
			status = "Searching …" if _searching else "Host a game or join one."
		"connecting":
			status = "Connecting …"
		"lobby":
			status = ("Hosting on port %d. Waiting until everyone is ready." % _lobby.port()) \
				if _lobby.is_host() else "In the lobby. Tick Ready when you are."
		"started":
			status = "Starting …"
		_:
			status = "Closed."
	var message := _lobby.message()
	_status.text = status if message.is_empty() else "%s\n%s" % [status, message]
	if state == "started":
		_enter_match()


func _enter_match() -> void:
	set_process(false)
	LanMatch.lobby = _lobby
	get_tree().change_scene_to_file(MAIN_SCENE)


func _is_auto() -> bool:
	return _args.has("--lan-host") or _args.has("--lan-join")


## Automatic lobby for tests: join the first game found, get ready; the
## host starts as soon as everyone is ready.
func _auto_step() -> void:
	if Time.get_ticks_msec() - _auto_start_ms > AUTO_TIMEOUT_MS:
		push_error("LAN lobby: timed out in state '%s' %s" % [_lobby.state(), _lobby.message()])
		get_tree().quit(1)
		set_process(false)
		return
	var state := _lobby.state()
	if state == "closed":
		push_error("LAN lobby closed: %s" % _lobby.message())
		get_tree().quit(1)
		set_process(false)
	elif state == "idle" and _args.has("--lan-join") and not _lobby.found_games().is_empty():
		_join(_lobby.found_games()[0]["address"])
	elif state == "lobby" and not _lobby.is_host() and not _ready_box.button_pressed:
		_ready_box.button_pressed = true
	elif state == "lobby" and _lobby.can_start() and not _has_open_slot():
		_on_start()


## Whether a slot still waits for a human (the automatic host starts only
## when every slot is taken, prompt 037).
func _has_open_slot() -> bool:
	return _lobby.slots().any(func(s: Dictionary) -> bool: return s["kind"] == "open")
