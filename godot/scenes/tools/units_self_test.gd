extends RefCounted
## Self-test of the unit reference in the main menu (prompt 066), run by
## `menu/flow_self_test.gd`: the menu entry opens the screen; every faction
## lists its entries (no environment objects, the base buildings under
## "General"); every entry opens the unit card with its name and without a
## missing translation; entries that cannot be built are marked; the card
## shows the 3D model, which turns, and the camouflage choice changes its
## biome; Back returns to the menu.

const L10nSelfTest := preload("res://scenes/menu/l10n_self_test.gd")
const UnitStats := preload("res://scenes/hud/unit_stats.gd")


func run(flow: Node, menu: Control) -> void:
	flow._check(menu.units_button != null and menu.units_button.is_visible_in_tree(), "main menu has no Units button")
	menu.units_button.pressed.emit()
	await flow.get_tree().process_frame
	var screen: Control = menu.units_screen
	flow._check(screen.is_visible_in_tree() and screen.card != null, "the unit reference did not open")
	if screen.card == null:
		return
	var factions: Array[String] = screen.faction_ids()
	var expected: Array[String] = ["de", "su", "us", "neutral"]
	flow._check(factions == expected, "unit reference factions %s" % [factions])
	var buildable := SimHost.buildable_ids()
	var listed := 0
	var marked := 0
	var models := 0
	for faction in factions:
		screen.open_faction(faction)
		await flow.get_tree().process_frame
		var ids: Array = screen.ids_of(faction)
		flow._check(screen.entries.size() == ids.size() and not ids.is_empty(), "%s lists %d of %d entries" % [faction, screen.entries.size(), ids.size()])
		for id: String in ids:
			listed += 1
			flow._check(not id.begins_with("env_"), "the unit reference lists the environment object %s" % id)
			screen.show_unit(id)
			var card: Control = screen.card
			var texts: PackedStringArray = card.texts()
			flow._check(card.type_id == id and texts.has(UnitStats.name_of(id)), "unit reference: card of %s shows %s" % [id, card.type_id])
			flow._check(card.tab_texts("game").size() >= 3 or UnitStats.card(id).size() <= 3,
				"unit reference: card of %s shows no values" % id)
			flow._check(card.rating_bars.ratings == SimHost.unit_stats(id).get("ratings", {}), "unit reference: ratings of %s" % id)
			var mark: bool = card._mark.visible
			flow._check(mark == (not buildable.has(id)), "unit reference: %s marked not buildable: %s" % [id, mark])
			marked += 1 if mark else 0
			models += 1 if card.model_view.has_model else 0
		var found := L10nSelfTest.untranslated(screen)
		flow._check(found.is_empty(), "unit reference (%s): %d text(s) without translation, first: %s" % [
			faction, found.size(), found[0] if not found.is_empty() else ""])
	flow._check(screen.ids_of("neutral").has("base_hq"), "the base buildings are not under General")
	flow._check(listed >= 100 and marked > 0, "unit reference lists %d entries, %d marked not buildable" % [listed, marked])
	flow._check(models >= listed * 3 / 4, "unit reference: only %d of %d entries have a 3D model" % [models, listed])
	# The model turns and takes the camouflage of the chosen biome.
	screen.open_faction("de")
	screen.show_unit("de_pz4_h")
	var view: Control = screen.card.model_view
	flow._check(view.has_model and view.is_visible_in_tree(), "unit reference: no 3D model of de_pz4_h")
	var yaw: float = view.yaw()
	view.turn(40.0, 0.0)
	flow._check(not is_equal_approx(view.yaw(), yaw), "the 3D model did not turn")
	var camo: OptionButton = screen.card._camo
	camo.select(2)
	camo.item_selected.emit(2)
	flow._check(view.biome == "winter" and view.has_model, "camouflage choice left the biome '%s'" % view.biome)
	print("self-test: unit reference ok (%d entries, %d not buildable, %d with a 3D model)" % [listed, marked, models])
	screen.back_button.pressed.emit()
	await flow.get_tree().process_frame
	flow._check(menu.screens["main"].visible and not screen.visible, "Back did not leave the unit reference")
