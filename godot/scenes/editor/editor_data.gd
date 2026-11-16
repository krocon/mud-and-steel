extends RefCounted
## Tools, object presets and choices of the map editor (prompt 028), shared
## by `map_editor.gd` and `editor_panel.gd`.

const SIZES: Array[int] = [256, 500, 1000, 2000, 4000]
const BIOMES: Array[String] = ["forest", "desert", "winter"]

## Tools: id → label.
const TOOLS: Dictionary[String, String] = {
	"raise": "Raise", "lower": "Lower", "smooth": "Smooth", "flatten": "Flatten",
	"paint": "Paint terrain", "river": "River", "scatter": "Vegetation / objects",
	"place": "Place object", "erase": "Erase objects", "start0": "Start player 1",
	"start1": "Start player 2",
	# Prompt 037: up to six players in teams.
	"start2": "Start player 3", "start3": "Start player 4",
	"start4": "Start player 5", "start5": "Start player 6",
}
## Start tools (prompt 037).
const START_TOOLS: Array[String] = ["start0", "start1", "start2", "start3", "start4", "start5"]
## Team choices of a start: -1 none, then teams 1 to 3.
const START_TEAMS: Array[String] = ["No team", "Team 1", "Team 2", "Team 3"]
## Object brush presets: kinds ("id:variant") and spacing in meters.
const SCATTER_PRESETS: Dictionary[String, Dictionary] = {
	"Deciduous wood": {"kinds": ["env_tree_deciduous:Oak", "env_tree_deciduous:Beech", "env_tree_deciduous:OakYoung"], "spacing": 5.0},
	"Conifer wood": {"kinds": ["env_tree_conifer:Spruce", "env_tree_conifer:Pine", "env_tree_conifer:SpruceYoung"], "spacing": 4.5},
	"Birch grove": {"kinds": ["env_tree_birch:Birch", "env_tree_birch:BirchYoung"], "spacing": 4.0},
	"Palms": {"kinds": ["env_tree_palm:Palm", "env_tree_palm:PalmShort", "env_tree_palm:PalmLeaning"], "spacing": 6.0},
	"Bushes": {"kinds": ["env_bush:Hazel", "env_bush:Round", "env_bush:Wide"], "spacing": 3.0},
	"Rocks": {"kinds": ["env_rock:Medium", "env_rock:Large"], "spacing": 4.0},
	"Hay": {"kinds": ["env_hay_bale:Rick", "env_hay_bale:Round"], "spacing": 6.0},
}
