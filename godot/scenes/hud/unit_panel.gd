extends PanelContainer
## Info field of the selection (prompts 031 and 095), bottom center; shown
## only while something is selected, also an enemy or a building that is
## only looked at.
##
## - Header: rank insignia and name in a serif face on the left, the kind
##   of unit with its symbol on the right. Very faint behind everything:
##   the national marking of the unit's faction (`HudTheme.draw_emblem`).
## - One unit: its picture (`godot/assets/thumbnails/<id>.png`; rendered
##   live by `unit_portrait.gd` where a type has none), beside it hit points
##   and soldiers as symbols with numbers, bars for health, ammunition, fuel
##   and experience (prompt 036, 050; numbers in their tooltips), then
##   armor (hull and turret front / side / rear), weapons with caliber and
##   range, wounded men and warnings, stance, fire mode, passengers and
##   towing (prompt 034), and the bonuses of its rank.
##   Prompt 066: under the picture the five ratings of the unit's type
##   (`rating_bars.gd`: symbols with bars, the numbers in the tooltip; type
##   values without rank or damage), in its corner the button that opens
##   the unit card.
## - Several units: a grid of tiles with a short name, a health bar and an
##   ammunition bar (red when low);
##   click selects that unit alone, Shift + click removes it.
## - Prompt 094: the tile of the active unit (`selection.active_unit`, the
##   one clicked last) has a gold ground. Two buttons in a column on the
##   right edge, behind a thin line, as large as the order tiles (ghost
##   tiles, `hud_tile.gd`): `center_button` lets the camera glide to the
##   active unit, the toggle `follow_button` is the follow mode (key C)
##   and always shows its state (gold while on).
## - Prompt 109: an own vehicle with riders (the single unit, or the active
##   one of several) lists them as tiles under the rest (`rider_tiles.gd`):
##   click puts exactly that squad off or unhitches that gun.
## - Prompt 175: the first line of an own (or allied) unit says what it
##   does (`SimHost.unit_activity`, "Activity: drives") and, for armed
##   units, whether it fires and why not (`SimHost.unit_fire_status`,
##   "Fire: no target in sight"). A unit the simulation stopped because it
##   got stuck says so (`SimHost.stuck_units`). With several units the
##   header sums them up ("5 drive, 2 in combat, 1 cannot get through").
## - Fades in and out (`HudTheme.fade`); `hud.gd` places it.

## The unit card of this catalog id is wanted, for unit `unit` (prompt 066).
signal card_requested(type_id: String, unit: int)

const Settings := preload("res://scenes/menu/settings.gd")
const UnitStats := preload("res://scenes/hud/unit_stats.gd")
const RatingBars := preload("res://scenes/hud/rating_bars.gd")
const UnitPortrait := preload("res://scenes/hud/unit_portrait.gd")
const RankBadge := preload("res://scenes/hud/rank_badge.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const HudIcons := preload("res://scenes/hud/hud_icons.gd")
const Thumbnail := preload("res://scenes/hud/thumbnail.gd")
const Picker := preload("res://scenes/hud/picker.gd")
const RiderTiles := preload("res://scenes/hud/rider_tiles.gd")
const HudTile := preload("res://scenes/hud/hud_tile.gd")
const UNIT_SCENE: PackedScene = preload("res://scenes/unit.tscn")
const REFRESH := 0.2
## Height of the ratings strip under the portrait (prompt 066).
const RATINGS_HEIGHT := 16
## Tiles shown at most; the rest is counted.
const MAX_TILES := 24
const TILE_SIZE := Vector2(74, 40)
## Width of the field and size of the picture at HUD factor 1.
const WIDTH := 470.0
const PICTURE := Vector2(120, 80)
const STANCES := {-1: "", 0: "HUD_STANCE_STANDING", 1: "HUD_STANCE_KNEELING", 2: "HUD_STANCE_PRONE"}

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Unit ids of the tiles (self-test).
var tile_ids: Array[int] = []
## Rank line of the single unit, e.g. "Veteran, 3 kills" (prompt 050;
## self-test).
var rank_line := ""
## Bonuses of the single unit's rank as their own line ("" for a recruit;
## prompt 066; self-test).
var bonus_line := ""
## Activity and fire status of the single unit, summary of several
## (prompt 175; self-test).
var activity_line := ""
var activity_summary := ""
## Ratings of the single unit's type and the button to its unit card
## (prompt 066; self-test).
var ratings: RatingBars
var card_button: Button
## Camera buttons (prompt 094; self-test): center the active unit, and the
## toggle of the follow mode.
var center_button: Button
var follow_button: Button
var _camera_buttons: VBoxContainer
var _divider: ColorRect
## Corner of the portrait with "Values".
var _corner: HBoxContainer
var _box: VBoxContainer
## Active unit the tiles show.
var _shown_active := -1
## Riders of the shown vehicle (prompt 109; self-test).
var riders: RiderTiles

## Symbol chips of the single unit (self-test): hit points, soldiers.
var health_chip: Chip
var soldiers_chip: Chip
## Kind of the single unit as the header names it (a tab id of the picker).
var kind := ""

var _portraits: UnitPortrait
var _frame: PanelContainer
var _kind_icon: Chip
var _faction := ""
var _own_faction := ""
var _single: HBoxContainer
var _portrait: TextureRect
var _title: Label
var _health: ProgressBar
var _ammo_bar: ProgressBar
var _fuel_bar: ProgressBar
## Warning line of the single unit (prompt 036; self-test).
var supply_warning := ""
var _details: Label
var _badge: RankBadge
var _xp_bar: ProgressBar
var _grid: GridContainer
var _more: Label
var _shown: Array[int] = []
var _refresh_left := 0.0
## Catalog id of the single unit shown.
var _single_type := ""


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "UnitPanel"
	mouse_filter = Control.MOUSE_FILTER_STOP
	_portraits = UnitPortrait.new()
	_portraits.name = "Portraits"
	add_child(_portraits)
	# The content left, the camera buttons right of a thin line.
	var row := HBoxContainer.new()
	row.add_theme_constant_override(&"separation", 8)
	add_child(row)
	var box := VBoxContainer.new()
	_box = box
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_constant_override(&"separation", 4)
	row.add_child(box)
	_divider = ColorRect.new()
	_divider.color = Color(HudTheme.TEXT_DIM, 0.18)
	_divider.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(_divider)
	_camera_buttons = VBoxContainer.new()
	_camera_buttons.name = "CameraButtons"
	_camera_buttons.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_child(_camera_buttons)
	center_button = HudTile.new("center")
	center_button.name = "CenterButton"
	center_button.ghost = true
	center_button.pressed.connect(func() -> void: main.selection.center_on_active())
	_camera_buttons.add_child(center_button)
	follow_button = HudTile.new("follow")
	follow_button.name = "FollowButton"
	follow_button.ghost = true
	follow_button.toggle_mode = true
	follow_button.toggled.connect(func(on: bool) -> void: main.selection.set_follow(on, true))
	_camera_buttons.add_child(follow_button)

	# Header: rank insignia and name left, the kind of unit right.
	var header := HBoxContainer.new()
	header.add_theme_constant_override(&"separation", 6)
	box.add_child(header)
	_badge = RankBadge.new()
	_badge.name = "RankBadge"
	header.add_child(_badge)
	_title = Label.new()
	_title.add_theme_font_override(&"font", HudTheme.serif())
	_title.add_theme_color_override(&"font_color", HudTheme.TEXT)
	_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_title.clip_text = true
	_title.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	header.add_child(_title)
	_kind_icon = Chip.new()
	_kind_icon.color = HudTheme.TEXT_DIM
	header.add_child(_kind_icon)

	_single = HBoxContainer.new()
	_single.add_theme_constant_override(&"separation", 8)
	box.add_child(_single)
	# The picture with the five ratings of the type under it and the
	# button to its unit card in its corner (prompt 066).
	var picture := VBoxContainer.new()
	picture.add_theme_constant_override(&"separation", 2)
	_single.add_child(picture)
	_frame = PanelContainer.new()
	picture.add_child(_frame)
	_portrait = TextureRect.new()
	_portrait.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_portrait.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	_frame.add_child(_portrait)
	ratings = RatingBars.new(RatingBars.Layout.STRIP)
	ratings.name = "Ratings"
	ratings.mouse_filter = Control.MOUSE_FILTER_PASS
	picture.add_child(ratings)
	card_button = Button.new()
	card_button.name = "CardButton"
	card_button.text = "UNIT_CARD_BUTTON"
	card_button.pressed.connect(func() -> void:
		if _shown.size() == 1:
			card_requested.emit(_single_type, _shown[0]))
	_corner = HBoxContainer.new()
	_corner.add_theme_constant_override(&"separation", 2)
	_portrait.add_child(_corner)
	_corner.set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	_corner.add_child(card_button)
	var text := VBoxContainer.new()
	text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	text.add_theme_constant_override(&"separation", 3)
	_single.add_child(text)
	# Hit points and soldiers as symbols with numbers.
	var chips := HBoxContainer.new()
	chips.add_theme_constant_override(&"separation", 12)
	text.add_child(chips)
	health_chip = Chip.new()
	health_chip.icon_id = "health"
	chips.add_child(health_chip)
	soldiers_chip = Chip.new()
	soldiers_chip.icon_id = "soldiers"
	chips.add_child(soldiers_chip)
	# Health, ammunition and fuel in one row, so the panel keeps its
	# height (prompt 036).
	var bars := HBoxContainer.new()
	bars.add_theme_constant_override(&"separation", 4)
	text.add_child(bars)
	_health = ProgressBar.new()
	_health.show_percentage = false
	_health.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var health_fill := StyleBoxFlat.new()
	health_fill.bg_color = Color(0.45, 0.78, 0.4)
	_health.add_theme_stylebox_override(&"fill", health_fill)
	bars.add_child(_health)
	_ammo_bar = _stock_bar(Color(0.95, 0.75, 0.25), "COMMON_RESOURCE_AMMO")
	bars.add_child(_ammo_bar)
	_fuel_bar = _stock_bar(Color(0.45, 0.7, 1.0), "COMMON_RESOURCE_FUEL")
	bars.add_child(_fuel_bar)
	_xp_bar = _stock_bar(Color(0.75, 0.68, 0.5), "HUD_EXPERIENCE")
	bars.add_child(_xp_bar)
	_details = Label.new()
	_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_details.add_theme_color_override(&"font_color", HudTheme.TEXT)
	text.add_child(_details)

	_grid = GridContainer.new()
	_grid.columns = 6
	_grid.add_theme_constant_override(&"h_separation", 3)
	_grid.add_theme_constant_override(&"v_separation", 3)
	box.add_child(_grid)
	_more = Label.new()
	box.add_child(_more)
	riders = RiderTiles.new()
	riders.setup(main)
	box.add_child(riders)
	visible = false
	apply_scale()


## Sizes and fonts for the HUD factor (`hud.gd` calls it when the factor
## changed).
func apply_scale() -> void:
	add_theme_stylebox_override(&"panel", HudTheme.panel_box())
	custom_minimum_size = Vector2(HudTheme.px(WIDTH), 0.0)
	_badge.custom_minimum_size = Vector2(HudTheme.px(16), HudTheme.px(18))
	_title.add_theme_font_size_override(&"font_size", HudTheme.font(17))
	_kind_icon.apply_scale()
	_frame.add_theme_stylebox_override(&"panel", HudTheme.box(Color(Thumbnail.backdrop(_single_type), 0.6), Color(0, 0, 0, 0), 0.0))
	_portrait.custom_minimum_size = (PICTURE * HudTheme.scale).round()
	ratings.custom_minimum_size = Vector2(HudTheme.px(PICTURE.x), RATINGS_HEIGHT)
	HudTheme.style_button(card_button, 11.0)
	for chip: Chip in [health_chip, soldiers_chip]:
		chip.apply_scale()
	var bar_height := HudTheme.px(9)
	_health.custom_minimum_size = Vector2(0, bar_height)
	for bar: ProgressBar in [_ammo_bar, _fuel_bar, _xp_bar]:
		bar.custom_minimum_size = Vector2(HudTheme.px(54), bar_height)
	for bar: ProgressBar in [_health, _ammo_bar, _fuel_bar, _xp_bar]:
		bar.add_theme_stylebox_override(&"background", HudTheme.box(HudTheme.TILE, Color(0, 0, 0, 0), 0.0, 3.0))
	_details.add_theme_font_size_override(&"font_size", HudTheme.font(12))
	_more.add_theme_font_size_override(&"font_size", HudTheme.font(12))
	for button: HudTile in [center_button, follow_button]:
		button.apply_scale()
	_camera_buttons.add_theme_constant_override(&"separation", int(HudTheme.px(HudTheme.GAP)))
	_divider.custom_minimum_size = Vector2(1.0, 0.0)
	for tile: Control in _grid.get_children():
		tile.custom_minimum_size = (TILE_SIZE * HudTheme.scale).round()
	riders.apply_scale()
	size = Vector2.ZERO


## The faint national marking behind the content.
func _draw() -> void:
	var faction := _faction if not _faction.is_empty() else _own_faction
	if not faction.is_empty():
		var radius := minf(size.y * 0.42, HudTheme.px(64))
		# Left of the line before the camera buttons.
		var right := _divider.global_position.x - global_position.x if _divider != null else size.x
		HudTheme.draw_emblem(self, faction, Vector2(right - radius - HudTheme.px(12), size.y * 0.5), radius, 0.055)


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_refresh_left -= delta
	var selected: Array[int] = main.selection.selected
	if selected != _shown or _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_refresh(selected)
	var active: int = main.selection.active_unit
	if active != _shown_active:
		_shown_active = active
		_mark_active()
		if selected.size() > 1:
			riders.show_for(active)
	riders.follow_leaving()
	# The buttons show the state whoever changed it (key, hand pan, ...).
	center_button.disabled = active < 0
	if follow_button.button_pressed != main.selection.follow:
		follow_button.set_pressed_no_signal(main.selection.follow)
	usec += Time.get_ticks_usec() - began


func _refresh(selected: Array[int]) -> void:
	var changed := selected != _shown
	_shown = selected.duplicate()
	HudTheme.fade(self, not selected.is_empty())
	if selected.is_empty():
		riders.show_for(-1)
		return
	riders.show_for(selected[0] if selected.size() == 1 else main.selection.active_unit)
	if _own_faction.is_empty():
		_own_faction = HudTheme.player_faction(sim_host)
	center_button.tooltip_text = tr("HUD_TIP_CENTER") % Settings.action_key_name("camera_center")
	follow_button.tooltip_text = tr("HUD_TIP_FOLLOW") % Settings.action_key_name("camera_follow")
	if selected.size() == 1:
		_single.visible = true
		_grid.visible = false
		_more.visible = false
		tile_ids.clear()
		_show_single(selected[0])
		return
	_single.visible = false
	_grid.visible = true
	_badge.visible = false
	_kind_icon.visible = false
	_title.text = tr("HUD_SELECTED_N") % selected.size()
	activity_summary = _activity_summary(selected)
	if not activity_summary.is_empty():
		_title.text += "   ·   " + activity_summary
	_set_faction("")
	if changed:
		_build_tiles(selected)
	_update_tiles()


func _show_single(id: int) -> void:
	var info := sim_host.unit_info(id)
	if info.is_empty():
		return
	var type_id: String = info.type_id
	var picture := Thumbnail.texture(type_id)
	if picture == null:
		# No rendered picture for this type: the live portrait.
		picture = _portraits.texture_for(type_id, func() -> Node3D:
			var unit := UNIT_SCENE.instantiate() as Node3D
			main._use_catalog_model(unit, type_id, 0)
			var ring := unit.get_node_or_null("SelectionRing") as Node3D
			if ring != null:
				ring.visible = false
			return unit)
	_portrait.texture = picture
	_title.text = info.name
	if type_id != _single_type or ratings.tooltip_text.is_empty():
		_single_type = type_id
		_frame.add_theme_stylebox_override(&"panel", HudTheme.box(Color(Thumbnail.backdrop(type_id), 0.6), Color(0, 0, 0, 0), 0.0))
		kind = Picker.tab_of({"id": type_id, "structure": main.selection.is_structure.call(id)})
		_kind_icon.icon_id = "kind_" + kind
		for spec: Array in Picker.TABS:
			if spec[0] == kind:
				_kind_icon.text = tr(spec[1])
		ratings.ratings = UnitStats.ratings(type_id)
		ratings.tooltip_text = UnitStats.ratings_line(type_id)
		card_button.tooltip_text = tr("UNIT_CARD_BUTTON_TIP") % Settings.action_key_name("unit_card")
	_set_faction(HudTheme.faction_of(type_id))
	_kind_icon.visible = true
	_show_rank(id, type_id)
	_health.max_value = maxi(int(info.max_hp), 1)
	_health.value = int(info.hp)
	var lines: Array[String] = []
	# Hit points and soldiers as symbols with numbers; the words are in
	# their tooltips.
	health_chip.text = "%d / %d" % [info.hp, info.max_hp]
	health_chip.tooltip_text = tr("HUD_HIT_POINTS") % [info.hp, info.max_hp]
	_health.tooltip_text = health_chip.tooltip_text
	soldiers_chip.visible = info.has("soldiers")
	var notes: Array[String] = []
	if info.has("soldiers"):
		soldiers_chip.text = "%d / %d" % [info.soldiers, info.squad_size]
		soldiers_chip.tooltip_text = tr("HUD_SOLDIERS") % [info.soldiers, info.squad_size]
		# Prompt 049: lightly / heavily wounded (untreated) men.
		var wounded: Vector2i = sim_host.unit_wounded_counts().get(id, Vector2i.ZERO)
		if wounded != Vector2i.ZERO:
			notes.append(tr("HUD_WOUNDED") % [wounded.x, wounded.y])
	# Prompt 177: the men serving a gun.
	if info.has("crew"):
		if int(info.crew) == 0:
			notes.append(tr("HUD_CREW_ABANDONED"))
		else:
			notes.append(tr("HUD_GUN_CREW") % [info.crew, info.crew_full])
	# Only warnings and wounded men get a line of their own; the bars carry
	# the percentages in their tooltips.
	var warning := _supply_warning(id)
	if not warning.is_empty():
		notes.append(warning)
	if not notes.is_empty():
		lines.append("   ·   ".join(notes))
	var hull: PackedInt64Array = info.armor_hull
	var turret: PackedInt64Array = info.armor_turret
	if hull.size() == 3 and hull[0] > 0:
		var armor := tr("HUD_ARMOR_HULL") % [hull[0], hull[1], hull[2]]
		if turret.size() == 3:
			armor += ", " + tr("HUD_ARMOR_TURRET") % [turret[0], turret[1], turret[2]]
		lines.append(armor + " " + tr("HUD_ARMOR_SIDES"))
	activity_line = _activity_line(id)
	if not activity_line.is_empty():
		lines.push_front(activity_line)
	var weapons: Array[String] = []
	for w: Dictionary in info.weapons:
		var count := "%d× " % w.count if int(w.count) > 1 else ""
		var caliber: float = w.caliber_mm
		var mm := "%d" % roundi(caliber) if is_equal_approx(caliber, roundf(caliber)) else "%.1f" % caliber
		weapons.append("%s%s (%s mm, %d m)" % [count, w.name, mm, int(w.range_m)])
	lines.append(tr("HUD_WEAPONS") % (", ".join(weapons) if not weapons.is_empty() else tr("HUD_WEAPONS_NONE")))
	var index := sim_host.unit_ids().find(id)
	if index >= 0:
		var state: Array[String] = []
		var stance: int = sim_host.unit_stances()[index]
		if stance >= 0:
			state.append(tr("HUD_STANCE") % tr(STANCES.get(stance, "?")))
			# Prompt 047: suppression and fire teams.
			var tactics := sim_host.squad_tactics(id)
			if tactics.get("retreating", false):
				state.append(tr("HUD_STATE_RETREATING"))
			if tactics.get("pinned", false):
				state.append(tr("HUD_STATE_PINNED"))
			elif int(tactics.get("suppression", 0)) > 0:
				state.append(tr("HUD_STATE_SUPPRESSED") % int(tactics.get("suppression", 0)))
			if tactics.get("assault", false):
				state.append(tr("HUD_STATE_ASSAULT"))
			elif int(tactics.get("bounding", -1)) >= 0:
				state.append(tr("HUD_STATE_BOUNDING"))
			# Prompt 196: out of breath after a run.
			if tactics.get("tired", false):
				state.append(tr("HUD_STATE_TIRED"))
		# Prompt 176: three fire modes and the limits.
		var mode: int = sim_host.unit_fire_modes()[index]
		state.append(tr(["HUD_STATE_FIRE_AT_WILL", "HUD_STATE_HOLDING_FIRE", "HUD_STATE_RETURN_FIRE"][clampi(mode, 0, 2)]))
		var limits: int = sim_host.unit_fire_limits()[index]
		if limits & 1:
			state.append(tr("HUD_STATE_CLOSE_ONLY"))
		if limits & 2:
			state.append(tr("HUD_STATE_VEHICLES_ONLY"))
		var queued := sim_host.unit_queue_length(id)
		if sim_host.unit_patrols(id):
			state.append(tr("HUD_STATE_PATROLLING"))
		elif queued > 0:
			state.append(tr("HUD_WAYPOINTS_ONE" if queued == 1 else "HUD_WAYPOINTS_N") % queued)
		# Same line, so the panel keeps its height (prompt 034).
		var transport := _transport_line(id, index)
		if not transport.is_empty():
			state.append(transport)
		lines.append(", ".join(state))
	# The rank's bonuses stay apart from the type's values (prompt 066).
	if not bonus_line.is_empty():
		lines.append(bonus_line)
	_details.text = "\n".join(lines)


## Activities that the summary of several units groups together, and the
## order of the groups.
const ACTIVITY_GROUPS := {
	"planning": "moving", "moving": "moving", "attacking": "fighting", "fighting": "fighting",
	"bombarding": "fighting", "mounting": "transport", "dismounting": "transport",
	"riding": "transport", "hitching": "transport", "unlimbering": "transport", "towed": "transport",
	"working": "working", "in_building": "in_building", "pinned": "pinned", "idle": "idle",
}
const GROUP_ORDER: Array[String] = ["stuck", "pinned", "fighting", "moving", "transport", "working", "in_building", "idle"]


## "Activity: … · Fire: …" for an own or allied unit (prompt 175); empty
## for others.
func _activity_line(id: int) -> String:
	var index := sim_host.unit_ids().find(id)
	if index < 0:
		return ""
	var owner: int = sim_host.unit_owners()[index]
	if owner != sim_host.local_player() and sim_host.relation(owner) != 1:
		return ""
	var parts: Array[String] = []
	var activity := sim_host.unit_activity(id)
	if sim_host.stuck_units().has(id):
		parts.append(tr("HUD_ACTIVITY") % tr("ACTIVITY_STUCK"))
	elif not activity.is_empty():
		var text := tr("ACTIVITY_" + str(activity.activity).to_upper())
		var target := int(activity.target)
		if target >= 0 and str(activity.activity) in ["attacking", "fighting"]:
			text = tr("ACTIVITY_TARGET") % [text, _unit_name(target)]
		parts.append(tr("HUD_ACTIVITY") % text)
	var fire := sim_host.unit_fire_status(id)
	if not fire.is_empty():
		var text := tr("FIRE_STATUS_" + str(fire.status).to_upper())
		var target := int(fire.target)
		if target >= 0 and str(fire.status) in ["firing", "reloading", "aiming"]:
			text = tr("ACTIVITY_TARGET") % [text, _unit_name(target)]
		parts.append(tr("HUD_FIRE_STATUS") % text)
	return "   ·   ".join(parts)


## "5 drive, 2 in combat, 1 cannot get through" for own units (prompt 175).
func _activity_summary(selected: Array[int]) -> String:
	var own: Array[int] = []
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var me := sim_host.local_player()
	for id in selected:
		var i := ids.find(id)
		if i >= 0 and owners[i] == me:
			own.append(id)
	if own.is_empty():
		return ""
	var stuck := sim_host.stuck_units()
	var names := sim_host.unit_activities(PackedInt64Array(own))
	var counts := {}
	for k in own.size():
		var group: String = "stuck" if stuck.has(own[k]) else ACTIVITY_GROUPS.get(names[k], "")
		if not group.is_empty():
			counts[group] = int(counts.get(group, 0)) + 1
	var parts: Array[String] = []
	for group in GROUP_ORDER:
		if counts.has(group):
			parts.append(tr("ACTIVITY_GROUP_" + group.to_upper()) % counts[group])
	return ", ".join(parts)


func _unit_name(id: int) -> String:
	var info := sim_host.unit_info(id)
	return str(info.get("name", "?"))


## Rank insignia, name and kills after the unit's name, the experience
## bar towards the next rank (prompt 050).
func _show_rank(id: int, type_id: String) -> void:
	var xp := sim_host.unit_experience(id)
	rank_line = ""
	bonus_line = ""
	_badge.visible = not xp.is_empty()
	_xp_bar.visible = not xp.is_empty()
	if xp.is_empty():
		return
	var rank: int = xp.rank
	_badge.faction = type_id.get_slice("_", 0)
	_badge.rank = rank
	rank_line = str(xp.rank_name)
	var kills: int = xp.kills
	if kills > 0:
		rank_line += ", " + tr("HUD_KILLS_ONE" if kills == 1 else "HUD_KILLS_N") % kills
	_title.text += "   ·   " + rank_line
	var next: int = xp.next_xp
	var start: int = xp.rank_xp
	if next < 0:
		_xp_bar.value = 100
		_xp_bar.tooltip_text = tr("HUD_XP_MAX").format({"rank": xp.rank_name, "xp": xp.xp})
	else:
		_xp_bar.value = clampi(100 * (int(xp.xp) - start) / maxi(next - start, 1), 0, 100)
		_xp_bar.tooltip_text = tr("HUD_XP_NEXT").format({"rank": xp.rank_name, "xp": xp.xp, "next": next})
	if rank > 0:
		bonus_line = tr("UNIT_CARD_RANK") % tr("HUD_XP_BONUS").format({"rank": xp.rank_name,
				"dispersion": xp.dispersion_pct, "reload": xp.reload_pct, "pinned": xp.suppression_pct})
		_badge.tooltip_text = tr("HUD_XP_BONUS").format({"rank": xp.rank_name, "dispersion": xp.dispersion_pct,
				"reload": xp.reload_pct, "pinned": xp.suppression_pct})
	else:
		_badge.tooltip_text = tr("HUD_XP_NO_BONUS") % xp.rank_name


## Passengers, towing and loading state of unit `id` (prompt 034).
func _transport_line(id: int, index: int) -> String:
	var parts: Array[String] = []
	var seats := sim_host.unit_seats(id)
	if seats > 0 and sim_host.unit_passengers()[index] > 0:
		parts.append(tr("HUD_PASSENGERS") % [sim_host.unit_passengers()[index], seats])
	var towing: int = sim_host.unit_towing()[index]
	if towing >= 0:
		parts.append(tr("HUD_TOWING") % str(sim_host.unit_info(towing).get("name", tr("HUD_A_GUN"))))
	var carrier: int = sim_host.unit_carriers()[index]
	if carrier >= 0:
		parts.append(tr("HUD_TOWED_BY") % str(sim_host.unit_info(carrier).get("name", tr("HUD_A_VEHICLE"))))
	if sim_host.unit_loading()[index] != 0:
		parts.append(tr("HUD_STATE_LOADING"))
	return ", ".join(parts)


## The faction whose marking lies behind the content.
func _set_faction(faction: String) -> void:
	if faction != _faction:
		_faction = faction
		queue_redraw()


## A thin bar for a stock (prompt 036).
func _stock_bar(color: Color, tip: String) -> ProgressBar:
	var bar := ProgressBar.new()
	bar.show_percentage = false
	bar.max_value = 100
	bar.tooltip_text = tip
	var fill := StyleBoxFlat.new()
	fill.bg_color = color
	bar.add_theme_stylebox_override(&"fill", fill)
	return bar


## Sets the ammunition and fuel bars of unit `id` and returns its
## warnings (prompt 036).
func _supply_warning(id: int) -> String:
	var index := sim_host.unit_ids().find(id)
	var ammo := -1
	var fuel := -1
	var immobile := false
	if index >= 0:
		ammo = sim_host.unit_ammo()[index]
		fuel = sim_host.unit_fuel()[index]
		immobile = sim_host.unit_immobile()[index] != 0
	_ammo_bar.visible = ammo >= 0
	_ammo_bar.value = maxi(ammo, 0)
	_ammo_bar.tooltip_text = tr("HUD_AMMO_PCT") % ammo
	_fuel_bar.visible = fuel >= 0
	_fuel_bar.value = maxi(fuel, 0)
	_fuel_bar.tooltip_text = tr("HUD_FUEL_PCT") % fuel
	var low := sim_host.supply_low_pct()
	var warnings: Array[String] = []
	if ammo == 0:
		warnings.append(tr("HUD_WARN_NO_AMMO"))
	elif ammo > 0 and low.size() == 2 and ammo <= low[0]:
		warnings.append(tr("HUD_WARN_LOW_AMMO"))
	if fuel == 0:
		warnings.append(tr("HUD_WARN_NO_FUEL"))
	elif fuel > 0 and low.size() == 2 and fuel <= low[1]:
		warnings.append(tr("HUD_WARN_LOW_FUEL"))
	if immobile:
		warnings.append(tr("HUD_WARN_IMMOBILIZED"))
	supply_warning = ", ".join(warnings)
	return supply_warning


func _build_tiles(selected: Array[int]) -> void:
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	tile_ids.clear()
	var types := sim_host.unit_type_ids()
	var ids := sim_host.unit_ids()
	for id: int in selected.slice(0, MAX_TILES):
		var index := ids.find(id)
		var tile := Tile.new()
		tile.custom_minimum_size = (TILE_SIZE * HudTheme.scale).round()
		tile.unit = id
		tile.label = _short_name(types[index] if index >= 0 else "")
		tile.faction = (types[index] if index >= 0 else "").get_slice("_", 0)
		tile.active = id == main.selection.active_unit
		tile.pressed_with.connect(_on_tile)
		_grid.add_child(tile)
		tile_ids.append(id)
	_more.visible = selected.size() > MAX_TILES
	_more.text = tr("HUD_MORE") % (selected.size() - MAX_TILES)


func _update_tiles() -> void:
	var ids := sim_host.unit_ids()
	var health := sim_host.unit_health()
	var ammo := sim_host.unit_ammo()
	var low := sim_host.supply_low_pct()
	var low_ammo: int = low[0] if low.size() == 2 else 0
	var ranks := sim_host.unit_ranks()
	for tile: Tile in _grid.get_children():
		var index := ids.find(tile.unit)
		var value := health[index] if index >= 0 else 0.0
		var stock: int = ammo[index] if index >= 0 else -1
		var rank: int = ranks[index] if index >= 0 and index < ranks.size() else 0
		if not is_equal_approx(value, tile.health) or stock != tile.ammo or rank != tile.rank:
			tile.health = value
			tile.ammo = stock
			tile.rank = rank
			tile.low_ammo = low_ammo
			tile.queue_redraw()


## Unit id of the tile marked as active, -1 without one (self-test).
func active_tile() -> int:
	for tile: Tile in _grid.get_children():
		if tile.active and not tile.is_queued_for_deletion():
			return tile.unit
	return -1


## Frames the tile of the active unit (prompt 094).
func _mark_active() -> void:
	for tile: Tile in _grid.get_children():
		var active := tile.unit == _shown_active
		if active != tile.active:
			tile.active = active
			tile.queue_redraw()


## Click selects only that unit, Shift + click removes it (also used by
## the self-test).
func _on_tile(id: int, shift: bool) -> void:
	var selection: Control = main.selection
	if shift:
		selection.set_selection(selection.selected.filter(func(other: int) -> bool: return other != id))
	else:
		selection.set_selection([id])


## `de_pz4_h` -> `PZ4 H`: the catalog id without faction, short enough for
## a tile.
static func _short_name(type_id: String) -> String:
	var parts := type_id.split("_")
	if parts.size() > 1:
		parts.remove_at(0)
	return " ".join(parts).to_upper().left(12)


## A symbol with a short text beside it (hit points, soldiers, the kind of
## unit): drawn, so it needs no picture file.
class Chip:
	extends Control

	var icon_id := "":
		set(value):
			icon_id = value
			queue_redraw()
	var text := "":
		set(value):
			if text != value:
				text = value
				apply_scale()
	var color := HudTheme.TEXT

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_PASS

	func apply_scale() -> void:
		var font_size := HudTheme.font(13)
		var width := ThemeDB.fallback_font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
		custom_minimum_size = Vector2(HudTheme.px(18) + 4.0 + ceilf(width), HudTheme.px(18))
		queue_redraw()

	func _draw() -> void:
		var side := HudTheme.px(16)
		HudIcons.draw(self, icon_id, Rect2(0.0, (size.y - side) * 0.5, side, side), color, maxf(1.4 * HudTheme.scale, 1.1))
		var font_size := HudTheme.font(13)
		draw_string(ThemeDB.fallback_font, Vector2(HudTheme.px(18) + 4.0, size.y * 0.5 + font_size * 0.36), text,
			HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, color)


## One unit of a multiple selection.
class Tile:
	extends Control

	signal pressed_with(id: int, shift: bool)

	var unit := -1
	var label := ""
	var health := 1.0
	## Ammunition in percent, -1 without a stock (prompt 036).
	var ammo := -1
	var low_ammo := 0
	## Rank insignia in the corner (prompt 050).
	var rank := 0
	var faction := "de"
	## The active unit (prompt 094): a gold ground.
	var active := false

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		tooltip_text = "HUD_TIP_TILE"

	func _gui_input(event: InputEvent) -> void:
		var button := event as InputEventMouseButton
		if button != null and button.pressed and button.button_index == MOUSE_BUTTON_LEFT:
			pressed_with.emit(unit, button.shift_pressed)
			accept_event()

	func _draw() -> void:
		var rect := Rect2(Vector2.ZERO, size)
		draw_style_box(HudTheme.plate(HudTheme.GHOST_ON if active else HudTheme.GHOST_HOVER), rect)
		var room := size.x - (20 if rank > 0 else 8)
		var font_size := HudTheme.font(12)
		draw_string(ThemeDB.fallback_font, Vector2(4, 5 + font_size), label, HORIZONTAL_ALIGNMENT_LEFT, room, font_size)
		if rank > 0:
			RankBadge.draw_badge(self, Rect2(size.x - 13, 3, 10, 12), faction, rank)
		var bar := Rect2(4, size.y - 10, size.x - 8, 5)
		draw_rect(bar, Color(0, 0, 0, 0.7))
		var color := Color(0.3, 0.9, 0.3) if health > 0.5 else (Color(0.95, 0.75, 0.2) if health > 0.25 else Color(0.95, 0.3, 0.2))
		draw_rect(Rect2(bar.position, Vector2(bar.size.x * health, bar.size.y)), color)
		if ammo >= 0:
			var stock := Rect2(4, size.y - 4, size.x - 8, 2)
			draw_rect(stock, Color(0, 0, 0, 0.7))
			var ammo_color := Color(0.95, 0.75, 0.25) if ammo > low_ammo else Color(0.95, 0.2, 0.15)
			draw_rect(Rect2(stock.position, Vector2(stock.size.x * ammo / 100.0, stock.size.y)), ammo_color)
