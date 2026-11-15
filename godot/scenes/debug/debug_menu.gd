extends PanelContainer
## Debug menu (F3): frame rate, draw calls, terrain chunks per LOD level and
## test commands. Presentation only; nothing here reaches the simulation.
##
## Commands:
## - "Test crater at focus": presses a presentation-only crater into the
##   displayed terrain at the camera focus and rebuilds the affected chunks
##   (the code path prompt 021 will use for real craters).
## - "Show terrain LOD": tints every chunk by its LOD level.
## - "Map objects" / "Decoration": show or hide them (prompt 013).
## - "Water": show or hide the water surface (prompt 014).
## - Biome and graphics quality switches (prompt 015).
## - "Fog of war": off reveals every unit and the whole map (prompt 018,
##   presentation only).

var main: Node3D

var _stats: Label


func _ready() -> void:
	visible = false
	position = Vector2(12, 12)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	add_child(box)
	var title := Label.new()
	title.text = "Debug (F3)"
	box.add_child(title)
	_stats = Label.new()
	box.add_child(_stats)
	var crater := Button.new()
	crater.text = "Test crater at focus"
	crater.pressed.connect(_on_crater)
	box.add_child(crater)
	var lod := CheckBox.new()
	lod.text = "Show terrain LOD"
	lod.toggled.connect(func(on: bool) -> void: main.terrain.set_show_lod(on))
	box.add_child(lod)
	var objects := CheckBox.new()
	objects.text = "Map objects"
	objects.button_pressed = true
	objects.toggled.connect(func(on: bool) -> void: main.map_objects.set_objects_visible(on))
	box.add_child(objects)
	var deco := CheckBox.new()
	deco.text = "Decoration"
	deco.button_pressed = true
	deco.toggled.connect(func(on: bool) -> void: main.map_objects.set_decoration_visible(on))
	box.add_child(deco)
	var water := CheckBox.new()
	water.text = "Water"
	water.button_pressed = true
	water.toggled.connect(func(on: bool) -> void: main.water.set_water_visible(on))
	box.add_child(water)
	var fog := CheckBox.new()
	fog.text = "Fog of war"
	fog.button_pressed = true
	fog.toggled.connect(func(on: bool) -> void: main.fog_of_war.set_fog_enabled(on))
	box.add_child(fog)
	# Biome and graphics quality (prompt 015), presentation only.
	box.add_child(_choice("Biome", main.atmosphere.BIOMES, main.atmosphere.preset.id,
		func(id: String) -> void: main.set_biome(id)))
	box.add_child(_choice("Quality", main.GraphicsQuality.LEVELS, main.atmosphere.quality,
		func(level: String) -> void: main.atmosphere.set_quality(level)))


func _unhandled_key_input(event: InputEvent) -> void:
	var key := event as InputEventKey
	if key and key.pressed and not key.echo and key.keycode == KEY_F3:
		visible = not visible
		get_viewport().set_input_as_handled()


func _process(_delta: float) -> void:
	if not visible:
		return
	var draw_calls := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
	var primitives := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
	_stats.text = "%d FPS\n%d draw calls, %.1f M triangles\nterrain chunks per LOD %s\ncamera %.0f m" % [
		Engine.get_frames_per_second(), draw_calls, primitives / 1e6,
		main.terrain.visible_per_lod, main.camera_rig.distance,
	]
	_stats.text += "\nobjects %d, decoration %d in %d squares" % [
		main.map_objects.object_instances, main.map_objects.deco_instances, main.map_objects.deco_chunks,
	]


func _on_crater() -> void:
	main.terrain.debug_crater(main.camera_rig.focus)


## A labeled option button; `on_select` gets the chosen item.
func _choice(label: String, items: Array[String], current: String, on_select: Callable) -> HBoxContainer:
	var row := HBoxContainer.new()
	var text := Label.new()
	text.text = label
	row.add_child(text)
	var options := OptionButton.new()
	for item in items:
		options.add_item(item)
	options.select(maxi(items.find(current), 0))
	options.item_selected.connect(func(index: int) -> void: on_select.call(items[index]))
	row.add_child(options)
	return row
