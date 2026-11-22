extends Control
## Order grid (prompts 031 and 095), bottom right: 5 x 3 tiles with a
## symbol each (`hud_tile.gd`, ghost tiles on one rounded panel), shown only
## while own units that take orders are selected. Every order has a fixed
## place, so the hand finds it blind; a place whose order does not fit the
## selection stays as a pale, greyed symbol on the first page and as an
## empty place on the others.
##
## Context column (right of a thin line, always kept so the grid never
## changes its width under the pointer): the orders that only some units
## have, in gold and only while they fit the selection, top to bottom in
## the order of `BUTTONS`: dismount, unlimber, camouflage, lay bridge, take
## up the bridge, engineer works. More than three (a mixed selection) open
## a further column.
##
## Pages (more than 15 orders exist):
##
## - first page: row 1 movement (move, advance, patrol, secure, stop),
##   row 2 combat (attack, ground, shell, barrage, smoke), row 3 posture
##   (fire discipline, stance, formation, occupy; the last place is free);
## - "fire" (prompt 176): fire at will, return fire, hold fire and the two
##   limits close range only and vehicles only (switches; a dot marks
##   them); the fire key cycles the three modes without the page;
## - "stance" and "formation" open a page with the choices and return to
##   the first page after one;
## - "works" (the context tile of engineers): their obstacles and
##   "positions", a further page with their field works.
##
## With only own buildings selected the grid shows the page "building":
## repair, sell and "cancel last order" in the third row.
##
## The last place of every other page leads back; the context column stays
## empty there. The page falls back to
## the first when the selection changes. Keys never depend on the page or
## on whether the grid shows: the selection reads them, not the tiles.
##
## Targeted orders (move, attack, ...) wait for a left click
## (`selection.command_mode`); stop, fire mode and the stances act at once.
## The tooltip of a tile names the order, its key and what it does; the key
## also stands small in the tile's corner, a dot marks a lasting state. A
## greyed tile's tooltip also says why it does nothing for the selection
## (prompt 175: the same reasons the simulation reports for a dropped
## order, `ORDER_REJECTED_*`, and a few of the grid's own, `HUD_WHY_*`).
##
## Base building (prompt 023): `build_open` says whether the build menu
## shows; the button for it sits in the top bar (`top_bar.gd`), the menu
## and the production bar of `base_hud.gd` in the dock above this grid
## (`hud.gd`).

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const HudTile := preload("res://scenes/hud/hud_tile.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const TransportOrders := preload("res://scenes/selection/transport_orders.gd")
const EngineerOrders := preload("res://scenes/selection/engineer_orders.gd")
const FormationOrders := preload("res://scenes/selection/formation_orders.gd")
const CrossingOrders := preload("res://scenes/selection/crossing_orders.gd")
const REFRESH := 0.2
const COLUMNS := 5
const ROWS := 3
## Place of the tile that leads back, on every page but the first.
const BACK_SLOT := COLUMNS * ROWS - 1
const PAGE_MAIN := "main"
const PAGE_BUILDING := "building"
## Places of the context column: slots from `CONTEXT_SLOT` on, three per
## column.
const CONTEXT_SLOT := COLUMNS * ROWS
const PAGE_CONTEXT := "context"
## Page a page leads back to.
const PARENTS: Dictionary[String, String] = {"stance": "main", "formation": "main", "works": "main", "positions": "works", "fire": "main"}
## id, label (translation key), action (key shown), kind, page, place
## (0..14, row by row; -1 in the context column, placed while they fit),
## tooltip (translation key or "").
## Kinds: "mode" (targeted), "now", "stance", "formation", "page" (opens
## the page named like the id), "back", "build" (no tile: the top bar),
## "work" (engineers), "layer" / "layer_now" (bridge layers), "camo",
## "service" (the selected own building: repair, sell, cancel an order).
const BUTTONS: Array[Array] = [
	["move", "HUD_CMD_MOVE", "order_move", "mode", "main", 0, "HUD_TIP_MOVE"],
	["attack_move", "HUD_CMD_ADVANCE", "order_attack_move", "mode", "main", 1, "HUD_TIP_ADVANCE"],
	["patrol", "HUD_CMD_PATROL", "order_patrol", "mode", "main", 2, "HUD_TIP_PATROL"],
	["secure", "HUD_CMD_SECURE", "order_secure", "mode", "main", 3, "HUD_TIP_SECURE"],
	["stop", "HUD_CMD_STOP", "order_stop", "now", "main", 4, ""],
	["attack", "HUD_CMD_ATTACK", "order_attack", "mode", "main", 5, "HUD_TIP_ATTACK"],
	["attack_ground", "HUD_CMD_GROUND", "order_attack_ground", "mode", "main", 6, "HUD_TIP_GROUND"],
	# Shelling a building, wall or tree until it is destroyed (prompt 093).
	["attack_object", "HUD_CMD_SHELL", "order_attack_object", "mode", "main", 7, "HUD_TIP_SHELL"],
	["barrage", "HUD_CMD_BARRAGE", "order_barrage", "mode", "main", 8, "HUD_TIP_BARRAGE"],
	["smoke", "HUD_CMD_SMOKE", "order_smoke", "mode", "main", 9, "HUD_TIP_SMOKE"],
	["fire", "HUD_CMD_FIRE_DISCIPLINE", "order_fire_mode", "page", "main", 10, "HUD_TIP_FIRE_DISCIPLINE"],
	["stance", "HUD_CMD_STANCE", "", "page", "main", 11, "HUD_TIP_STANCE"],
	["formation", "HUD_FORMATION", "order_formation", "page", "main", 12, "HUD_TIP_FORMATION"],
	["garrison", "HUD_CMD_OCCUPY", "order_garrison", "mode", "main", 13, "HUD_TIP_OCCUPY"],
	# Fire discipline (prompt 176): the modes by code, then the limits.
	["fire_at_will", "HUD_CMD_FIRE_AT_WILL", "", "firemode", "fire", 0, "HUD_TIP_FIRE_AT_WILL"],
	["return_fire", "HUD_CMD_RETURN_FIRE", "", "firemode", "fire", 1, "HUD_TIP_RETURN_FIRE"],
	["hold_fire", "HUD_CMD_HOLD_FIRE", "", "firemode", "fire", 2, "HUD_TIP_HOLD_FIRE"],
	["close_only", "HUD_CMD_CLOSE_ONLY", "", "firelimit", "fire", 3, "HUD_TIP_CLOSE_ONLY"],
	["vehicles_only", "HUD_CMD_VEHICLES_ONLY", "", "firelimit", "fire", 4, "HUD_TIP_VEHICLES_ONLY"],
	["stance_stand", "HUD_CMD_STAND", "stance_stand", "stance", "stance", 0, ""],
	["stance_kneel", "HUD_CMD_KNEEL", "stance_kneel", "stance", "stance", 1, ""],
	["stance_prone", "HUD_CMD_PRONE", "stance_prone", "stance", "stance", 2, ""],
	["stance_auto", "HUD_CMD_AUTO", "stance_auto", "stance", "stance", 3, ""],
	# Formations (prompt 046), by code.
	["formation_0", "HUD_FORMATION_AUTO", "", "formation", "formation", 0, ""],
	["formation_1", "HUD_FORMATION_LINE", "", "formation", "formation", 1, ""],
	["formation_2", "HUD_FORMATION_WEDGE", "", "formation", "formation", 2, ""],
	["formation_3", "HUD_FORMATION_COLUMN", "", "formation", "formation", 3, ""],
	["formation_4", "HUD_FORMATION_BROAD_WEDGE", "", "formation", "formation", 4, ""],
	["formation_5", "HUD_FORMATION_DISPERSED", "", "formation", "formation", 5, ""],
	# Context column: shown only while they fit.
	["disembark", "HUD_CMD_DISMOUNT", "order_disembark", "now", "context", -1, "HUD_TIP_DISMOUNT"],
	["unlimber", "HUD_CMD_UNLIMBER", "order_unlimber", "now", "context", -1, "HUD_TIP_UNLIMBER"],
	# Bridge-laying tanks and camouflage (prompt 055).
	["camouflage", "HUD_CMD_CAMOUFLAGE", "", "camo", "context", -1, "HUD_TIP_CAMOUFLAGE"],
	["bridge_lay", "HUD_CMD_LAY_BRIDGE", "", "layer", "context", -1, "HUD_TIP_LAY_BRIDGE"],
	["bridge_pickup", "HUD_CMD_TAKE_UP", "", "layer_now", "context", -1, "HUD_TIP_TAKE_UP"],
	["works", "HUD_CMD_WORKS", "", "page", "context", -1, "HUD_TIP_WORKS"],
	# Engineer orders (prompt 035): obstacles here, positions (prompt 052)
	# on their own page.
	["work_mines_at", "HUD_WORK_MINES_AT", "", "work", "works", 0, ""],
	["work_mines_ap", "HUD_WORK_MINES_AP", "", "work", "works", 1, ""],
	["work_wire", "HUD_WORK_WIRE", "", "work", "works", 2, ""],
	["work_hedgehogs", "HUD_WORK_HEDGEHOGS", "", "work", "works", 3, ""],
	["work_teeth", "HUD_WORK_TEETH", "", "work", "works", 4, ""],
	["work_clear", "HUD_WORK_CLEAR", "", "work", "works", 5, ""],
	["work_bridge", "HUD_WORK_BRIDGE", "", "work", "works", 6, ""],
	["positions", "HUD_CMD_POSITIONS", "", "page", "works", 7, "HUD_TIP_POSITIONS"],
	["work_foxhole", "HUD_WORK_FOXHOLE", "", "work", "positions", 0, ""],
	["work_trench", "HUD_WORK_TRENCH", "", "work", "positions", 1, ""],
	["work_comm_trench", "HUD_WORK_COMM_TRENCH", "", "work", "positions", 2, ""],
	["work_mg_nest", "HUD_WORK_MG_NEST", "", "work", "positions", 3, ""],
	["work_dugout", "HUD_WORK_DUGOUT", "", "work", "positions", 4, ""],
	["work_vehicle_pit", "HUD_WORK_VEHICLE_PIT", "", "work", "positions", 5, ""],
	["work_camo_net", "HUD_WORK_CAMO_NET", "", "work", "positions", 6, ""],
	["work_bunker", "HUD_WORK_BUNKER", "", "work", "positions", 7, ""],
	# One own building selected (prompt 036; `base_hud.gd` acts).
	["repair", "HUD_REPAIR", "", "service", "building", 10, "HUD_TIP_REPAIR"],
	["sell", "HUD_SELL", "", "service", "building", 11, "HUD_TIP_SELL"],
	["cancel_last", "HUD_CANCEL_LAST", "", "service", "building", 12, "HUD_TIP_CANCEL_LAST"],
	["back", "HUD_CMD_BACK", "", "back", "", BACK_SLOT, ""],
	["build", "HUD_CMD_BUILD", "", "build", "", -1, "HUD_TIP_BUILD"],
]

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Tile per id (self-test). "build" is the button of the top bar.
var buttons: Dictionary[String, Button] = {}
## Whether the build menu is open.
var build_open := false
## Page shown.
var page := PAGE_MAIN
## Formation the formation tile shows (prompt 046; self-test).
var formation_shown := 0

var _specs: Dictionary[String, Array] = {}
## Place of each context tile shown (from `CONTEXT_SLOT` on).
var _context_slots: Dictionary[String, int] = {}
## Columns of the context part (at least one, kept even when empty).
var _context_columns := 1
var _refresh_left := 0.0
var _shown_selection: Array[int] = []


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "CommandBar"
	mouse_filter = Control.MOUSE_FILTER_STOP
	for spec in BUTTONS:
		_specs[spec[0]] = spec
		if spec[3] == "build":
			continue
		var tile := HudTile.new(spec[0])
		tile.name = "Order_" + spec[0]
		tile.ghost = true
		tile.accent = spec[4] == PAGE_CONTEXT
		tile.toggle_mode = spec[3] == "mode" or spec[3] == "work" or spec[3] == "layer"
		tile.pressed.connect(press.bind(spec[0]))
		add_child(tile)
		buttons[spec[0]] = tile
	main.selection.command_mode_changed.connect(func(_mode: String) -> void: _update())
	main.selection.formation_changed.connect(func(_code: int) -> void: _update())
	visible = false
	apply_scale()
	_update()


## Sizes for the HUD factor (`hud.gd` calls it when the factor changed).
func apply_scale() -> void:
	var tile := HudTheme.tile()
	var gap := HudTheme.px(HudTheme.GAP)
	var pad := HudTheme.px(HudTheme.PAD)
	var columns := COLUMNS + _context_columns
	custom_minimum_size = Vector2(columns * tile + (columns - 1) * gap + _divider() + 2.0 * pad, ROWS * tile + (ROWS - 1) * gap + 2.0 * pad)
	size = custom_minimum_size
	for id: String in buttons:
		var b: HudTile = buttons[id]
		if b.get_parent() != self:
			continue
		b.apply_scale()
		var slot := slot_of(id)
		if slot >= 0:
			b.position = slot_rect(slot).position
	queue_redraw()


## Extra room of the line between the grid and the context column.
func _divider() -> float:
	return HudTheme.px(HudTheme.GAP * 2.0)


## Rectangle of place `slot` inside the grid: 0..14 row by row, from
## `CONTEXT_SLOT` on the context column top to bottom.
func slot_rect(slot: int) -> Rect2:
	var tile := HudTheme.tile()
	var step := tile + HudTheme.px(HudTheme.GAP)
	var pad := HudTheme.px(HudTheme.PAD)
	if slot >= CONTEXT_SLOT:
		var k := slot - CONTEXT_SLOT
		return Rect2(pad + _divider() + (COLUMNS + floorf(float(k) / ROWS)) * step, pad + (k % ROWS) * step, tile, tile)
	return Rect2(pad + (slot % COLUMNS) * step, pad + floorf(float(slot) / COLUMNS) * step, tile, tile)


## Place of the tile `id` (0..14, a context place while it shows), -1
## without one (self-test).
func slot_of(id: String) -> int:
	if not _specs.has(id):
		return -1
	if _specs[id][4] == PAGE_CONTEXT:
		return _context_slots.get(id, -1)
	return int(_specs[id][5])


## The panel and the thin line before the context column; the tiles bring
## their own ground.
func _draw() -> void:
	draw_style_box(HudTheme.panel_box(), Rect2(Vector2.ZERO, size))
	var pad := HudTheme.px(HudTheme.PAD)
	var x := slot_rect(CONTEXT_SLOT).position.x - _divider() * 0.5 - HudTheme.px(HudTheme.GAP) * 0.5
	draw_line(Vector2(x, pad + 2.0), Vector2(x, size.y - pad - 2.0), Color(HudTheme.TEXT_DIM, 0.18), 1.0)


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_refresh_left -= delta
	var selected: Array[int] = main.selection.selected
	if selected != _shown_selection:
		_shown_selection = selected.duplicate()
		page = PAGE_MAIN
		_refresh_left = 0.0
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_update()
	usec += Time.get_ticks_usec() - began


## Opens page `id` of the grid.
func open_page(id: String) -> void:
	page = id
	_update()


## Presses the tile `id` (also used by the self-test).
func press(id: String) -> void:
	var selection: Control = main.selection
	var spec: Array = _specs.get(id, [])
	if spec.is_empty():
		return
	match spec[3]:
		"mode", "work", "layer":
			selection.set_command_mode("" if selection.command_mode == id else id)
		"layer_now":
			CrossingOrders.pick_up(selection)
		"camo":
			CrossingOrders.toggle_camouflage(selection)
		"stance":
			selection.set_stance({"stance_stand": 0, "stance_kneel": 1, "stance_prone": 2, "stance_auto": -1}[id])
			page = PAGE_MAIN
		"firemode":
			selection.set_fire_mode({"fire_at_will": 0, "hold_fire": 1, "return_fire": 2}[id])
		"firelimit":
			selection.toggle_fire_limit("close" if id == "close_only" else "vehicles")
		"formation":
			selection.set_formation(id.get_slice("_", 1).to_int())
			page = PAGE_MAIN
		"page":
			page = id
		"back":
			page = PARENTS.get(page, PAGE_MAIN)
		"service":
			var service: Node = main.get("base_hud")
			if service != null and not buttons[id].disabled:
				service.call({"repair": "toggle_repair", "sell": "sell", "cancel_last": "cancel_last"}[id])
				service.refresh_now()
		"build":
			build_open = not build_open
			var base_hud: Node = main.get("base_hud")
			if base_hud != null and base_hud.has_method("refresh_now"):
				base_hud.refresh_now()
		"now":
			if id == "stop":
				selection.stop_selected()
			elif id == "disembark":
				TransportOrders.disembark(selection)
			elif id == "unlimber":
				TransportOrders.unlimber(selection)
	_update()


## Whether the build menu should show (read by `base_hud.gd`).
func build_menu_open() -> bool:
	return build_open


func _update() -> void:
	var selection: Control = main.selection
	var selected: Array[int] = selection.selected
	var ids := sim_host.unit_ids()
	var stances := sim_host.unit_stances()
	var modes := sim_host.unit_fire_modes()
	var limits := sim_host.unit_fire_limits()
	var owners := sim_host.unit_owners()
	var me := sim_host.local_player()
	var base_view: Node = main.get("base_view")
	var mobile := 0
	var squads := 0
	# Selected own units per fire mode (codes 0, 1, 2) and with each limit.
	var in_mode := [0, 0, 0]
	var limited := [0, 0]
	var guns := 0
	var smoke := 0
	for id: int in selected:
		var i := ids.find(id)
		if i < 0 or owners[i] != me:
			continue
		if base_view == null or not base_view.structures.has(id):
			mobile += 1
		if stances[i] >= 0:
			squads += 1
		if i < modes.size():
			in_mode[modes[i]] += 1
		if i < limits.size():
			limited[0] += limits[i] & 1
			limited[1] += (limits[i] >> 1) & 1
		var weapon := sim_host.unit_indirect(id)
		if weapon.y > 0.0:
			guns += 1
			if weapon.z > 0.0:
				smoke += 1
	# Only an own selection with orders brings the grid up: units, or
	# buildings that can be served; not an enemy looked at.
	var service := {}
	var base_hud: Node = main.get("base_hud")
	if mobile == 0 and base_hud != null and not selected.is_empty():
		service = base_hud.service_state()
		if not (service.service or service.production):
			service = {}
	if mobile > 0 and page == PAGE_BUILDING:
		page = PAGE_MAIN
	elif mobile == 0:
		page = PAGE_BUILDING
	HudTheme.fade(self, mobile > 0 or not service.is_empty())
	# Vehicles with passengers, towing links (prompt 034).
	var transport := TransportOrders.counts(selection)
	var engineers := EngineerOrders.engineers(selection).size()
	var layers := CrossingOrders.layers(selection).size()
	var camo := CrossingOrders.camouflageable(selection)
	# Units with a gun that damages objects (prompt 093).
	var shells := mobile > 0 and sim_host.can_shell_objects(PackedInt64Array(selected))
	formation_shown = selection.formation
	var own_count: int = in_mode[0] + in_mode[1] + in_mode[2]
	# The mode all own selected units share (-1: mixed).
	var common := -1
	for code in 3:
		if own_count > 0 and in_mode[code] == own_count:
			common = code
	var context: Array[String] = []
	for spec in BUTTONS:
		var id: String = spec[0]
		if spec[3] == "build":
			continue
		var b: HudTile = buttons[id]
		var label := tr(spec[1])
		# Whether the selection can use the order at all; a tile of the
		# first page stays in its place greyed, others leave theirs empty.
		var fits := true
		# Why a greyed tile does nothing (prompt 175).
		var why := ""
		match spec[3]:
			"mode":
				fits = mobile > 0 and not (id == "garrison" and squads == 0) \
						and not (id == "barrage" and guns == 0) and not (id == "smoke" and smoke == 0) \
						and not (id == "attack_object" and not shells)
				if mobile == 0:
					why = "ORDER_REJECTED_NO_MOBILE_UNITS"
				elif id == "garrison" and squads == 0:
					why = "ORDER_REJECTED_NO_SQUADS"
				elif id == "barrage" and guns == 0:
					why = "ORDER_REJECTED_NO_ARTILLERY"
				elif id == "smoke" and smoke == 0:
					why = "HUD_WHY_NO_SMOKE"
				elif id == "attack_object" and not shells:
					why = "ORDER_REJECTED_NO_OBJECT_WEAPON"
				b.set_pressed_no_signal(selection.command_mode == id)
			"stance":
				fits = squads > 0
				why = "ORDER_REJECTED_NO_SQUADS"
			"formation":
				fits = mobile > 0
				why = "ORDER_REJECTED_NO_MOBILE_UNITS"
				b.marked = formation_shown == id.get_slice("_", 1).to_int()
			"now":
				fits = mobile > 0 and not (id == "disembark" and transport.x == 0) \
						and not (id == "unlimber" and transport.y == 0)
				if mobile == 0:
					why = "ORDER_REJECTED_NO_MOBILE_UNITS"
				elif id == "disembark":
					why = "HUD_WHY_NO_PASSENGERS"
				elif id == "unlimber":
					why = "ORDER_REJECTED_NOTHING_TO_UNLIMBER"

			"page":
				fits = (id == "stance" and squads > 0) or (id == "formation" and mobile > 0) \
						or ((id == "works" or id == "positions") and engineers > 0)
				why = {"stance": "ORDER_REJECTED_NO_SQUADS", "formation": "ORDER_REJECTED_NO_MOBILE_UNITS",
					"works": "ORDER_REJECTED_NO_ENGINEERS", "positions": "ORDER_REJECTED_NO_ENGINEERS"}.get(id, "")
				if id == "formation":
					b.icon_id = "formation_%d" % formation_shown
					label += ": " + tr(FormationOrders.label(formation_shown))
				if id == "fire":
					fits = mobile > 0 or own_count > 0
					why = "ORDER_REJECTED_NO_MOBILE_UNITS"
					var names := ["HUD_CMD_FIRE_AT_WILL", "HUD_CMD_HOLD_FIRE", "HUD_CMD_RETURN_FIRE"]
					label += ": " + (tr(names[common]) if common >= 0 else tr("HUD_FIRE_MIXED"))
					b.icon_id = ["fire_at_will", "hold_fire", "return_fire"][common] if common >= 0 else "fire"
					b.marked = common > 0 or limited[0] > 0 or limited[1] > 0
			"firemode":
				fits = own_count > 0
				why = "ORDER_REJECTED_NO_MOBILE_UNITS"
				var code: int = {"fire_at_will": 0, "hold_fire": 1, "return_fire": 2}[id]
				b.marked = common == code
			"firelimit":
				fits = own_count > 0
				why = "ORDER_REJECTED_NO_MOBILE_UNITS"
				var k := 0 if id == "close_only" else 1
				b.marked = own_count > 0 and limited[k] == own_count
			"service":
				fits = false
				if not service.is_empty():
					match id:
						"repair":
							fits = service.can_repair
							label = tr("HUD_STOP_REPAIR") if service.repairing else tr("HUD_REPAIR")
							b.marked = service.repairing
						"sell":
							fits = service.service
							label = tr("HUD_CANCEL_SITE") if service.site else tr("HUD_SELL")
						"cancel_last":
							fits = service.can_cancel
			"work":
				fits = engineers > 0
				why = "ORDER_REJECTED_NO_ENGINEERS"
				b.set_pressed_no_signal(selection.command_mode == id)
			"layer":
				fits = layers > 0
				why = "HUD_WHY_NO_BRIDGE_LAYER"
				b.set_pressed_no_signal(selection.command_mode == id)
			"layer_now":
				fits = layers > 0
				why = "HUD_WHY_NO_BRIDGE_LAYER"
			"camo":
				fits = not camo.is_empty()
				why = "HUD_WHY_NO_CAMOUFLAGE"
				var on := 0
				for unit: int in camo:
					if sim_host.camouflage_state(unit) > 0:
						on += 1
				var covered := on > 0 and on == camo.size()
				label = tr("HUD_CMD_UNCOVER") if covered else tr("HUD_CMD_CAMOUFLAGE")
				b.marked = covered
		var on_page: bool = (spec[4] == page) or (spec[3] == "back" and PARENTS.has(page))
		b.disabled = not fits
		if spec[4] == PAGE_CONTEXT:
			# Only while it fits, on the first page, in the next free place.
			on_page = page == PAGE_MAIN and fits and mobile > 0
			if on_page:
				context.append(id)
		# Greyed in its place: the first page and a building's orders.
		b.visible = on_page and (fits or page == PAGE_MAIN or spec[3] == "service")
		var key := Settings.action_key_name(spec[2]) if not (spec[2] as String).is_empty() else ""
		b.key_text = _short_key(key)
		# The name the tile stands for (self-test, screen readers).
		b.set_meta(&"label", label)
		b.tooltip_text = _tip(spec, label, key)
		if not fits and not why.is_empty():
			b.tooltip_text += "\n" + tr("HUD_WHY_GREYED") % tr(why)
		b.set_meta(&"why", "" if fits else why)
	_place_context(context)


## Gives the context tiles that show their places, and the grid the
## columns they need (one at least, so it keeps its width).
func _place_context(context: Array[String]) -> void:
	var slots: Dictionary[String, int] = {}
	for k in context.size():
		slots[context[k]] = CONTEXT_SLOT + k
	var columns := maxi(ceili(float(context.size()) / ROWS), 1)
	if slots == _context_slots and columns == _context_columns:
		return
	_context_slots = slots
	_context_columns = columns
	apply_scale()


## A key name short enough for a tile's corner.
static func _short_key(key: String) -> String:
	var short: String = {"Comma": ",", "Period": ".", "Minus": "-", "Slash": "/"}.get(key, key)
	return short.replace("Shift+", "⇧") if ThemeDB.fallback_font.has_char(0x21E7) else short.replace("Shift+", "S+")


## Tooltip of a tile: name, key, and what the order does.
func _tip(spec: Array, label: String, key: String) -> String:
	var text := label if key.is_empty() else "%s [%s]" % [label, key]
	var what := ""
	match spec[0]:
		"attack_move":
			what = tr("HUD_TIP_ADVANCE") % Settings.action_key_name("order_move")
		"secure":
			what = tr("HUD_TIP_SECURE") % FormationOrders.SECURE_RADIUS
		"formation":
			what = tr("HUD_TIP_FORMATION") % Settings.action_key_name("order_formation")
		_:
			if spec[0] == "sell" and label == tr("HUD_CANCEL_SITE"):
				what = tr("HUD_TIP_CANCEL_SITE")
			elif spec[3] == "work":
				what = _work_tooltip(EngineerOrders.kind_of(spec[0]))
			elif not (spec[6] as String).is_empty():
				what = tr(spec[6])
	return text if what.is_empty() else text + "\n" + what


func _notification(what: int) -> void:
	# Tooltips with numbers and keys are built text: rebuild them in the
	# new language.
	if what == NOTIFICATION_TRANSLATION_CHANGED and sim_host != null and not buttons.is_empty():
		_update()


## Tooltip of an engineer button: what it does and what one piece costs.
func _work_tooltip(kind: String) -> String:
	match kind:
		"clear":
			return tr("HUD_TIP_WORK_CLEAR")
		"bridge":
			return tr("HUD_TIP_WORK_BRIDGE")
	var cost := sim_host.work_cost(kind)
	var price := ""
	if cost.size() >= 4:
		price = " " + tr("HUD_TIP_WORK_PRICE") % [cost[0], cost[1], cost[2], cost[3]]
	var how := tr("HUD_TIP_WORK_LINE") if EngineerOrders.KINDS[kind][1] else tr("HUD_TIP_WORK_POINT")
	return how + price
