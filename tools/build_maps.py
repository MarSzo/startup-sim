#!/usr/bin/env python3
"""Generates the building maps (client/maps/*.json) from a readable description.

The generated JSON files are the single source of truth read by both the server
and the client; this script only exists to make editing the layout easier.

    python3 tools/build_maps.py            # write client/maps/*.json
    python3 tools/build_maps.py --preview  # print ASCII only
"""
import json
import os
import sys

W, H = 60, 48
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
OUT = os.path.join(ROOT, "client", "maps")

LEGEND = {
    "#": {"type": "wall", "solid": True, "color": "#3b3b4f"},
    ".": {"type": "floor", "solid": False, "color": "#c9c2b0"},
    ",": {"type": "carpet", "solid": False, "color": "#7d8fb3"},
    ":": {"type": "tiles", "solid": False, "color": "#e6e6e6"},
    "_": {"type": "lobby", "solid": False, "color": "#d8c7a0"},
    "=": {"type": "parking", "solid": False, "color": "#6f6f6f"},
    "D": {"type": "door", "solid": False, "color": "#a0522d"},
    "G": {"type": "glass_door", "solid": False, "color": "#8fd3e8"},
    "B": {"type": "card_gate", "solid": False, "color": "#e0b040", "access": "card"},
    "L": {"type": "locked_door", "solid": True, "color": "#6a3a2a", "access": "service"},
    "g": {"type": "garage_gate", "solid": False, "color": "#9a9a9a"},
    "E": {"type": "elevator_door", "solid": False, "color": "#b8c4cc"},
    "e": {"type": "elevator", "solid": False, "color": "#9aa8b0"},
    "S": {"type": "stairs", "solid": False, "color": "#b09070"},
    "T": {"type": "furniture", "solid": True, "color": "#8b6b4a"},
    "X": {"type": "car", "solid": True, "color": "#b03a2e"},
    "v": {"type": "grass", "solid": False, "color": "#5e8c4a"},
    "p": {"type": "sidewalk", "solid": False, "color": "#a8a8a0"},
    "z": {"type": "smoking_area", "solid": False, "color": "#8a7f6a"},
    "F": {"type": "fence", "solid": True, "color": "#4a3a2a"},
    "~": {"type": "void", "solid": True, "color": "#1c1c24"},
}


class Floor:
    def __init__(self, floor, fill, room_fill="-"):
        self.floor = floor
        self.t = [[fill] * W for _ in range(H)]
        self.r = [[room_fill] * W for _ in range(H)]
        self.rooms = {}
        self.links = []
        self.spawns = []

    def room(self, key, rid, name, kind):
        self.rooms[key] = {"id": rid, "name": name, "type": kind}

    def area(self, x0, y0, x1, y1, tile, room=None):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.t[y][x] = tile
                if room is not None:
                    self.r[y][x] = room

    def box(self, x0, y0, x1, y1, tile, room):
        """Room interior x0..x1, y0..y1 surrounded by walls."""
        self.area(x0 - 1, y0 - 1, x1 + 1, y1 + 1, "#", "-")
        self.area(x0, y0, x1, y1, tile, room)

    def put(self, x0, y0, x1, y1, tile):
        self.area(x0, y0, x1, y1, tile)

    def rows(self):
        return ["".join(r) for r in self.t], ["".join(r) for r in self.r]


# Shared building geometry (must line up between floors).
ELEV = (24, 8, 28, 12)       # elevator cabin interior
ELEV_DOOR = (25, 13, 27, 13)
STAIRS = (31, 6, 38, 12)     # stairwell interior
STAIRS_FLIGHT = (32, 6, 37, 7)
STAIRS_DOOR = (34, 13, 35, 13)
STAIRS_ARRIVAL = (34, 10)    # tile where you appear after taking the stairs


def building_shell(f, inside_fill):
    # L-shaped footprint: full width below y=13, narrower (x<=48) above.
    f.area(2, 13, 57, 33, "#", "-")
    f.area(2, 2, 48, 13, "#", "-")


def elevator_and_stairs(f, hall_room):
    f.box(*ELEV, "e", "L")
    f.room("L", 20, "Winda", "elevator")
    f.area(*ELEV_DOOR, "E", hall_room)
    f.box(*STAIRS, ".", "Q")
    f.room("Q", 21, "Klatka schodowa", "stairs")
    f.area(*STAIRS_FLIGHT, "S", "Q")
    f.area(*STAIRS_DOOR, "D", hall_room)


def floor0():
    f = Floor(0, "v", "O")
    f.room("O", 1, "Na zewnątrz", "outside")
    # Map edge fence
    f.area(0, 0, W - 1, 0, "F", "-")
    f.area(0, H - 1, W - 1, H - 1, "F", "-")
    f.area(0, 0, 0, H - 1, "F", "-")
    f.area(W - 1, 0, W - 1, H - 1, "F", "-")
    building_shell(f, ".")

    # --- inside ---
    f.box(3, 3, 18, 32, "=", "K")
    f.room("K", 2, "Parking wewnętrzny", "parking")
    for y in (5, 11, 17, 23, 29):
        f.put(5, y, 7, y + 1, "X")
        f.put(14, y, 16, y + 1, "X")
    f.area(8, 33, 12, 33, "g", "K")                      # garage gate to the outside

    f.box(20, 14, 56, 20, ".", "H")
    f.room("H", 3, "Hol", "hall")
    f.area(19, 16, 19, 17, "D", "H")                     # parking <-> hall
    elevator_and_stairs(f, "H")

    f.box(40, 3, 47, 12, ":", "Y")
    f.room("Y", 9, "Zaplecze techniczne", "service")
    f.put(41, 4, 46, 4, "T")                             # racks
    f.put(41, 8, 42, 11, "T")
    f.area(43, 13, 44, 13, "L", "H")                     # locked (service access)

    f.box(20, 22, 25, 32, ".", "P")
    f.room("P", 4, "Portiernia", "reception")
    f.put(21, 27, 21, 30, "T")                           # porter's desk

    f.box(27, 22, 40, 32, "_", "E")
    f.room("E", 5, "Wejście", "entrance")
    f.area(26, 28, 26, 29, "D", "E")                     # portiernia <-> lobby
    for x in range(28, 40, 2):                           # card gates hall <-> lobby
        f.area(x, 21, x, 21, "B", "E")
    f.area(31, 33, 35, 33, "G", "E")                     # glass entrance doors

    f.box(42, 22, 56, 32, ".", "Z")
    f.room("Z", 6, "Sklep", "shop")
    f.area(41, 29, 41, 30, "D", "E")                     # shop <-> lobby
    for y in (24, 27):
        f.put(45, y, 54, y, "T")                         # shelves
    f.put(51, 31, 55, 31, "T")                           # counter

    # --- outside ---
    f.area(1, 34, W - 2, 36, "p", "O")                   # sidewalk
    f.area(7, 34, 13, 37, "=", "O")                      # driveway
    f.area(3, 38, 30, 45, "=", "R")
    f.room("R", 7, "Parking zewnętrzny", "parking")
    for x in (4, 9, 14, 19, 24):
        f.put(x, 39, x + 2, 40, "X")
        f.put(x, 43, x + 2, 44, "X")
    f.area(42, 39, 54, 45, "z", "M")
    f.room("M", 8, "Strefa palenia", "smoking")
    f.put(44, 41, 46, 41, "T")                           # bench
    f.put(50, 43, 50, 43, "T")                           # ashtray

    f.spawns = [[x, y] for y in (35, 36) for x in range(28, 39)]
    return f


def floor1():
    f = Floor(1, "~")
    building_shell(f, ".")

    f.box(20, 14, 44, 20, ".", "C")
    f.room("C", 1, "Recepcja", "reception")
    elevator_and_stairs(f, "C")
    f.put(30, 17, 34, 17, "T")                           # reception desk

    f.box(46, 14, 56, 20, ",", "Z")
    f.room("Z", 2, "Zarząd", "management")
    f.put(49, 16, 53, 17, "T")

    f.box(3, 3, 18, 20, ",", "I")
    f.room("I", 3, "IT / Produkt", "department")
    for y in (5, 9, 13, 17):
        f.put(5, y, 9, y, "T")
        f.put(12, y, 16, y, "T")

    f.box(40, 3, 47, 12, ",", "R")
    f.room("R", 4, "HR", "department")
    f.put(42, 6, 45, 6, "T")
    f.area(43, 13, 44, 13, "D", "C")                     # HR <-> reception

    f.box(3, 22, 56, 24, ".", "K")
    f.room("K", 5, "Korytarz", "corridor")
    f.area(30, 21, 34, 21, "D", "K")                     # reception <-> corridor
    f.area(10, 21, 11, 21, "D", "K")                     # IT <-> corridor
    f.area(51, 21, 52, 21, "D", "K")                     # board <-> corridor

    f.box(3, 26, 22, 32, ",", "B")
    f.room("B", 6, "Biznes", "department")
    for y in (28, 31):
        f.put(5, y, 9, y, "T")
        f.put(13, y, 17, y, "T")
    f.area(12, 25, 13, 25, "D", "K")

    f.box(24, 26, 42, 32, ",", "H")
    f.room("H", 7, "Chill room", "common")
    f.put(27, 28, 29, 29, "T")                           # sofa
    f.put(36, 30, 39, 30, "T")                           # table
    f.area(32, 25, 34, 25, "D", "K")

    f.box(44, 26, 49, 32, ":", "W")
    f.room("W", 8, "Łazienka damska", "bathroom")
    f.area(46, 25, 46, 25, "D", "K")
    f.box(51, 26, 56, 32, ":", "M")
    f.room("M", 9, "Łazienka męska", "bathroom")
    f.area(53, 25, 53, 25, "D", "K")
    return f


def links():
    ex0, ey0, ex1, ey1 = ELEV
    sx0, sy0, sx1, sy1 = STAIRS_FLIGHT
    elevator = {"kind": "elevator", "id": "main", "area": [ex0, ey0, ex1 - ex0 + 1, ey1 - ey0 + 1]}
    return {
        0: [elevator, {"kind": "stairs", "area": [sx0, sy0, sx1 - sx0 + 1, sy1 - sy0 + 1], "to_floor": 1, "to": list(STAIRS_ARRIVAL)}],
        1: [elevator, {"kind": "stairs", "area": [sx0, sy0, sx1 - sx0 + 1, sy1 - sy0 + 1], "to_floor": 0, "to": list(STAIRS_ARRIVAL)}],
    }


def to_json(f, floor_links):
    tiles, rooms = f.rows()
    used = {c for row in tiles for c in row}
    return {
        "version": 1,
        "id": "floor%d" % f.floor,
        "floor": f.floor,
        "tile_px": 16,
        "width": W,
        "height": H,
        "legend": {k: v for k, v in LEGEND.items() if k in used},
        "tiles": tiles,
        "rooms": rooms,
        "room_defs": f.rooms,
        "links": floor_links,
        "spawns": f.spawns,
    }


def check(f):
    # Doors must connect two walkable tiles (catches walls drawn over doors).
    for y in range(1, H - 1):
        for x in range(1, W - 1):
            if f.t[y][x] in "DEGBg":
                n = [f.t[y][x - 1], f.t[y][x + 1], f.t[y - 1][x], f.t[y + 1][x]]
                if sum(1 for c in n if not LEGEND[c]["solid"]) < 2:
                    sys.exit("floor %d: door at (%d,%d) leads nowhere" % (f.floor, x, y))


def main():
    floors = [floor0(), floor1()]
    for f in floors:
        check(f)
    lk = links()
    if "--preview" in sys.argv:
        for f in floors:
            print("=== piętro %d ===" % f.floor)
            print("\n".join(f.rows()[0]))
            print()
        return
    for f in floors:
        path = os.path.join(OUT, "floor%d.json" % f.floor)
        with open(path, "w", encoding="utf-8") as fh:
            json.dump(to_json(f, lk[f.floor]), fh, ensure_ascii=False, indent=1)
            fh.write("\n")
    building = {
        "version": 1,
        "floors": [
            {"floor": 0, "file": "floor0.json", "name": "Parter"},
            {"floor": 1, "file": "floor1.json", "name": "Piętro 1"},
            {"floor": 2, "file": None, "name": "Piętro 2", "locked": True},
        ],
    }
    with open(os.path.join(OUT, "building.json"), "w", encoding="utf-8") as fh:
        json.dump(building, fh, ensure_ascii=False, indent=1)
        fh.write("\n")
    print("written to", OUT)


if __name__ == "__main__":
    main()
