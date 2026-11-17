extends Control
## Online lobby over Steam (prompt 026): host a lobby, find one in the list,
## join by lobby code or Steam invitation, get ready and start together.
## Opened from the main menu ("Online (Steam)") or with `just online`; Back
## returns to the main menu. Same flow and match hand-over as the LAN lobby
## (`lan/lan_lobby.gd`); all Steam code is in `SteamLobby` (Rust,
## `crates/steam`).
##
## Steam is only initialized here, so single player and LAN never need it.
## Without a running Steam client (or in a build without Steam) the screen
## says why and offers a retry.
##
## Command line (after `--`), for tests with two Steam accounts:
##   --steam-host               host a lobby right away, start when all ready
##   --steam-join[=<code>]      join the lobby code, or the first lobby listed
##   --steam-friends-only       hosted lobby is for friends only
##   --steam-self-test          play automatically, quit with exit code 0/1
##   --map=<name>               host: play on data/maps/<name>/
##   --no-steam                 do not initialize Steam (flow self-test)

const LanMatch := preload("res://scenes/lan/lan_match.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const SlotList := preload("res://scenes/menu/slot_list.gd")
const MAIN_SCENE := "res://scenes/main.tscn"
const SEED := 42
## Members of the Steam lobby: the largest maps have six starts (prompt
## 037); the slots follow the map.
const MAX_PLAYERS := 6
## Milliseconds between automatic refreshes of the lobby list.
const REFRESH_INTERVAL_MS := 5000
## Automatic modes give up after this many milliseconds (wall clock).
const AUTO_TIMEOUT_MS := 120000

var _lobby := SteamLobby.new()
var _args := {}
var _connected := false
var _next_refresh_ms := 0
var _auto_start_ms := Time.get_ticks_msec()
var _lobby_lines: PackedStringArray = []

var _account: Label
var _retry: Button
var _setup: Control
var _map: OptionButton
var _maps: PackedStringArray = []
var _mode: OptionButton
var _shared_vision: CheckBox
var _slot_list: SlotList
## The host applied map, mode and slots to its new lobby (prompt 037).
var _match_applied := false
var _public: CheckBox
var _lobbies: ItemList
var _code: LineEdit
var _invitation: Button
var _in_lobby: Control
var _lobby_code: Label
var _players: Label
var _ready_box: CheckBox
var _start: Button
var _status: Label
## Returns to the main menu.
var back_button: Button


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		var parts := arg.split("=", true, 1)
		_args[parts[0]] = parts[1] if parts.size() > 1 else ""
	LanMatch.self_test = _args.has("--steam-self-test")
	LanMatch.kind = "Online"
	Settings.ensure_loaded()
	theme = MenuTheme.get_theme()
	add_child(MenuTheme.background())
	_build_ui()
	if _args.has("--no-steam") or _args.has("--self-test"):
		_status.text = "LOBBY_NO_STEAM_RUN"
		return
	_connect_steam()


func _process(_delta: float) -> void:
	if not _connected:
		return
	_lobby.poll()
	var state := _lobby.state()
	if (state == "idle" or state == "closed") and Time.get_ticks_msec() >= _next_refresh_ms:
		_refresh()
	_update_lobbies()
	_update_lobby(state)
	if _is_auto():
		_auto_step(state)


func _connect_steam() -> void:
	_connected = _lobby.connect_steam()
	_retry.visible = not _connected
	_setup.visible = _connected
	if not _connected:
		_account.text = "LOBBY_STEAM_UNAVAILABLE"
		_status.text = _lobby.message()
		if _is_auto():
			push_error("Steam lobby: %s" % _lobby.message())
			get_tree().quit(1)
		return
	_account.text = tr("LOBBY_STEAM_SIGNED_IN").format({"name": _lobby.user_name(), "app": SteamLobby.app_id()})
	var code := SteamLobby.launch_lobby_code()
	if _args.has("--steam-host"):
		_on_host()
	elif _args.get("--steam-join", "") != "":
		code = _args["--steam-join"]
	if code != "":
		_join(code)


func _build_ui() -> void:
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(600, 0)
	center.add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)

	box.add_child(MenuTheme.title("LOBBY_ONLINE_TITLE", MenuTheme.HEADING_SIZE))
	_account = MenuTheme.hint("LOBBY_STEAM_CONNECTING")
	box.add_child(_account)
	_retry = Button.new()
	_retry.text = "LOBBY_TRY_AGAIN"
	_retry.visible = false
	_retry.pressed.connect(_connect_steam)
	box.add_child(_retry)

	_setup = VBoxContainer.new()
	_setup.visible = false
	box.add_child(_setup)
	_map = OptionButton.new()
	_maps = MatchSetup.available_maps()
	for map_name in _maps:
		_map.add_item(MatchSetup.map_title(map_name))
	# Self-tests keep the small built-in map unless they name one.
	if not _args.has("--steam-self-test"):
		_map.select(MatchSetup.default_map_index(_maps))
	if _args.has("--map") and _args["--map"] in _maps:
		_map.select(Array(_maps).find(_args["--map"]))
	_setup.add_child(_labeled("LOBBY_MAP_HOST", _map))
	_mode = OptionButton.new()
	for id in MatchSetup.MODES:
		_mode.add_item(MatchSetup.MODES[id])
	_mode.select(MatchSetup.MODES.keys().find("battle"))
	_setup.add_child(_labeled("LOBBY_MODE_HOST", _mode))
	_shared_vision = CheckBox.new()
	_shared_vision.text = "LOBBY_SHARED_VISION"
	_shared_vision.button_pressed = true
	_setup.add_child(_shared_vision)
	_public = CheckBox.new()
	_public.text = "LOBBY_PUBLIC"
	_public.button_pressed = not _args.has("--steam-friends-only")
	_setup.add_child(_public)
	var host := Button.new()
	host.text = "LOBBY_HOST_GAME"
	host.pressed.connect(_on_host)
	_setup.add_child(host)

	var list_row := HBoxContainer.new()
	var list_label := Label.new()
	list_label.text = "LOBBY_OPEN_GAMES"
	list_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list_row.add_child(list_label)
	var refresh := Button.new()
	refresh.text = "LOBBY_REFRESH"
	refresh.pressed.connect(_refresh)
	list_row.add_child(refresh)
	_setup.add_child(list_row)
	_lobbies = ItemList.new()
	_lobbies.custom_minimum_size = Vector2(0, 120)
	_lobbies.item_activated.connect(func(_index: int) -> void: _on_join_selected())
	_setup.add_child(_lobbies)
	var join := Button.new()
	join.text = "LOBBY_JOIN_SELECTED"
	join.pressed.connect(_on_join_selected)
	_setup.add_child(join)

	_code = LineEdit.new()
	_code.placeholder_text = "LOBBY_CODE_HINT"
	_code.text_submitted.connect(func(_text: String) -> void: _on_join_code())
	_code.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var by_code := Button.new()
	by_code.text = "LOBBY_JOIN_CODE"
	by_code.pressed.connect(_on_join_code)
	var code_row := HBoxContainer.new()
	code_row.add_child(_code)
	code_row.add_child(by_code)
	_setup.add_child(code_row)

	_invitation = Button.new()
	_invitation.visible = false
	_invitation.pressed.connect(func() -> void: _join(_lobby.invitation_code()))
	box.add_child(_invitation)

	_in_lobby = VBoxContainer.new()
	_in_lobby.visible = false
	box.add_child(_in_lobby)
	_lobby_code = Label.new()
	var copy := Button.new()
	copy.text = "LOBBY_COPY_CODE"
	copy.pressed.connect(func() -> void: DisplayServer.clipboard_set(_lobby.lobby_code()))
	var invite := Button.new()
	invite.text = "LOBBY_INVITE"
	invite.pressed.connect(func() -> void: _lobby.invite_friends())
	var code_line := HBoxContainer.new()
	_lobby_code.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	code_line.add_child(_lobby_code)
	code_line.add_child(copy)
	code_line.add_child(invite)
	_in_lobby.add_child(code_line)
	_players = Label.new()
	_in_lobby.add_child(_players)
	_slot_list = SlotList.new()
	_slot_list.allow_open = true
	_slot_list.kind_changed.connect(func(index: int, kind: String, level: String) -> void:
		_lobby.set_slot_kind(index, kind, level))
	_slot_list.side_changed.connect(func(index: int, team: int, faction: String) -> void:
		_lobby.set_slot_side(index, team, faction))
	_in_lobby.add_child(_slot_list)
	_ready_box = CheckBox.new()
	_ready_box.text = "LOBBY_READY"
	_ready_box.toggled.connect(func(on: bool) -> void: _lobby.set_ready(on))
	_in_lobby.add_child(_ready_box)
	_start = Button.new()
	_start.text = "LOBBY_START_MATCH"
	_start.pressed.connect(_on_start)
	_in_lobby.add_child(_start)
	var leave := Button.new()
	leave.text = "LOBBY_LEAVE"
	leave.pressed.connect(_on_leave)
	_in_lobby.add_child(leave)

	_status = Label.new()
	_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_status)
	back_button = Button.new()
	back_button.text = "LOBBY_BACK_TO_MENU"
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


func _on_host() -> void:
	_match_applied = false
	if _lobby.host(MAX_PLAYERS, _public.button_pressed):
		print("Steam: creating a lobby")


## Host: map, mode, shared vision and one slot per start of the map, with
## the map's teams (prompt 037).
func _apply_match_settings() -> void:
	_match_applied = true
	var map_name: String = _args["--map"] if _args.has("--map") and _args["--map"] in _maps else _maps[_map.selected]
	var starts := MatchSetup.map_starts(map_name)
	_lobby.set_match(map_name, MatchSetup.MODES.keys()[_mode.selected], _shared_vision.button_pressed, starts.size())
	for s in starts:
		_lobby.set_slot_side(s["player"], s["team"], "")


func _refresh() -> void:
	_lobby.refresh()
	_next_refresh_ms = Time.get_ticks_msec() + REFRESH_INTERVAL_MS


func _on_join_selected() -> void:
	var selected := _lobbies.get_selected_items()
	var found := _lobby.found_lobbies()
	var index := selected[0] if not selected.is_empty() else 0
	if index < found.size():
		_join(found[index]["code"])


func _on_join_code() -> void:
	var code := _code.text.strip_edges()
	if not code.is_empty():
		_join(code)


func _join(code: String) -> void:
	_ready_box.button_pressed = false
	if _lobby.join(code):
		print("Steam: joining lobby %s" % code)


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


func _update_lobbies() -> void:
	var lines: PackedStringArray = []
	for entry in _lobby.found_lobbies():
		var compatible: bool = entry["compatible"]
		lines.append("%s  (%d/%d)%s" % [
			entry["name"], entry["players"], entry["max_players"],
			"" if compatible else "  " + tr("LOBBY_INCOMPATIBLE"),
		])
	if lines == _lobby_lines:
		return
	_lobby_lines = lines
	var selected := _lobbies.get_selected_items()
	_lobbies.clear()
	for line in lines:
		_lobbies.add_item(line)
	if not selected.is_empty() and selected[0] < _lobbies.item_count:
		_lobbies.select(selected[0])


func _update_lobby(state: String) -> void:
	var in_game := state in ["creating", "joining", "connecting", "lobby"]
	_setup.visible = not in_game
	_in_lobby.visible = in_game
	back_button.visible = not in_game
	_ready_box.visible = state == "lobby" and not _lobby.is_host()
	_start.visible = state == "lobby" and _lobby.is_host()
	_start.disabled = not _lobby.can_start()
	var code := _lobby.lobby_code()
	_lobby_code.text = tr("LOBBY_CODE") % code if code != "" else ""
	var invitation := _lobby.invitation_code()
	_invitation.visible = invitation != ""
	_invitation.text = tr("LOBBY_INVITATION") % invitation
	var lines: PackedStringArray = []
	for player in _lobby.players():
		lines.append("%s: %s%s%s" % [
			tr("COMMON_PLAYER_N") % player["id"], player["name"],
			"  " + tr("LOBBY_YOU") if player["local"] else "",
			"  – " + tr("LOBBY_READY_MARK") if player["ready"] else "",
		])
	_players.text = "\n".join(lines)
	_players.visible = false
	if state == "lobby" and _lobby.is_host() and not _match_applied:
		_apply_match_settings()
	_slot_list.visible = state == "lobby"
	_slot_list.host = _lobby.is_host()
	if state == "lobby":
		_slot_list.show_slots(_lobby.slots())
	var status: String
	match state:
		"idle":
			status = tr("LOBBY_IDLE_STEAM")
		"creating":
			status = tr("LOBBY_CREATING")
		"joining":
			status = tr("LOBBY_JOINING")
		"connecting":
			status = tr("LOBBY_CONNECTING_HOST")
		"lobby":
			status = tr("LOBBY_WAITING_STEAM") if _lobby.is_host() else tr("LOBBY_IN_LOBBY")
		"started":
			status = tr("LOBBY_STARTING")
		_:
			status = tr("LOBBY_CLOSED")
	var message := _lobby.message()
	_status.text = status if message.is_empty() else "%s\n%s" % [status, message]
	if state == "started":
		_enter_match()


func _enter_match() -> void:
	set_process(false)
	LanMatch.lobby = _lobby
	get_tree().change_scene_to_file(MAIN_SCENE)


func _is_auto() -> bool:
	return _args.has("--steam-host") or _args.has("--steam-join")


## Automatic lobby for tests: join the first lobby listed (or the code),
## get ready; the host starts as soon as everyone is ready.
func _auto_step(state: String) -> void:
	if Time.get_ticks_msec() - _auto_start_ms > AUTO_TIMEOUT_MS:
		push_error("Steam lobby: timed out in state '%s' %s" % [state, _lobby.message()])
		get_tree().quit(1)
		set_process(false)
		return
	if state == "closed":
		push_error("Steam lobby closed: %s" % _lobby.message())
		get_tree().quit(1)
		set_process(false)
	elif state == "idle" and _args.has("--steam-join") and not _lobby.found_lobbies().is_empty():
		_join(_lobby.found_lobbies()[0]["code"])
	elif state == "lobby" and not _lobby.is_host() and not _ready_box.button_pressed:
		_ready_box.button_pressed = true
	elif state == "lobby" and _lobby.can_start():
		_on_start()
