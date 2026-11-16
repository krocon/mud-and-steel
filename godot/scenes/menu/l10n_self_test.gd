extends RefCounted
## Language and accessibility in the flow self-test (prompt 043), driven by
## `flow_self_test.gd`:
##
## - `menu_test`: switches the settings screen to German, finds no text
##   without translation in any menu screen or settings tab, checks unit
##   names from the catalog and a mission text in German, sets every
##   setting of prompt 043 (and a key with a modifier), saves, reloads the
##   file as a restart does and finds every value again; then back to the
##   defaults (English for the rest of the test).
## - `match_test`: German during a match: no untranslated text in the HUD,
##   the pause menu and the base HUD; subtitles for a warning; back to
##   English.
##
## "Without translation": a text (label, button, tooltip, option, tab title,
## list item) that contains a translation key (`Language.is_key` words) the
## current language has no text for. Static texts hold their key and are
## translated when drawn, so the check looks at the key itself.

const Settings := preload("res://scenes/menu/settings.gd")
const Language := preload("res://scenes/menu/language.gd")
const Missions := preload("res://scenes/menu/missions.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const Subtitles := preload("res://scenes/hud/subtitles.gd")

var flow: Node


func _init(flow_test: Node) -> void:
	flow = flow_test


## Texts under `root` with a key that has no translation in the current
## language ("path: text").
static func untranslated(root: Node) -> PackedStringArray:
	var found: PackedStringArray = []
	var key_word := RegEx.create_from_string("[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+")
	var stack: Array[Node] = [root]
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		stack.append_array(node.get_children(true))
		for text in _texts(node):
			for m in key_word.search_all(text):
				var key := m.get_string()
				if not Language.has(key):
					found.append("%s: %s" % [root.get_path_to(node), text])
					break
	return found


static func _texts(node: Node) -> PackedStringArray:
	var texts: PackedStringArray = []
	if node is Control:
		texts.append((node as Control).tooltip_text)
	if node is Label or node is Button or node is LineEdit or node is RichTextLabel:
		texts.append(str(node.get("placeholder_text")) if node is LineEdit else str(node.get("text")))
	if node is OptionButton:
		var option := node as OptionButton
		for i in option.item_count:
			texts.append(option.get_item_text(i))
	if node is ItemList:
		var list := node as ItemList
		for i in list.item_count:
			texts.append(list.get_item_text(i))
	if node is TabContainer:
		var tabs := node as TabContainer
		for i in tabs.get_tab_count():
			texts.append(tabs.get_tab_title(i))
	return texts


func _check_clean(root: Node, where: String) -> void:
	var found := untranslated(root)
	flow._check(found.is_empty(), "%s: %d text(s) without %s translation, first: %s" % [
		where, found.size(), Language.current, found[0] if not found.is_empty() else ""])


func menu_test(menu: Control) -> void:
	# The scan itself finds a key without a text.
	var probe := Label.new()
	probe.text = "Probe HUD_NO_SUCH_KEY"  # l10n-ignore (self-test probe)
	flow._check(untranslated(probe).size() == 1, "the missing translation scan misses a key")
	probe.free()
	Settings.reset()
	menu.settings_button.pressed.emit()
	await flow._frames(2)
	var panel: PanelContainer = menu.settings_panel
	panel.language_option.select(1 + Language.LANGUAGES.keys().find("de"))
	panel.language_option.item_selected.emit(1 + Language.LANGUAGES.keys().find("de"))
	await flow._frames(2)
	flow._check(TranslationServer.get_locale() == "de", "language not switched to German: %s" % TranslationServer.get_locale())
	flow._check(panel.tr("COMMON_SETTINGS") == "Einstellungen", "no German settings title")
	for screen: String in menu.screens:
		menu.show_screen(screen)
		await flow._frames(2)
		_check_clean(menu, "menu screen '%s' in German" % screen)
	menu.settings_button.pressed.emit()
	await flow._frames(2)
	_check_clean(panel, "settings in German")
	# Names from the catalog and the mission texts follow the language.
	var card: Dictionary = SimHost.catalog_card("de_pz4_h")
	flow._check(card.get("name", "") == "Panzerkampfwagen IV Ausf. H", "catalog name in German: %s" % card.get("name", ""))
	var tutorial: Dictionary = Missions.tutorial()
	flow._check(Missions.text(tutorial.get("path", ""), tutorial.get("title", "")) == "Grundausbildung",
		"tutorial title in German: %s" % Missions.text(tutorial.get("path", ""), tutorial.get("title", "")))
	# Every mission text has a German translation.
	for m in Missions.list():
		for text in SimHost.mission_texts(m["path"]):
			if Missions.text(m["path"], text) == text:
				flow._check(false, "mission %s: no German text for '%s'" % [m["id"], text])
				break

	# Every setting of prompt 043, saved and read back as after a restart.
	panel.options["color_palette"].item_selected.emit(1)
	panel.options["font_size"].item_selected.emit(2)
	panel.options["upscaler"].item_selected.emit(2)
	panel.ui_scale_slider.value = 1.25
	panel.ui_scale_slider.drag_ended.emit(true)
	panel.render_scale_slider.value = 0.75
	panel.subtitles_box.button_pressed = true
	panel.shake_box.button_pressed = false
	panel.vsync_box.button_pressed = false
	panel.fps_option.item_selected.emit(Settings.FPS_LIMITS.find(60))
	panel.resolution_option.item_selected.emit(1 + Settings.RESOLUTIONS.find("1600x900"))
	panel.options["window_mode"].item_selected.emit(0)
	panel.capture("air_recon")
	_push_key(KEY_7, true)
	await flow._frames(2)
	flow._check(RelationColors.palette == "red_green", "palette not applied")
	flow._check(is_equal_approx(MenuTheme.font_scale, 1.15), "font size not applied")
	flow._check(is_equal_approx(menu.get_tree().root.content_scale_factor, 1.25), "UI scale not applied")
	flow._check(is_equal_approx(menu.get_tree().root.scaling_3d_scale, 0.75), "render scale not applied")
	flow._check(Engine.max_fps == 60, "frame limit not applied")
	panel.save_button.pressed.emit()
	await flow._frames(2)
	Settings.reset()
	flow._check(TranslationServer.get_locale() == "en", "defaults do not bring English back in the self-test")
	Settings.load_file()
	var expected := {
		"language": "de", "color_palette": "red_green", "font_size": 1.15, "upscaler": "fsr2", "ui_scale": 1.25,
		"render_scale": 0.75, "subtitles": true, "screen_shake": false, "vsync": false, "fps_limit": 60,
		"resolution": "1600x900", "window_mode": "windowed",
	}
	for key: String in expected:
		var value: Variant = Settings.get_value(key)
		var same: bool = is_equal_approx(value, expected[key]) if value is float else value == expected[key]
		flow._check(same, "setting %s read back as %s" % [key, value])
	flow._check(TranslationServer.get_locale() == "de", "saved language not applied after loading")
	flow._check(Settings.bindings.get("air_recon", 0) == (KEY_7 | KEY_MASK_SHIFT), "Shift+7 for the recon flight not kept")
	var event := InputEventKey.new()
	event.physical_keycode = KEY_7
	event.shift_pressed = true
	flow._check(InputMap.event_is_action(event, "air_recon", true), "Shift+7 does not call the recon flight")
	# Back to the defaults for the rest of the test.
	Settings.reset()
	Settings.save_file()
	flow._check(TranslationServer.get_locale() == "en" and RelationColors.palette == "standard" \
		and is_equal_approx(menu.get_tree().root.content_scale_factor, 1.0), "defaults not restored")
	menu.show_screen("main")
	await flow._frames(2)
	print("flow self-test: language and accessibility ok (menus in German, %d settings read back)" % expected.size())


## In a running match: the HUD and the pause menu in German.
func match_test(main: Node3D) -> void:
	Settings.set_value("language", "de")
	Settings.set_value("subtitles", true)
	await flow._frames(3)
	_check_clean(main.get_node("Hud"), "match HUD in German")
	main.game_menu.open_pause()
	await flow._frames(2)
	_check_clean(main.game_menu, "pause menu in German")
	main.game_menu.close_pause()
	await flow._frames(2)
	var before: int = main.hud.subtitles.shown
	Subtitles.notice("ui_under_attack")
	await flow._frames(1)
	flow._check(main.hud.subtitles.shown == before + 1, "no subtitle for a warning")
	var line: Label = main.hud.subtitles.get_child(main.hud.subtitles.get_child_count() - 1)
	flow._check(line != null and line.text == "Wir werden angegriffen!", "subtitle '%s'" % (line.text if line else ""))
	Settings.set_value("subtitles", false)
	Settings.set_value("language", "auto")
	await flow._frames(2)
	flow._check(TranslationServer.get_locale() == "en", "language not back to English")
	print("flow self-test: match HUD in German ok")


func _push_key(keycode: Key, shift: bool) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.pressed = pressed
		event.physical_keycode = keycode
		event.keycode = keycode
		event.shift_pressed = shift
		flow.get_tree().root.push_input(event)
