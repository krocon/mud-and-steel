"""Slot materials for soldiers. Colors come from the shared palette file
godot/assets/models/infantry/uniform_palettes.json, which Godot reads as well.

The exported model carries the colors of the biome it was built for; in the game the
biome is switched by material overrides (godot/scenes/tools/soldier_look.gd).
"""

import json
from pathlib import Path

import bpy

REPO_ROOT = Path(__file__).resolve().parents[3]
PALETTE_FILE = REPO_ROOT / "godot" / "assets" / "models" / "infantry" / "uniform_palettes.json"

SLOTS = ["Cloth", "Canvas", "Leather", "Paint", "Skin", "Metal", "Wood", "Rubber", "Glass",
         "Insignia"]
BIOMES = ["forest", "desert", "winter"]

ROUGHNESS = {"Metal": 0.45, "Glass": 0.1, "Paint": 0.7, "Leather": 0.6}


def load_palette():
    return json.loads(PALETTE_FILE.read_text())


def srgb_to_linear(hex_color):
    hex_color = hex_color.lstrip("#")
    out = []
    for i in (0, 2, 4):
        c = int(hex_color[i:i + 2], 16) / 255.0
        out.append(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4)
    return tuple(out)


def slot_colors(faction, biome):
    palette = load_palette()
    colors = dict(palette["fixed"])
    colors.update(palette["factions"][faction][biome])
    return colors


def make_materials(faction, biome):
    """Creates (or updates) one material per slot. Vertex color multiplies the base color."""
    colors = slot_colors(faction, biome)
    materials = {}
    for slot in SLOTS:
        mat = bpy.data.materials.get(slot) or bpy.data.materials.new(slot)
        rgb = srgb_to_linear(colors[slot])
        mat.diffuse_color = (*rgb, 1.0)
        mat.use_nodes = True
        nodes = mat.node_tree.nodes
        bsdf = nodes.get("Principled BSDF")
        bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
        bsdf.inputs["Roughness"].default_value = ROUGHNESS.get(slot, 0.9)
        bsdf.inputs["Metallic"].default_value = 0.6 if slot == "Metal" else 0.0
        if "Col" not in [n.name for n in nodes]:
            attr = nodes.new("ShaderNodeVertexColor")
            attr.name = "Col"
            attr.layer_name = "Col"
            mix = nodes.new("ShaderNodeMix")
            mix.name = "Tint"
            mix.data_type = "RGBA"
            mix.blend_type = "MULTIPLY"
            mix.inputs["Factor"].default_value = 1.0
            mix.inputs[6].default_value = (*rgb, 1.0)
            links = mat.node_tree.links
            links.new(attr.outputs["Color"], mix.inputs[7])
            links.new(mix.outputs[2], bsdf.inputs["Base Color"])
        else:
            nodes["Tint"].inputs[6].default_value = (*rgb, 1.0)
        materials[slot] = mat
    return materials
