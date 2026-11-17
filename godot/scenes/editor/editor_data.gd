extends RefCounted
## Tools, object presets and choices of the map editor (prompt 028), shared
## by `map_editor.gd` and `editor_panel.gd`.

const SIZES: Array[int] = [256, 500, 1000, 2000, 4000]
const BIOMES: Array[String] = ["forest", "desert", "winter"]

## Tools: id → label (translation key).
const TOOLS: Dictionary[String, String] = {
	"raise": "EDITOR_TOOL_RAISE", "lower": "EDITOR_TOOL_LOWER", "smooth": "EDITOR_TOOL_SMOOTH",
	"flatten": "EDITOR_TOOL_FLATTEN",
	# Prompt 060: mountains.
	"ridge": "EDITOR_TOOL_RIDGE", "valley": "EDITOR_TOOL_VALLEY", "terrace": "EDITOR_TOOL_TERRACE",
	"road": "EDITOR_TOOL_ROAD",
	"paint": "EDITOR_TOOL_PAINT", "river": "EDITOR_TOOL_RIVER", "scatter": "EDITOR_TOOL_SCATTER",
	"place": "EDITOR_TOOL_PLACE", "erase": "EDITOR_TOOL_ERASE", "start0": "EDITOR_TOOL_START0",
	"start1": "EDITOR_TOOL_START1",
	# Prompt 037: up to six players in teams.
	"start2": "EDITOR_TOOL_START2", "start3": "EDITOR_TOOL_START3",
	"start4": "EDITOR_TOOL_START4", "start5": "EDITOR_TOOL_START5",
	# Prompt 052: neutral trench strongpoints (map markers).
	"strongpoint": "EDITOR_TOOL_STRONGPOINT",
	# Prompt 059: compounds (house, walled yard, props) and the village brush.
	"compound": "EDITOR_TOOL_COMPOUND", "village": "EDITOR_TOOL_VILLAGE",
	# Prompt 039: places for mission scripts (mission_points.toml).
	"mission_area": "EDITOR_TOOL_MISSION_AREA", "mission_spawn": "EDITOR_TOOL_MISSION_SPAWN",
}
## Mission tools (prompt 039): a click inside an area or this close (m) to
## a spawn point removes it, elsewhere adds one.
const MISSION_TOOLS: Array[String] = ["mission_area", "mission_spawn"]
const MISSION_PICK_M := 6.0
## A strongpoint click within this distance (m) of one removes it.
const STRONGPOINT_PICK_M := 20.0
## Mountain tools (prompt 060).
const MOUNTAIN_TOOLS: Array[String] = ["ridge", "valley", "terrace", "road"]
## Start tools (prompt 037).
const START_TOOLS: Array[String] = ["start0", "start1", "start2", "start3", "start4", "start5"]
## Team choices of a start (translation keys): -1 none, then teams 1 to 3.
const START_TEAMS: Array[String] = ["EDITOR_START_NO_TEAM", "EDITOR_START_TEAM_1", "EDITOR_START_TEAM_2", "EDITOR_START_TEAM_3"]
## Object brush presets: kinds ("id:variant") and spacing in meters. The
## names are ids (`map_editor.gd` scatter_preset); SCATTER_LABELS shows them.
## Optional (prompt 074): `density` scales the panel's density for the
## kinds; `then` is a second scatter in the same dab (kinds, spacing,
## density), e.g. trees between the pieces of a rock ridge (the brush
## places nothing inside a rock).
const SCATTER_PRESETS: Dictionary[String, Dictionary] = {
	"Deciduous wood": {"kinds": ["env_tree_deciduous:Oak", "env_tree_deciduous:Beech", "env_tree_deciduous:OakYoung"], "spacing": 5.0},  # l10n-ignore
	"Conifer wood": {"kinds": ["env_tree_conifer:Spruce", "env_tree_conifer:Pine", "env_tree_conifer:SpruceYoung"], "spacing": 4.5},  # l10n-ignore
	"Birch grove": {"kinds": ["env_tree_birch:Birch", "env_tree_birch:BirchYoung"], "spacing": 4.0},  # l10n-ignore
	"Palms": {"kinds": ["env_tree_palm:Palm", "env_tree_palm:PalmShort", "env_tree_palm:PalmLeaning"], "spacing": 6.0},  # l10n-ignore
	"Bushes": {"kinds": ["env_bush:Hazel", "env_bush:Round", "env_bush:Wide"], "spacing": 3.0},  # l10n-ignore
	"Rocks": {"kinds": ["env_rock:Medium", "env_rock:Large"], "spacing": 4.0},  # l10n-ignore
	"Hay": {"kinds": ["env_hay_bale:Rick", "env_hay_bale:Round"], "spacing": 6.0},  # l10n-ignore
	"Rock slabs": {"kinds": ["env_rock_slab:Field", "env_rock_slab:Long", "env_rock_slab:Broken",  # l10n-ignore
		"env_rock_slab:Small", "env_rock_slab:Wide", "env_rock_slab:Strip", "env_rock_slab:Small"],  # l10n-ignore
		"spacing": 7.0, "density": 0.6},
	"Rock outcrops": {"kinds": ["env_rock_outcrop:Group", "env_rock_outcrop:Block", "env_rock_outcrop:Split",  # l10n-ignore
		"env_rock_outcrop:Pair", "env_rock_outcrop:Leaning", "env_rock_outcrop:Layered"],  # l10n-ignore
		"spacing": 14.0, "density": 0.25,
		"then": {"kinds": ["env_bush:Hazel", "env_bush:Round", "env_rock:Medium", "env_rock:Large"],  # l10n-ignore
			"spacing": 4.0, "density": 0.6}},
	"Rock ridge": {"kinds": ["env_rock_massif:Ridge", "env_rock_massif:Long", "env_rock_massif:Short",  # l10n-ignore
		"env_rock_massif:High", "env_rock_massif:Double"],  # l10n-ignore
		"spacing": 9.0, "density": 0.6,
		"then": {"kinds": ["env_tree_conifer:Pine", "env_tree_conifer:Spruce", "env_tree_conifer:SpruceYoung",  # l10n-ignore
			"env_bush:Hazel", "env_bush:Wide"], "spacing": 4.5, "density": 1.0}},  # l10n-ignore
}
## Labels (translation keys) of the object brush presets, by id.
const SCATTER_LABELS: Dictionary[String, String] = {
	"Deciduous wood": "EDITOR_PRESET_DECIDUOUS",  # l10n-ignore
	"Conifer wood": "EDITOR_PRESET_CONIFER",  # l10n-ignore
	"Birch grove": "EDITOR_PRESET_BIRCH",  # l10n-ignore
	"Palms": "EDITOR_PRESET_PALMS",  # l10n-ignore
	"Bushes": "EDITOR_PRESET_BUSHES",  # l10n-ignore
	"Rocks": "EDITOR_PRESET_ROCKS",  # l10n-ignore
	"Hay": "EDITOR_PRESET_HAY",  # l10n-ignore
	"Rock slabs": "EDITOR_PRESET_ROCK_SLABS",  # l10n-ignore
	"Rock outcrops": "EDITOR_PRESET_ROCK_OUTCROPS",  # l10n-ignore
	"Rock ridge": "EDITOR_PRESET_ROCK_RIDGE",  # l10n-ignore
}
## Groups of the "Place object" list (prompt 041): group name (translation
## key) -> catalog id prefixes or ids, in this order; kinds no group names
## come last (OTHER_GROUP).
const OBJECT_GROUPS: Array = [
	["EDITOR_GROUP_TREES", ["env_tree", "env_bush", "env_log", "env_grass"]],
	["EDITOR_GROUP_VILLAGE", ["env_house_farm", "env_house_stone", "env_house_mud", "env_house_log", "env_barn", "env_church"]],
	["EDITOR_GROUP_TOWN", ["env_house_town", "env_town_hall", "env_warehouse", "env_block_town"]],
	["EDITOR_GROUP_DESERT", ["env_house_desert", "env_block_desert", "env_mosque", "env_police_post", "env_wall_mud",
		"env_awning", "env_jars", "env_well_desert", "env_cart_wood", "env_market_stall"]],
	["EDITOR_GROUP_DUMPS", ["env_dump"]],
	["EDITOR_GROUP_FENCES", ["env_fence", "env_wall", "env_hedge"]],
	["EDITOR_GROUP_BRIDGES", ["env_bridge"]],
	["EDITOR_GROUP_PROPS", ["env_hay", "env_woodpile", "env_crate", "env_barrel", "env_pole", "env_rock"]],
	["EDITOR_GROUP_FORTIFICATIONS", ["fort_"]],
]
const OTHER_GROUP := "EDITOR_GROUP_OTHER"


## The group of a placeable kind ("id:variant"), or OTHER_GROUP.
static func object_group(kind: String) -> String:
	var id := kind.get_slice(":", 0)
	for group: Array in OBJECT_GROUPS:
		for prefix: String in group[1]:
			if id.begins_with(prefix):
				return group[0]
	return OTHER_GROUP
