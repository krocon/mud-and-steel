extends RefCounted
## Look of all menus (prompt 027): dark field grey and olive with sand as the
## accent, Godot's default font, plain frames. Built in code, so there is no
## `.tres` to keep in sync, and cached once per run.
##
## Why this look: muted military colors fit the WW2 setting without any
## insignia (factions appear by name only); light text on dark panels stays
## readable over the 3D scene; no downloaded fonts.

const BACKGROUND := Color("1f211c")
const PANEL := Color("2e3128")
const PANEL_BORDER := Color("6b6a4f")
const BUTTON := Color("3c4033")
const BUTTON_HOVER := Color("4d5240")
const BUTTON_PRESSED := Color("5d6249")
const ACCENT := Color("c8b682")
const TEXT := Color("e6e1cf")
const TEXT_DIM := Color("a39f8c")
const VICTORY := Color("b9c98a")
const DEFEAT := Color("d58a6e")

const FONT_SIZE := 18
const TITLE_SIZE := 44
const HEADING_SIZE := 26

static var _theme: Theme
## Factor on all font sizes (setting `font_size`, prompt 043).
static var font_scale := 1.0


## `size` times the font size setting, for font size overrides.
static func font(size: int) -> int:
	return roundi(size * font_scale)


## Sets the font size factor; the shared theme follows at once.
static func set_font_scale(factor: float) -> void:
	font_scale = factor
	get_theme().default_font_size = font(FONT_SIZE)


static func get_theme() -> Theme:
	if _theme == null:
		_theme = _build()
	return _theme


## A label in the title style.
static func title(text: String, size := TITLE_SIZE) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", font(size))
	label.add_theme_color_override("font_color", ACCENT)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	return label


## A dimmed explanatory label that wraps.
static func hint(text: String) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_color_override("font_color", TEXT_DIM)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return label


## Full-screen dark background with a faint vignette band at the bottom.
static func background() -> Control:
	var rect := ColorRect.new()
	rect.color = BACKGROUND
	rect.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var band := ColorRect.new()
	band.color = Color(PANEL_BORDER, 0.12)
	band.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE)
	band.offset_top = -6
	band.mouse_filter = Control.MOUSE_FILTER_IGNORE
	rect.add_child(band)
	return rect


static func _build() -> Theme:
	var theme := Theme.new()
	theme.default_font_size = font(FONT_SIZE)
	for type in ["Label", "Button", "CheckBox", "OptionButton", "LineEdit", "SpinBox", "ItemList", "TabContainer"]:
		theme.set_color("font_color", type, TEXT)
	theme.set_color("font_hover_color", "Button", ACCENT)
	theme.set_color("font_pressed_color", "Button", ACCENT)
	theme.set_color("font_focus_color", "Button", ACCENT)
	theme.set_color("font_disabled_color", "Button", TEXT_DIM.darkened(0.3))
	theme.set_color("font_hover_color", "OptionButton", ACCENT)
	theme.set_color("font_selected_color", "TabContainer", ACCENT)
	theme.set_color("font_unselected_color", "TabContainer", TEXT_DIM)

	theme.set_stylebox("panel", "PanelContainer", _box(PANEL, PANEL_BORDER, 2, 20))
	theme.set_stylebox("panel", "TabContainer", _box(PANEL, PANEL_BORDER, 1, 14))
	theme.set_stylebox("panel", "PopupMenu", _box(PANEL, PANEL_BORDER, 1, 6))
	for type in ["Button", "OptionButton"]:
		theme.set_stylebox("normal", type, _box(BUTTON, PANEL_BORDER, 1, 8))
		theme.set_stylebox("hover", type, _box(BUTTON_HOVER, ACCENT, 1, 8))
		theme.set_stylebox("pressed", type, _box(BUTTON_PRESSED, ACCENT, 1, 8))
		theme.set_stylebox("focus", type, _box(Color.TRANSPARENT, ACCENT, 1, 8))
		theme.set_stylebox("disabled", type, _box(BUTTON.darkened(0.3), PANEL_BORDER.darkened(0.3), 1, 8))
	theme.set_stylebox("normal", "LineEdit", _box(BACKGROUND, PANEL_BORDER, 1, 6))
	theme.set_stylebox("focus", "LineEdit", _box(Color.TRANSPARENT, ACCENT, 1, 6))
	theme.set_stylebox("panel", "ItemList", _box(BACKGROUND, PANEL_BORDER, 1, 6))
	theme.set_stylebox("tab_selected", "TabContainer", _box(BUTTON_PRESSED, ACCENT, 1, 8))
	theme.set_stylebox("tab_unselected", "TabContainer", _box(BUTTON, PANEL_BORDER, 1, 8))
	theme.set_stylebox("tab_hovered", "TabContainer", _box(BUTTON_HOVER, ACCENT, 1, 8))
	theme.set_stylebox("background", "ProgressBar", _box(BACKGROUND, PANEL_BORDER, 1, 2))
	theme.set_stylebox("fill", "ProgressBar", _box(ACCENT.darkened(0.2), ACCENT, 0, 2))
	theme.set_constant("separation", "VBoxContainer", 10)
	theme.set_constant("separation", "HBoxContainer", 10)
	return theme


static func _box(fill: Color, border: Color, width: int, margin: int) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = fill
	box.border_color = border
	box.set_border_width_all(width)
	box.set_content_margin_all(margin)
	box.content_margin_top = margin * 0.6
	box.content_margin_bottom = margin * 0.6
	return box
