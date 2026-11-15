"""Hash of everything a model is built from (blueprint + catalog entry).

Stored on the root object at build time; `validate.py` compares it to detect
models that were not rebuilt after a blueprint or catalog change. The catalog
`model` field is ignored, since it is set after the first build.
"""

import hashlib
import json

from . import catalog


def source_hash(faction, unit_id):
    u = dict(catalog.unit(faction, unit_id))
    u.pop("model", None)
    h = hashlib.sha256()
    h.update(catalog.blueprint_path(faction, unit_id).read_bytes())
    h.update(json.dumps(u, sort_keys=True, ensure_ascii=False).encode())
    return h.hexdigest()[:16]
