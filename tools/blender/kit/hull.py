"""Hulls and superstructures from side profiles with sloped plates.

Profiles are drawn in meters as lists of `[y, z]` points (side view, front +Y).
The hull part scales the profiles along Y so that the hull length matches the
catalog (`hull_length` in the context); the blueprint therefore only fixes
proportions and plate angles, never the main dimensions.
"""

import math

from .registry import part


def _scale_profile(profile, sy, cy):
    return [((y - cy) * sy, z) for y, z in profile]


def profile_z_at(profile, y):
    """Highest z of a closed profile at position y (roof height)."""
    best = None
    n = len(profile)
    for i in range(n):
        (y0, z0), (y1, z1) = profile[i], profile[(i + 1) % n]
        if min(y0, y1) - 1e-6 <= y <= max(y0, y1) + 1e-6:
            z = z0 if abs(y1 - y0) < 1e-9 else z0 + (z1 - z0) * (y - y0) / (y1 - y0)
            z = max(z, z0, z1) if abs(y1 - y0) < 1e-9 else z
            best = z if best is None else max(best, z)
    return best


@part("plate_hull")
def plate_hull(model, p, ctx):
    """Welded hull: lower hull between the tracks and a superstructure.

    Parameters:
    - `lower`: side profile of the lower hull (between the tracks).
    - `upper`: side profile of the superstructure (over the tracks).
    - `upper_half_width`: "full" (out to the fenders, default) or meters.
    - `upper_side_slope_deg`: inward slope of the superstructure side plates
      from vertical (T-34: 40).
    - `fender_z`: height of the fenders (default: track top + 0.02),
      `fender_thickness`, `fender_front`, `fender_rear` (Y extent, default:
      superstructure extent), `fender_slope_front` (m the front fender drops).
    - `bevel`: edge bevel of the plates in meters (worn edges).
    - `cast`: rounded cast look (more bevel segments, smooth shading).
    Returns `hull_top_z`, `hull_front_y`, `hull_rear_y`, `hull_upper`
    (scaled upper profile), `hull_half_width`.
    """
    lower = [tuple(v) for v in p.req("lower")]
    upper = [tuple(v) for v in p.req("upper")]
    bevel = p.get("bevel", 0.015)
    cast = p.get("cast", False)
    ft = p.get("fender_thickness", 0.012)

    ys = [y for y, _ in lower + upper]
    raw_len = max(ys) - min(ys)
    target = ctx["hull_length"]
    sy = target / raw_len
    if abs(sy - 1.0) > 0.08:
        raise ValueError(f"plate_hull: profiles are {raw_len:.2f} m long but the catalog hull is "
                         f"{target:.2f} m; fix the profile (scale {sy:.3f})")
    cy = (max(ys) + min(ys)) / 2
    lower = _scale_profile(lower, sy, cy)
    upper = _scale_profile(upper, sy, cy)

    hw_lower = ctx["track_inner_x"] - p.get("track_gap", 0.03)
    full = ctx["half_width"]
    uhw = p.get("upper_half_width", "full")
    hw_upper = full if uhw == "full" else float(uhw)
    slope = math.tan(math.radians(p.get("upper_side_slope_deg", 0.0)))
    z0 = min(z for _, z in upper)

    hull = model.mb("Hull")
    if cast:
        hull.bevel_segments = 3
        bevel = max(bevel, 0.05)
    hull.extrude_x(lower, -hw_lower, hw_lower, "Paint", bevel=bevel)
    if hw_upper > hw_lower + 0.05 or slope:
        hull.extrude_x(
            upper,
            lambda y, z: -(hw_upper - (z - z0) * slope),
            lambda y, z: hw_upper - (z - z0) * slope,
            "Paint",
            bevel=bevel,
            smooth=cast,
        )
    else:
        hull.extrude_x(upper, -hw_upper, hw_upper, "Paint", bevel=bevel, smooth=cast)
    hull.bevel_segments = 1

    # fenders over the tracks
    fz = p.get("fender_z", ctx["track_top_z"] + 0.02)
    uys = [y for y, _ in upper]
    f_front = p.get("fender_front", max(uys))
    f_rear = p.get("fender_rear", min(uys))
    drop = p.get("fender_slope_front", 0.0)
    inner = hw_lower
    for side in (-1, 1):
        xa, xb = side * inner, side * full
        x0, x1 = min(xa, xb), max(xa, xb)
        hull.extrude_x([(f_rear, fz), (f_front - 0.35 if drop else f_front, fz), (f_front, fz - drop),
                        (f_front, fz - drop - ft), (f_front - 0.35 if drop else f_front, fz - ft), (f_rear, fz - ft)],
                       x0, x1, "Paint")

    top = max(z for _, z in upper + lower)
    return {
        "hull_top_z": top,
        "hull_front_y": max(y for y, _ in lower + upper),
        "hull_rear_y": min(y for y, _ in lower + upper),
        "hull_upper": upper,
        "hull_half_width": hw_upper,
        "fender_z": fz,
    }


@part("deck_box")
def deck_box(model, p, ctx):
    """Box on the hull (engine deck grilles, hatches, stowage bins).

    Parameters: `center` [x, y] or [x, y, z] (z of the bottom face, default
    hull top), `size` [x, y, z], `material` (default Paint), `bevel`,
    `parent` (default Hull), `top_inset` [x, y] (optional: the top face is
    this much smaller on each side, giving sloped walls, e.g. the turret
    platform of the T-26S Model 1939; `bevel` is then ignored).
    """
    c = p.req("center")
    cx, cy = c[0], c[1]
    cz = c[2] if len(c) > 2 else ctx["hull_top_z"]
    sx, sy, sz = p.req("size")
    mb = model.mb(p.get("parent", "Hull"))
    inset = p.get("top_inset")
    if inset:
        def ring(hx, hy, z):
            return [(cx - hx, cy + hy, z), (cx + hx, cy + hy, z), (cx + hx, cy - hy, z), (cx - hx, cy - hy, z)]
        mb.loft([ring(sx / 2, sy / 2, cz), ring(sx / 2 - inset[0], sy / 2 - inset[1], cz + sz)],
                p.get("material", "Paint"))
        p.get("bevel")
        return
    mb.box((cx, cy, cz + sz / 2), (sx, sy, sz), p.get("material", "Paint"), bevel=p.get("bevel", 0.0))


@part("hull_hatch")
def hull_hatch(model, p, ctx):
    """Hinged hull hatch (driver, radio operator) as its own object.

    Parameters: `center` [x, y] on the roof, `size` [x, y], `z` (default hull
    top; "casemate" = the casemate roof of a `casemate` vehicle), `hinge`
    "front" / "rear" / "left" / "right" (default rear), `socket` (adds
    Socket_Hatch_NN at the hatch).
    """
    x, y = p.req("center")
    sx, sy = p.req("size")
    z = p.get("z", ctx["hull_top_z"])
    if z == "casemate":
        z = ctx["casemate_top_z"]
    hinge = p.get("hinge", "rear")
    hp = {"rear": (x, y - sy / 2), "front": (x, y + sy / 2), "left": (x - sx / 2, y), "right": (x + sx / 2, y)}[hinge]
    name = model.next_index("Hatch_")
    mb = model.mb(name, parent="Hull", pivot=(hp[0], hp[1], z))
    mb.box((x, y, z + 0.02), (sx, sy, 0.04), "Paint", bevel=0.008)
    mb.box((x, y, z + 0.05), (sx * 0.15, sy * 0.5, 0.02), "Metal")
    if p.get("socket", False):
        model.socket(model.next_index("Socket_Hatch_"), "Hull", (x, y, z))
