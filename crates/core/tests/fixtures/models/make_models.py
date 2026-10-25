"""Writes the small test models in this folder. Run it from anywhere:

    python3 crates/core/tests/fixtures/models/make_models.py

gable-house-mm.obj is a house 8 m wide, 10 m deep, 6 m to the eaves and 9 m to
the ridge, in millimetres with z up, as Archicad's OBJ is assumed to be.
l-block-y-up.obj is an L-shaped block, 30 by 30 m and 12 m tall, in metres with
y up, its roof and floor in two quads each. gable-house.glb is the gable house
again in glTF's metres with y up, as a child node moved by its parent.
"""

import json
import struct
from pathlib import Path

HERE = Path(__file__).parent

# The gable house in metres, z up.
HOUSE_CORNERS = [
    (0, 0, 0), (8, 0, 0), (8, 10, 0), (0, 10, 0),
    (0, 0, 6), (8, 0, 6), (8, 10, 6), (0, 10, 6),
    (4, 0, 9), (4, 10, 9),
]
# Counterclockwise seen from outside, so every normal points out.
HOUSE_FACES = [
    [0, 3, 2, 1],        # floor
    [0, 1, 5, 8, 4],     # south gable
    [2, 3, 7, 9, 6],     # north gable
    [1, 2, 6, 5],        # east wall
    [3, 0, 4, 7],        # west wall
    [5, 6, 9, 8],        # east roof
    [4, 8, 9, 7],        # west roof
]

L_PLAN = [(0, 0), (30, 0), (30, 10), (10, 10), (10, 30), (0, 30)]
L_HEIGHT = 12


def l_block():
    corners = [(x, y, 0) for x, y in L_PLAN] + [(x, y, L_HEIGHT) for x, y in L_PLAN]
    top = len(L_PLAN)
    faces = [
        # Floor and roof in two convex quads each, the way exporters split them.
        [0, 3, 2, 1], [0, 5, 4, 3],
        [top + 0, top + 1, top + 2, top + 3], [top + 0, top + 3, top + 4, top + 5],
    ]
    for i in range(len(L_PLAN)):
        j = (i + 1) % len(L_PLAN)
        faces.append([i, j, top + j, top + i])
    return corners, faces


def write_obj(path, corners, faces, groups=None):
    lines = ["# Test model for openarchi, see make_models.py"]
    lines += [f"v {x:g} {y:g} {z:g}" for x, y, z in corners]
    for index, face in enumerate(faces):
        if groups and index in groups:
            lines.append(f"g {groups[index]}")
        lines.append("f " + " ".join(str(i + 1) for i in face))
    path.write_text("\n".join(lines) + "\n")


def y_up(corners):
    """z-up metres to glTF's y up: x stays, y is up, z points south."""
    return [(x, z, -y) for x, y, z in corners]


def triangles(faces):
    return [(face[0], face[i], face[i + 1]) for face in faces for i in range(1, len(face) - 1)]


def write_glb(path, corners, faces):
    positions = b"".join(struct.pack("<3f", *corner) for corner in corners)
    indices = b"".join(struct.pack("<3I", *t) for t in triangles(faces))
    binary = positions + indices
    lows = [min(c[i] for c in corners) for i in range(3)]
    highs = [max(c[i] for c in corners) for i in range(3)]
    document = {
        "asset": {"version": "2.0", "generator": "openarchi make_models.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"translation": [50, 0, -20], "children": [1]}, {"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
        "buffers": [{"byteLength": len(binary)}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": len(positions)},
            {"buffer": 0, "byteOffset": len(positions), "byteLength": len(indices)},
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": len(corners), "type": "VEC3", "min": lows, "max": highs},
            {"bufferView": 1, "componentType": 5125, "count": len(indices) // 4, "type": "SCALAR"},
        ],
    }
    text = json.dumps(document).encode()
    text += b" " * (-len(text) % 4)
    binary += b"\0" * (-len(binary) % 4)
    chunks = struct.pack("<II", len(text), 0x4E4F534A) + text + struct.pack("<II", len(binary), 0x004E4942) + binary
    path.write_bytes(struct.pack("<III", 0x46546C67, 2, 12 + len(chunks)) + chunks)


millimetres = [(x * 1000, y * 1000, z * 1000) for x, y, z in HOUSE_CORNERS]
write_obj(HERE / "gable-house-mm.obj", millimetres, HOUSE_FACES, {0: "slab", 1: "walls", 5: "roof"})
l_corners, l_faces = l_block()
write_obj(HERE / "l-block-y-up.obj", y_up(l_corners), l_faces)
write_glb(HERE / "gable-house.glb", y_up(HOUSE_CORNERS), HOUSE_FACES)
