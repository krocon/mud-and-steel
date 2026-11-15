"""US faction parts (prefix `us.`)."""

from .registry import part


@part("us.transmission_cover")
def transmission_cover(model, p, ctx):
    """Rounded cast or three-piece transmission cover at the Sherman's nose.

    Parameters: `y` (axis of the rounding), `z`, `radius`, `width` (default
    lower hull width + 0.1), `segments`.
    """
    y, z = p.req("y"), p.req("z")
    r = p.req("radius")
    w = p.get("width", ctx["track_inner_x"] * 2)
    seg = p.get("segments", 12)
    mb = model.mb("Hull")
    mb.cylinder((-w / 2, y, z), (w / 2, y, z), r, seg=seg, mat="Paint", smooth=True)
    # bolted flange on top
    mb.box((0, y - r * 0.2, z + r * 0.92), (w, 0.12, 0.05), "Paint")


@part("us.hatch_hoods")
def hatch_hoods(model, p, ctx):
    """Driver and assistant driver hatches with periscope hoods on the glacis
    top (M4A3 large-hatch hull). Parameters: `x`, `y`, `size` [x, y]; adds
    two Hatch_NN objects and Socket_Hatch_NN."""
    x, y = p.req("x"), p.req("y")
    sx, sy = p.get("size", [0.55, 0.6])
    z = ctx["hull_top_z"]
    for side in (-1, 1):
        cx = side * x
        name = model.next_index("Hatch_")
        mb = model.mb(name, parent="Hull", pivot=(cx + side * sx / 2, y, z))
        mb.box((cx, y, z + 0.025), (sx, sy, 0.05), "Paint", bevel=0.01)
        mb.box((cx - side * sx * 0.15, y + sy * 0.25, z + 0.09), (0.16, 0.1, 0.08), "Glass")
        model.socket(model.next_index("Socket_Hatch_"), "Hull", (cx, y, z))
