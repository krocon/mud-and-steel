extends RefCounted
## Self-test of the map picker and the map previews (prompt 201), run by
## `menu/flow_self_test.gd`:
##
## 1. Every offered map gets a preview picture (built cold, then read back
##    from the disk cache): not empty, not one flat color, with a relief
##    grid; every start layout marks its starts on the map (at least two,
##    inside the map); maps without variants have just the standard one.
##    Prints the times per map and the cache size.
## 2. The picker opens from the single-player setup with a row per map;
##    the search filters, a click on a column title sorts; the details
##    show the marked map and its 3D relief builds; Enter on a row chooses
##    it (the setup's map and preview follow, with the starts of the
##    chosen layout); Esc closes it without a change.

const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MapPreview := preload("res://scenes/menu/map_preview.gd")
const MapPicker := preload("res://scenes/menu/map_picker.gd")


func run(flow: Node, menu: Control) -> void:
	_previews_test(flow)
	await _picker_test(flow, menu)


func _previews_test(flow: Node) -> void:
	var maps := MatchSetup.available_maps()
	var lines: PackedStringArray = []
	for map_name in maps:
		MapPreview.invalidate(map_name)
		var started := Time.get_ticks_usec()
		var info := MapPreview.info(map_name)
		var info_ms := (Time.get_ticks_usec() - started) / 1000.0
		var entry := MapPreview.build_now(map_name)
		flow._check(not entry.is_empty() and not entry.has("error"), "map preview of %s failed: %s" % [map_name, entry.get("error", "nothing")])
		if entry.is_empty() or entry.has("error"):
			continue
		var texture: Texture2D = entry["texture"]
		var image := texture.get_image()
		flow._check(image != null and image.get_width() >= 50 and image.get_height() >= 50, "map preview of %s is %s" % [map_name, image.get_size() if image != null else "missing"])
		if image != null:
			var first := image.get_pixel(0, 0)
			var varied := false
			for k in 64:
				var p := image.get_pixel(k * 7919 % image.get_width(), k * 104729 % image.get_height())
				if not p.is_equal_approx(first):
					varied = true
			flow._check(varied, "map preview of %s is one flat color" % map_name)
		flow._check(int(entry["relief_columns"]) >= 2 and (entry["relief"] as PackedFloat32Array).size() == int(entry["relief_columns"]) * int(entry["relief_rows"]), "relief of %s is empty" % map_name)
		var layouts: Array = info.get("layouts", [])
		var variants := MatchSetup.variant_choices(map_name)
		var expected := 1 if variants.size() == 1 else variants.size() - 1
		flow._check(layouts.size() == expected, "%s: %d layouts in the preview, %d start layouts in the setup" % [map_name, layouts.size(), expected])
		var size_m: Vector2 = info.get("size_m", Vector2.ZERO)
		for l: Dictionary in layouts:
			var starts: Array = l["starts"]
			flow._check(starts.size() >= 2, "%s, layout %s: %d starts marked" % [map_name, l["name"], starts.size()])
			for s: Dictionary in starts:
				var p: Vector2 = s["position"]
				flow._check(absf(p.x) <= size_m.x * 0.5 and absf(p.y) <= size_m.y * 0.5, "%s: start %d outside the map" % [map_name, s["player"]])
		# Warm: read back from the disk cache.
		MapPreview._loaded.erase(map_name)
		flow._check(not MapPreview.cached(map_name).is_empty(), "map preview of %s not read back from the cache" % map_name)
		var t: Dictionary = MapPreview.timings.get(map_name, {})
		lines.append("%s %dx%d build %.0f ms, cold %.0f ms, warm %.1f ms, info %.1f ms" % [
			map_name, image.get_width(), image.get_height(), t.get("build_ms", 0.0), t.get("cold_ms", 0.0), t.get("warm_ms", 0.0), info_ms])
	print("map picker self-test: previews\n  " + "\n  ".join(lines))
	print("map picker self-test: cache %.1f MB for %d maps" % [MapPreview.cache_bytes() / 1048576.0, maps.size()])


func _picker_test(flow: Node, menu: Control) -> void:
	menu.single_player_button.pressed.emit()
	await flow._frames(2)
	var picker: Control = menu.open_map_picker()
	flow._check(picker != null, "the setup's map button opens no picker")
	if picker == null:
		return
	await flow._frames(2)
	var maps := MatchSetup.available_maps()
	flow._check(picker.rows.size() == maps.size(), "the picker lists %d of %d maps" % [picker.rows.size(), maps.size()])
	print("map picker self-test: picker opened in %.0f ms with %d maps" % [MapPicker.last_open_ms, picker.rows.size()])
	# Search.
	picker.search.text = "coastal"
	picker.search.text_changed.emit(picker.search.text)
	flow._check(picker.rows.size() == 1 and picker.rows[0]["name"] == "coastal_heights", "search 'coastal' finds %s" % [picker.rows.map(func(r: Dictionary) -> String: return r["name"])])
	picker.search.text = ""
	picker.search.text_changed.emit("")
	# Sorting by size: ascending areas.
	picker.table.column_title_clicked.emit(1, MOUSE_BUTTON_LEFT)
	var areas: Array = picker.rows.map(func(r: Dictionary) -> float: return r["size_m"].x * r["size_m"].y)
	var sorted := areas.duplicate()
	sorted.sort()
	flow._check(areas == sorted, "sorting by size gives %s" % [areas])
	# Details and the 3D relief of a map with variants.
	flow._check(picker.select("twin_meadow"), "twin_meadow not in the picker")
	await flow._frames(2)
	flow._check(picker.title_label.text == MatchSetup.map_title("twin_meadow"), "details show '%s'" % picker.title_label.text)
	flow._check(picker.layout_option.item_count == 2, "twin_meadow shows %d start layouts" % picker.layout_option.item_count)
	picker.set_relief(true)
	for i in 300:
		if picker.preview.is_ready():
			break
		await flow._frames(1)
	await flow._frames(2)
	var relief: Control = picker.preview._relief
	flow._check(relief != null and relief.visible and relief._mesh_instance.mesh != null, "the 3D relief of twin_meadow was not built")
	picker.set_relief(false)
	# Enter chooses the marked row.
	flow._check(picker.select("coastal_heights"), "coastal_heights not in the picker")
	picker.table.grab_focus()
	await flow._frames(1)
	flow._push_key(KEY_ENTER, true)
	await flow._frames(3)
	flow._check(not is_instance_valid(picker), "Enter did not close the picker")
	flow._check(menu.map_button.map_name == "coastal_heights", "Enter chose %s" % menu.map_button.map_name)
	flow._check(menu.map_preview.map_name == "coastal_heights" and menu.map_preview.marked_starts().size() == 4, "the setup's preview shows %s with %d starts" % [menu.map_preview.map_name, menu.map_preview.marked_starts().size()])
	# Esc closes without a change.
	picker = menu.open_map_picker()
	await flow._frames(2)
	picker.select("test_small")
	flow._push_key(KEY_ESCAPE, true)
	await flow._frames(3)
	flow._check(not is_instance_valid(picker), "Esc did not close the picker")
	flow._check(menu.map_button.map_name == "coastal_heights", "Esc changed the map to %s" % menu.map_button.map_name)
	menu.select_map(MatchSetup.DEFAULT_MAP)
	menu.show_screen("main")
	await flow._frames(1)
	print("map picker self-test: picker, search, sorting, 3D relief, Enter and Esc ok")
