extends VBoxContainer
## The player slots of a match (prompt 037): one row per slot with who
## plays there (human, computer with difficulty, open, closed), the team
## and the faction. Used by the single-player setup, the LAN lobby and the
## online lobby; it only shows and reports, the owner applies the changes.
##
## `show_slots` takes dictionaries like `LanLobby.slots()`: `index`, `kind`
## (`human`, `computer`, `open`, `closed`), `level`, `team`, `faction`,
## `name`, `ready`, `local`.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")

## The kind (and difficulty) of slot `index` was changed by the user.
signal kind_changed(index: int, kind: String, level: String)
## Team or faction of slot `index` was changed by the user.
signal side_changed(index: int, team: int, faction: String)

## Choices for slots without a human, in this order: `open` (only in
## network lobbies), `closed`, then a computer per difficulty.
var allow_open := false
## Whether the user may change slots other than their own (host, single
## player).
var host := true

var _rows: Array[Dictionary] = []
var _slots: Array = []
var _updating := false


## Shows `slots`; rebuilds the rows only when their number changes.
func show_slots(slots: Array) -> void:
	_slots = slots
	if _rows.size() != slots.size():
		for child in get_children():
			remove_child(child)
			child.queue_free()
		_rows.clear()
		for i in slots.size():
			_rows.append(_make_row(i))
	_updating = true
	for i in slots.size():
		_update_row(_rows[i], slots[i])
	_updating = false


## The kind items are built with `tr`: rebuild the rows when the language
## changes (prompt 043).
func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and not _rows.is_empty():
		_rows.clear()
		(func() -> void: show_slots(_slots)).call_deferred()


## The slots as last shown, with the user's changes (single player).
func slots() -> Array:
	return _slots


func _kind_items() -> Array[Array]:
	var items: Array[Array] = []
	if allow_open:
		items.append(["open", "", tr("MATCH_SLOT_OPEN")])
	items.append(["closed", "", tr("MATCH_SLOT_CLOSED")])
	for level: String in MatchSetup.OPPONENTS:
		items.append(["computer", level, tr("MATCH_SLOT_COMPUTER") % tr(MatchSetup.LEVEL_NAMES.get(level, level))])
	return items


func _make_row(index: int) -> Dictionary:
	var row := HBoxContainer.new()
	add_child(row)
	var label := Label.new()
	label.custom_minimum_size = Vector2(150, 0)
	label.clip_text = true
	row.add_child(label)
	var kind := OptionButton.new()
	kind.custom_minimum_size = Vector2(170, 0)
	for item in _kind_items():
		kind.add_item(item[2])
	kind.item_selected.connect(func(selected: int) -> void:
		if not _updating:
			var item: Array = _kind_items()[selected]
			kind_changed.emit(index, item[0], item[1]))
	row.add_child(kind)
	var team := OptionButton.new()
	for t in MatchSetup.MAX_PLAYERS:
		team.add_item(tr("COMMON_TEAM_N") % (t + 1))
	team.item_selected.connect(func(_selected: int) -> void: _emit_side(index))
	row.add_child(team)
	var faction := OptionButton.new()
	faction.add_item("MATCH_FACTION_SCENARIO")
	for id in MatchSetup.FACTIONS:
		faction.add_item(MatchSetup.FACTIONS[id])
	faction.item_selected.connect(func(_selected: int) -> void: _emit_side(index))
	row.add_child(faction)
	return {"label": label, "kind": kind, "team": team, "faction": faction}


func _emit_side(index: int) -> void:
	if _updating:
		return
	var row := _rows[index]
	var team: int = (row.team as OptionButton).selected
	var f: int = (row.faction as OptionButton).selected
	var faction: String = "" if f <= 0 else MatchSetup.FACTIONS.keys()[f - 1]
	side_changed.emit(index, team, faction)


func _update_row(row: Dictionary, slot: Dictionary) -> void:
	var label: Label = row.label
	var kind: OptionButton = row.kind
	var team: OptionButton = row.team
	var faction: OptionButton = row.faction
	var human: bool = slot.get("kind", "") == "human"
	var local: bool = slot.get("local", false)
	if human:
		var name_text: String = slot.get("name", "")
		label.text = "%s%s%s" % [
			name_text if not name_text.is_empty() else tr("COMMON_PLAYER_N") % (int(slot.get("index", 0)) + 1),
			(" " + tr("MATCH_SUFFIX_YOU")) if local else "",
			" ✓" if slot.get("ready", false) else "",
		]
	else:
		label.text = tr("MATCH_SLOT_N") % (int(slot.get("index", 0)) + 1)
	kind.visible = not human
	kind.disabled = not host
	if not human:
		var items := _kind_items()
		for i in items.size():
			if items[i][0] == slot.get("kind", "") and (items[i][0] != "computer" or items[i][1] == slot.get("level", "")):
				if kind.selected != i:
					kind.select(i)
	var playing: bool = human or slot.get("kind", "") == "computer"
	var editable := host or local
	team.disabled = not editable or not playing
	faction.disabled = not editable or not playing
	var t: int = clampi(int(slot.get("team", 0)), 0, team.item_count - 1)
	if team.selected != t:
		team.select(t)
	var f: int = MatchSetup.FACTIONS.keys().find(slot.get("faction", "")) + 1
	if faction.selected != f:
		faction.select(f)
