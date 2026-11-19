extends PanelContainer
## Top bar of the match HUD (prompt 095), top right, always shown: the
## resources, the build button, help and menu.
##
## - Resources (matches with base building): a symbol and the stock of
##   fuel, ammunition and manpower, the income per minute small beside it
##   (name and numbers in the tooltip); red while a stock is empty.
## - "Build" opens and closes the build menu (`command_bar.press("build")`;
##   the menu sits above the order grid).
## - "?" opens the settings on the page with the keys, the menu button the
##   pause menu (Escape).
##
## Presentation only: the numbers come from `SimHost.economy_state`.

const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const HudTile := preload("res://scenes/hud/hud_tile.gd")
const HudIcons := preload("res://scenes/hud/hud_icons.gd")
const RESOURCES: Array[Array] = [
	["fuel", "COMMON_RESOURCE_FUEL"],
	["ammo", "COMMON_RESOURCE_AMMO"],
	["manpower", "COMMON_RESOURCE_MANPOWER"],
]
const REFRESH := 0.25

var main: Node3D
var sim_host: SimHost
## Time of this frame's `_process` in microseconds (hud.gd).
var usec := 0
## Stock per resource as shown (self-test).
var amounts: Dictionary[String, int] = {}
var build_button: HudTile
var help_button: HudTile
var menu_button: HudTile

var _row: HBoxContainer
var _stocks: Dictionary[String, Stock] = {}
var _refresh_left := 0.0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "TopBar"
	mouse_filter = Control.MOUSE_FILTER_STOP
	_row = HBoxContainer.new()
	add_child(_row)
	build_button = HudTile.new("build", HudTheme.SMALL_TILE_PX)
	build_button.name = "Build"
	build_button.toggle_mode = true
	build_button.tooltip_text = "HUD_TIP_BUILD"
	build_button.pressed.connect(func() -> void: main.hud.command_bar.press("build"))
	_row.add_child(build_button)
	for spec in RESOURCES:
		var stock := Stock.new()
		stock.name = "Stock_" + spec[0]
		stock.icon_id = spec[0]
		_row.add_child(stock)
		_stocks[spec[0]] = stock
	help_button = HudTile.new("help", HudTheme.SMALL_TILE_PX)
	help_button.name = "Help"
	help_button.tooltip_text = "HUD_TIP_HELP"
	help_button.pressed.connect(open_help)
	_row.add_child(help_button)
	menu_button = HudTile.new("menu", HudTheme.SMALL_TILE_PX)
	menu_button.name = "Menu"
	menu_button.tooltip_text = "HUD_TIP_MENU"
	menu_button.pressed.connect(func() -> void:
		var menu: Node = main.get("game_menu")
		if menu != null:
			menu.open_pause())
	_row.add_child(menu_button)
	apply_scale()
	_refresh()


## Opens the pause menu on the settings page that lists the keys.
func open_help() -> void:
	var menu: Node = main.get("game_menu")
	if menu != null:
		menu.open_controls()


## Sizes for the HUD factor (`hud.gd` calls it when the factor changed).
func apply_scale() -> void:
	add_theme_stylebox_override(&"panel", HudTheme.box(HudTheme.PANEL, HudTheme.PANEL_BORDER, HudTheme.px(3)))
	_row.add_theme_constant_override(&"separation", int(HudTheme.px(HudTheme.GAP)))
	for tile: HudTile in [build_button, help_button, menu_button]:
		tile.apply_scale()
	for id: String in _stocks:
		_stocks[id].apply_scale()
	size = Vector2.ZERO


func _process(delta: float) -> void:
	var began := Time.get_ticks_usec()
	_refresh_left -= delta
	if _refresh_left <= 0.0:
		_refresh_left = REFRESH
		_refresh()
	usec += Time.get_ticks_usec() - began


func _refresh() -> void:
	var state := sim_host.economy_state()
	var economy: bool = state.get("enabled", false)
	build_button.visible = economy
	build_button.set_pressed_no_signal(main.hud != null and main.hud.command_bar.build_open)
	for spec in RESOURCES:
		var stock: Stock = _stocks[spec[0]]
		stock.visible = economy
		if not economy:
			continue
		var income: PackedInt64Array = state.income
		var amount := int(state[spec[0]])
		amounts[spec[0]] = amount
		stock.set_numbers(amount, income[RESOURCES.find(spec)])
		stock.tooltip_text = tr("HUD_RESOURCE_AMOUNT") % [tr(spec[1]), amount, income[RESOURCES.find(spec)]]


## One resource: symbol, stock, and the income small beside it.
class Stock:
	extends Control

	var icon_id := ""
	var amount := 0
	var income := 0

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_PASS

	func set_numbers(new_amount: int, new_income: int) -> void:
		if new_amount != amount or new_income != income:
			amount = new_amount
			income = new_income
			queue_redraw()

	func apply_scale() -> void:
		custom_minimum_size = Vector2(HudTheme.px(100), HudTheme.px(HudTheme.SMALL_TILE_PX))
		queue_redraw()

	func _draw() -> void:
		var font := ThemeDB.fallback_font
		var short := amount <= 0
		var color := HudTheme.RED if short else HudTheme.TEXT
		var side := HudTheme.px(18)
		HudIcons.draw(self, icon_id, Rect2(HudTheme.px(4), (size.y - side) * 0.5, side, side),
			HudTheme.RED if short else HudTheme.GOLD, maxf(1.5 * HudTheme.scale, 1.1))
		var big := HudTheme.font(15)
		var x := HudTheme.px(27)
		var text := str(amount)
		draw_string(font, Vector2(x, size.y * 0.5 + big * 0.36), text, HORIZONTAL_ALIGNMENT_LEFT, -1, big, color)
		x += font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, big).x + HudTheme.px(4)
		var small := HudTheme.font(10)
		draw_string(font, Vector2(x, size.y * 0.5 + big * 0.36), "+%d" % income, HORIZONTAL_ALIGNMENT_LEFT, -1, small, HudTheme.TEXT_DIM)
