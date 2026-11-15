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
