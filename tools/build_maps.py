#!/usr/bin/env python3
"""Generates the building maps (client/maps/*.json) from a readable description.

The generated JSON files are the single source of truth read by both the server
and the client; this script only exists to make editing the layout easier.
Everything a floor file holds comes from here (edit this, not the JSON).

    python3 tools/build_maps.py            # write client/maps/*.json
    python3 tools/build_maps.py --preview  # print ASCII only

The layout follows the hand-drawn plan (numbers in the comments are the
numbers on the drawing): ground floor 1-12, floor 1 13-58. Floor 3 follows
the architect's plan of the third storey (room numbers 301-319).
"""
import json
import os
import sys

W, H = 70, 72
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
OUT = os.path.join(ROOT, "client", "maps")

# One character per tile type across all floors.
LEGEND = {
    "#": {"type": "wall", "solid": True, "color": "#3b3b4f"},
    ".": {"type": "floor", "solid": False, "color": "#c9c2b0"},
    ",": {"type": "carpet", "solid": False, "color": "#7d8fb3"},
    ":": {"type": "tiles", "solid": False, "color": "#e6e6e6"},
    "_": {"type": "lobby", "solid": False, "color": "#d8c7a0"},
    "=": {"type": "parking", "solid": False, "color": "#6f6f6f"},
    "D": {"type": "door", "solid": False, "color": "#a0522d"},
    "G": {"type": "glass_door", "solid": False, "color": "#8fd3e8"},
    # access: needs a pass/card ("card") or service access ("service");
    # free_dir: direction you may always pass in (exit through the gates).
    "B": {"type": "card_gate", "solid": False, "color": "#e0b040", "access": "card", "free_dir": "down"},
    "L": {"type": "service_door", "solid": False, "color": "#6a3a2a", "access": "service"},
    "g": {"type": "garage_gate", "solid": False, "color": "#9a9a9a", "access": "card", "free_dir": "up"},
    # Locked for good (the closed zone, the server room, a locked wardrobe).
    "x": {"type": "locked_door", "solid": True, "color": "#7a2a2a"},
    "E": {"type": "elevator_door", "solid": False, "color": "#b8c4cc"},
    "e": {"type": "elevator", "solid": False, "color": "#9aa8b0"},
    "S": {"type": "stairs", "solid": False, "color": "#b09070"},
    "s": {"type": "steps", "solid": False, "color": "#a58a6c"},
    # Furniture: all solid, the type only decides how the client draws it.
    "T": {"type": "table", "solid": True, "color": "#8b6b4a"},
    "W": {"type": "desk", "solid": True, "color": "#9a7650"},
    "K": {"type": "counter", "solid": True, "color": "#c9a37a"},
    "H": {"type": "shelf", "solid": True, "color": "#7a7f8a"},
    "Q": {"type": "sofa", "solid": True, "color": "#5b7fbf"},
    "P": {"type": "plant", "solid": True, "color": "#3f8a3a"},
    "R": {"type": "rack", "solid": True, "color": "#2a2d34"},
    "N": {"type": "bench", "solid": True, "color": "#8a6a45"},
    "A": {"type": "ashtray", "solid": True, "color": "#6d6d6d"},
    "U": {"type": "toilet", "solid": True, "color": "#f2f2f2"},
    "u": {"type": "urinal", "solid": True, "color": "#f2f2f2"},
    "V": {"type": "sink", "solid": True, "color": "#dfe8ee"},
    "X": {"type": "car", "solid": True, "color": "#b03a2e"},
    "C": {"type": "coffee_machine", "solid": True, "color": "#2b2b30"},
    "J": {"type": "kitchen_counter", "solid": True, "color": "#d8d2c4"},
    "O": {"type": "fruit_bowl", "solid": True, "color": "#e0a040"},
    "c": {"type": "cupboard", "solid": True, "color": "#b89a70"},
    "i": {"type": "kitchen_sink", "solid": True, "color": "#cfd8de"},
    "d": {"type": "dishwasher", "solid": True, "color": "#bfc6cc"},
    "f": {"type": "fridge", "solid": True, "color": "#e8eef2"},
    "w": {"type": "wardrobe", "solid": True, "color": "#8a6a4a"},
    "o": {"type": "bin", "solid": True, "color": "#5a5f66"},
    # Commuting: the street in front of the building, the tram line and a
    # bike rack by the entrance.
    "r": {"type": "street", "solid": False, "color": "#55585e"},
    "t": {"type": "tram_track", "solid": False, "color": "#6b6259"},
    "b": {"type": "bike_rack", "solid": True, "color": "#9aa4ab"},
    # Toilet stalls: thin partitions and a door that can be locked from inside
    # (locked = solid for everyone; the server tells clients which ones).
    "|": {"type": "partition", "solid": True, "color": "#c3c9d1"},
    "Y": {"type": "sanitizer", "solid": True, "color": "#e8f1f8"},
    # Board room door: only with a meeting now (calendar); leaving is free.
    "Z": {"type": "board_door", "solid": False, "color": "#7a4a2a", "access": "board", "free_dir": "right"},
    "k": {"type": "stall_door", "solid": False, "color": "#9fb3c8"},
    "v": {"type": "grass", "solid": False, "color": "#5e8c4a"},
    "p": {"type": "sidewalk", "solid": False, "color": "#a8a8a0"},
    "z": {"type": "smoking_area", "solid": False, "color": "#8a7f6a"},
    "F": {"type": "fence", "solid": True, "color": "#4a3a2a"},
    "n": {"type": "balcony", "solid": False, "color": "#9a8a78"},
    "h": {"type": "railing", "solid": True, "color": "#3a3530"},
    "~": {"type": "void", "solid": True, "color": "#1c1c24"},
}

# Departments of the company (recruitment.json): rooms whose desks belong
# to one carry its id.
IT, BUSINESS, BOARD, MOBILE, DEVOPS, AI, FINANCE, SALES, MARKETING, SUPPORT = range(1, 11)


class Floor:
    def __init__(self, floor, fill, room_fill="-"):
        self.floor = floor
        self.t = [[fill] * W for _ in range(H)]
        self.r = [[room_fill] * W for _ in range(H)]
        self.rooms = {}
        self.links = []
        self.spawns = []
        self.npcs = []
        self.places = {}

    def room(self, key, rid, name, kind, see=None, gender=None, outdoor=False, detector=False,
             light=None, switch_door=None, windows=False, lit_by=None, below=None, department=None):
        d = {"id": rid, "name": name, "type": kind}
        if see:
            # Rooms whose people are visible from here (open door / window).
            d["see"] = see
        if gender:
            # Bathrooms: "female" / "male" (only a sign; anyone may go in).
            d["gender"] = gender
        if outdoor:
            # Under the open sky: weather (rain, sun) applies here.
            d["outdoor"] = True
        if detector:
            # A smoke detector on the ceiling (cigarettes set off the alarm).
            d["detector"] = True
        if below:
            # A balcony: the rooms right under it (seen from above).
            d["below"] = below
        if light:
            # "always" (common areas) or "switch" (a switch by the door).
            d["light"] = light
        if windows:
            # Daylight gets in (dark at night without the lamp).
            d["windows"] = True
        if lit_by:
            # A stall has no lamp of its own: the bathroom's light.
            d["lit_by"] = lit_by
        if department:
            # The desks here are this department's.
            d["department"] = department
        self.rooms[key] = d
        if switch_door:
            self.switch_doors[key] = switch_door
        return d

    switch_doors = None

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

    def walls(self, x0, y0, x1, y1, tile, room):
        """Same as box, given the wall lines instead of the interior."""
        self.box(x0 + 1, y0 + 1, x1 - 1, y1 - 1, tile, room)

    def put(self, x0, y0, x1, y1, tile):
        self.area(x0, y0, x1, y1, tile)

    def door(self, x0, y0, x1, y1, tile, room):
        self.area(x0, y0, x1, y1, tile, room)

    def plants(self, spots):
        for x, y in spots:
            self.put(x, y, x, y, "P")

    def desk_rows(self, x0, x1, ys):
        for y in ys:
            self.put(x0, y, x1, y, "W")

    def rows(self):
        return ["".join(r) for r in self.t], ["".join(r) for r in self.r]

    def finish(self):
        """Light switches: on the room side of the door they were given."""
        for key, (dx, dy) in (self.switch_doors or {}).items():
            for nx, ny in [(dx + 1, dy), (dx - 1, dy), (dx, dy + 1), (dx, dy - 1)]:
                if self.r[ny][nx] == key and not LEGEND[self.t[ny][nx]]["solid"] and self.t[ny][nx] not in "Dk":
                    self.rooms[key]["switch"] = [nx, ny]
                    break
            else:
                sys.exit("floor %d: no switch spot by the door of %s" % (self.floor, key))


def stall(f, key, rid, name, bath, gender, toilet, stand, door, bath_name):
    """A toilet stall: its own room (nobody outside sees in; from inside you
    still see the bathroom), lit by the bathroom's lamp."""
    f.room(key, rid, name, "stall", see=[bath] if bath else None, gender=gender, lit_by=bath_name)
    f.area(*toilet, *toilet, "U", key)
    f.area(*stand, *stand, ":", key)
    f.area(*door, *door, "k", key)


# ------------------------------------------------------------------ shared
# Shared building geometry (must line up between floors): two elevators
# side by side, doors facing down (8/9 on the ground floor, 14/15 upstairs),
# each running on its own.
ELEV_A = (36, 41, 38, 42)    # cabin interior: 3 x 2, a tight fit for 6
ELEV_B = (40, 41, 42, 42)
SHAFT = (35, 40, 43, 43)     # walls around both (a pillar between the doors)
ELEV_A_DOOR = (36, 43, 38, 43)
ELEV_B_DOOR = (40, 43, 42, 43)

# The stairwells are maps of their own ("floors" 4 and 5 in the building
# list, not real storeys): a U-shaped staircase - flight up from the lower
# floor on the left, the landing (półpiętro) at the top, flight on the right
# leading to the upper floor. Walking it takes a few seconds and you only
# see the stairwell. Map 4 joins the ground floor and floor 1, map 5 floor 1
# and floor 3 (floor 2 is locked; its stairwell door stays shut).
STAIRWELL_FLOOR = 4              # between 0 and 1
STAIRWELL_UP_FLOOR = 5           # between 1 and 3
MID_FLIGHT_A = (31, 7, 33, 14)   # from / to the lower floor
MID_FLIGHT_B = (35, 7, 37, 14)   # from / to the upper floor
MID_ARRIVAL_A = (32, 13)
MID_ARRIVAL_B = (36, 13)
STAIRS0 = (19, 41, 21, 42)       # the flight in 4 (ground floor)
STAIRS0_ARRIVAL = (24, 42)
STAIRS1 = (19, 39, 21, 40)       # the flight in 24 (floor 1), down
STAIRS1_ARRIVAL = (25, 41)
STAIRS1_UP = (26, 39, 28, 40)    # the flight in 24 (floor 1), up to floor 3
STAIRS1_UP_ARRIVAL = (27, 42)
STAIRS3 = (25, 39, 27, 40)       # the flight in 301 (floor 3)
STAIRS3_ARRIVAL = (30, 41)

# Outside (south of the building): sidewalk, street, the car park, the tram.
STREET_Y = 61
TRAM_Y = 70


def elevators(f, hall_room):
    f.area(*SHAFT, "#", "-")
    f.box(*ELEV_A, "e", "L")
    f.room("L", 20, "Winda", "elevator", light="always")
    f.area(*ELEV_A_DOOR, "E", hall_room)
    f.box(*ELEV_B, "e", "l")
    f.room("l", 22, "Winda 2", "elevator", light="always")
    f.area(*ELEV_B_DOOR, "E", hall_room)


# ------------------------------------------------------------ ground floor
def floor0():
    f = Floor(0, "v", "O")
    f.switch_doors = {}
    f.room("O", 1, "Na zewnątrz", "outside", see=["R"], outdoor=True)
    f.area(0, 0, W - 1, 0, "F", "-")                     # map edge fence
    f.area(0, H - 1, W - 1, H - 1, "F", "-")
    f.area(0, 0, 0, H - 1, "F", "-")
    f.area(W - 1, 0, W - 1, H - 1, "F", "-")
    f.area(18, 13, 54, 57, "#", "-")                     # the building

    # 12: the indoor car park (the gate to the north, the drive round the
    # west side of the building).
    f.walls(18, 13, 54, 40, "=", "K")
    f.room("K", 2, "Parking wewnętrzny", "parking", light="always")
    for y in (15, 19, 23, 27, 31, 35):
        f.put(19, y, 21, y + 1, "X")
        f.put(51, y, 53, y + 1, "X")
    for x in (26, 44):
        f.put(x, 16, x + 2, 17, "X")
    f.area(33, 13, 38, 13, "g", "K")                     # garage gate (card; out is free)

    # 4: the stairwell (flight up to the landing).
    f.walls(18, 40, 27, 44, ".", "Q")
    f.room("Q", 21, "Klatka schodowa", "stairs", light="always")
    f.area(*STAIRS0, "S", "Q")

    # 1: the shop (entrance from the street).
    f.walls(18, 44, 27, 57, ".", "S")
    f.room("S", 6, "Sklep", "shop", detector=True, light="always", windows=True)
    f.put(19, 46, 19, 49, "H")                           # alcohol & cigarettes
    f.put(21, 46, 23, 46, "H")                           # sandwiches
    f.put(21, 49, 23, 49, "H")                           # fast food
    f.put(26, 46, 26, 49, "H")                           # snacks
    f.put(21, 51, 23, 51, "H")                           # drinks
    f.put(26, 52, 26, 53, "H")                           # umbrella stand
    f.put(19, 54, 21, 54, "K")                           # checkout counter
    f.door(22, 57, 23, 57, "G", "S")
    f.places["shelves"] = {
        "1": [21, 46, 3, 1], "2": [21, 49, 3, 1], "3": [26, 46, 1, 4],
        "4": [21, 51, 3, 1], "5": [19, 46, 1, 4], "6": [26, 52, 1, 2],
    }

    # 3: the entrance hall. North part (lifts, stairs, the car park) behind
    # the card gates; the porter's desk (5, the porter sits at 6) by them.
    f.walls(27, 40, 43, 57, "_", "H")
    f.room("H", 3, "Hol", "hall", detector=True, light="always", windows=True)
    elevators(f, "H")
    f.door(27, 42, 27, 42, "D", "Q")                     # stairwell <-> hall
    f.door(31, 40, 32, 40, "D", "H")                     # car park <-> hall
    f.area(35, 46, 39, 46, "#", "-")                     # back of the porter's lodge
    for x in range(28, 35):                              # card gates (out is free)
        f.area(x, 46, x, 46, "B", "H")
    f.put(35, 47, 35, 51, "K")                           # 5: porter's desk
    # 7: the toilet by the lodge (one lockable stall).
    f.walls(39, 46, 43, 52, ":", "T")
    f.room("T", 7, "Toaleta", "stall", light="always")
    f.area(41, 47, 41, 47, "U", "T")
    f.area(41, 52, 41, 52, "k", "T")
    f.put(42, 50, 42, 50, "V")
    # 2: the draught lobby (wiatrołap): glass doors out, a door into the hall.
    f.walls(27, 53, 35, 57, "_", "E")
    f.room("E", 5, "Wiatrołap", "entrance", light="always", windows=True)
    f.door(31, 53, 32, 53, "D", "E")
    f.door(30, 57, 32, 57, "G", "E")
    # 11: closed zone (locked doors from the hall).
    f.walls(43, 40, 54, 57, ".", "Z")
    f.room("Z", 9, "Strefa zamknięta", "service", light="always")
    f.door(43, 45, 43, 45, "x", "Z")
    f.door(43, 55, 43, 55, "x", "Z")
    f.plants([(28, 41), (28, 56), (42, 56)])

    # --- outside ---
    f.area(12, 9, 16, STREET_Y - 1, "=", "O")            # the drive to the garage
    f.area(12, 9, 40, 12, "=", "O")
    f.area(1, 58, W - 2, 60, "p", "O")                   # sidewalk
    f.area(1, STREET_Y, W - 2, STREET_Y, "r", "O")       # street
    f.area(3, 62, 30, 69, "=", "R")
    f.room("R", 8, "Parking zewnętrzny", "parking", see=["O"], outdoor=True)
    for x in (4, 9, 14, 19, 24):
        f.put(x, 63, x + 2, 64, "X")                     # parked (rows 67-68: free for players)
    f.area(1, TRAM_Y, W - 2, TRAM_Y, "t", "O")           # tram line
    f.area(32, 69, 40, 69, "p", "O")                     # tram stop platform
    f.put(36, 58, 39, 58, "b")                           # bike rack by the entrance
    f.put(24, 58, 24, 58, "A")                           # ashtray by the entrance
    f.area(42, 62, 54, 68, "z", "M")
    f.room("M", 10, "Strefa palenia", "smoking", outdoor=True)
    f.put(44, 64, 46, 64, "N")                           # bench
    f.put(50, 66, 50, 66, "A")                           # ashtray

    f.spawns = [[x, y] for y in (59, 60) for x in range(26, 36)]
    f.places.update({
        "street_y": STREET_Y,
        "tram_y": TRAM_Y,
        "walk_home": [2, 59],        # west end of the sidewalk (going home on foot)
        "walk_arrival": [1, 59],     # walking in from the west
        "taxi": [34, 60],            # the taxi stand (the car stops on the street by it)
        "tram_stop": [36, 69],
        "car_bays": [[5, 67], [10, 67], [15, 67], [20, 67], [25, 67], [28, 67]],
        "bike_rack": [[36, 58], [37, 58], [38, 58], [39, 58]],
        "police": [34, 60],          # the officer gets out here
        "fire": [28, 60],            # the crew gets out here
    })
    f.npcs = [
        # The porter sits behind the desk; escorts newcomers to the reception.
        {"kind": "porter", "name": "Portier", "home": [37, 49], "escort_to": [1, 36, 36]},
        # Behind the till; customers pay from the other side of the counter.
        {"kind": "cashier", "name": "Kasa", "home": [20, 55]},
        {"kind": "guard", "name": "Ochrona", "home": [25, 55]},
        # 10: the cleaner sits in the hall all day (her round in the afternoon).
        {"kind": "cleaner", "name": "Pani Krysia", "home": [38, 56]},
    ]
    return f


# ----------------------------------------------------------------- floor 1
def floor1():
    f = Floor(1, "~")
    f.switch_doors = {}

    # 48: the balcony (over the drive; smoking allowed, out in the open).
    f.area(18, 1, 33, 5, "h", "-")
    f.area(19, 2, 32, 5, "n", "X")
    f.room("X", 36, "Balkon", "balcony", outdoor=True, below=["Na zewnątrz"])
    f.put(31, 2, 31, 2, "A")                             # ashtray
    f.put(20, 2, 22, 2, "N")                             # bench

    # The main block (35-40 left, 33/34 middle, 41-47 right) and the lower wing.
    f.area(18, 6, 53, 38, "#", "-")
    f.area(2, 37, 65, 60, "#", "-")

    # 34 chill room (open to 35, the kitchenette) and 33, the corridor.
    f.area(29, 7, 42, 13, ",", "H")
    f.room("H", 7, "Chill room", "common", light="switch", windows=True)
    f.rooms["H"]["switch"] = [30, 13]                    # on the server room's wall
    f.area(19, 7, 28, 11, ":", "N")
    f.area(19, 12, 22, 16, ":", "N")
    f.room("N", 14, "Aneks kuchenny", "common", light="switch", switch_door=(23, 6), windows=True)
    f.door(23, 6, 24, 6, "G", "N")                       # out to the balcony
    f.put(19, 7, 19, 7, "f")                             # fridge
    f.put(20, 7, 21, 7, "c")                             # cupboard (mugs)
    f.put(22, 7, 22, 7, "J")
    f.put(25, 7, 25, 7, "C")                             # coffee machine
    f.put(26, 7, 26, 7, "J")
    f.put(27, 7, 27, 7, "i")                             # sink
    f.put(28, 7, 28, 7, "d")                             # dishwasher
    f.put(20, 10, 22, 11, "T")                           # a small table
    f.put(31, 8, 33, 9, "Q")                             # sofas
    f.put(37, 8, 39, 9, "Q")
    f.put(35, 11, 37, 11, "T")                           # the table (treats)
    f.put(41, 7, 41, 7, "O")                             # fruit bowl (free fruit)
    f.put(42, 7, 42, 7, "Y")                             # sanitizer by the food
    f.places["tray"] = [36, 11]

    f.area(30, 14, 42, 37, ".", "K")
    f.room("K", 5, "Korytarz", "corridor", detector=True, light="always")
    # The island in the corridor: bathrooms (51-54), the disabled toilet
    # (50), a wardrobe (55), the reception desk (49, the receptionist sits
    # at 56) and a bin (57).
    f.area(35, 14, 39, 26, "#", "-")
    f.area(35, 28, 39, 32, "#", "-")                     # (a walkway between the two)
    # 54: women's WC (a lockable room of its own, door to the corridor).
    f.area(36, 15, 38, 16, ":", "a")
    f.room("a", 30, "WC damskie", "stall", gender="female", light="always")
    f.put(36, 16, 36, 16, "U")
    f.door(39, 16, 39, 16, "k", "a")
    # 53: women's bathroom (washbasin).
    f.area(36, 18, 38, 19, ":", "W")
    f.room("W", 8, "Łazienka damska", "bathroom", gender="female", light="always")
    f.door(35, 18, 35, 18, "D", "W")
    f.put(38, 18, 38, 18, "V")
    f.put(38, 19, 38, 19, "Y")
    # 52: men's WC, through 51 (men's bathroom, washbasin).
    f.area(36, 21, 38, 22, ":", "x")
    f.room("x", 33, "WC męskie", "stall", see=["M"], gender="male", lit_by="Łazienka męska")
    f.put(36, 21, 36, 21, "U")
    f.door(37, 23, 37, 23, "k", "x")
    f.area(36, 24, 38, 25, ":", "M")
    f.room("M", 9, "Łazienka męska", "bathroom", gender="male", light="always")
    f.door(37, 26, 37, 26, "D", "M")
    f.put(36, 24, 36, 24, "V")
    f.put(38, 24, 38, 24, "Y")
    # 50: the disabled toilet (lockable, roomy).
    f.area(36, 29, 38, 31, ":", "y")
    f.room("y", 35, "WC dla niepełnosprawnych", "stall", light="always")
    f.door(37, 28, 37, 28, "k", "y")
    f.put(38, 31, 38, 31, "U")
    f.put(36, 29, 36, 29, "V")
    f.put(36, 33, 38, 33, "w")                           # 55: wardrobe
    f.put(34, 35, 38, 35, "K")                           # 49: reception desk
    f.put(42, 36, 42, 36, "o")                           # 57: bin
    f.plants([(30, 14), (30, 37)])

    # Left column: 36 server room, 37 the CEO (the board), 38 team room,
    # 39 and 40 meeting rooms, 24 the stairwell.
    f.walls(23, 12, 29, 17, ":", "Y")
    f.room("Y", 15, "Serwerownia", "service", light="always")
    f.put(24, 13, 28, 13, "R")
    f.put(24, 16, 26, 16, "R")
    f.door(29, 15, 29, 15, "x", "Y")                     # locked
    f.walls(18, 17, 29, 22, ",", "Z")
    f.room("Z", 2, "Zarząd", "management", detector=True, light="switch", switch_door=(29, 19),
           windows=True, department=BOARD)
    f.put(21, 19, 25, 20, "T")                           # the board table
    f.door(29, 19, 29, 19, "Z", "K")                     # meetings only
    f.places["founder"] = [23, 21]
    f.walls(18, 22, 29, 30, ",", "I")
    f.room("I", 3, "Produkt / IT", "department", detector=True, light="switch", switch_door=(29, 27),
           windows=True, department=IT)
    f.desk_rows(21, 24, (24, 27))
    f.door(29, 27, 29, 27, "D", "K")
    f.walls(18, 30, 29, 34, ",", "m")
    f.room("m", 16, "Sala spotkań 1", "meeting", detector=True, light="switch", switch_door=(29, 32), windows=True)
    f.put(21, 32, 25, 32, "T")
    f.door(29, 32, 29, 32, "D", "K")
    f.walls(18, 34, 29, 38, ",", "n")
    f.room("n", 17, "Sala spotkań 2", "meeting", detector=True, light="switch", switch_door=(29, 36), windows=True)
    f.put(21, 36, 25, 36, "T")
    f.door(29, 36, 29, 36, "D", "K")
    f.walls(18, 38, 29, 43, ".", "Q")
    f.room("Q", 21, "Klatka schodowa", "stairs", light="always")
    f.area(*STAIRS1, "S", "Q")                           # down to the ground floor
    f.area(*STAIRS1_UP, "S", "Q")                        # up to floor 3

    # Right column: 47 meeting room, 46 HR, 44 marketing (45 cleaning
    # cupboard), 43 sales, 42 customer service, 41 storeroom.
    f.walls(43, 6, 53, 11, ",", "s")
    f.room("s", 18, "Sala spotkań 3", "meeting", detector=True, light="switch", switch_door=(43, 9), windows=True)
    f.put(46, 8, 50, 9, "T")
    f.door(43, 9, 43, 9, "D", "K")
    f.walls(43, 11, 53, 15, ",", "R")
    f.room("R", 4, "HR", "department", detector=True, light="switch", switch_door=(43, 14), windows=True)
    f.put(46, 13, 49, 13, "W")                           # HR desk
    f.door(43, 14, 43, 14, "D", "K")
    f.walls(43, 15, 53, 24, ",", "k")
    f.room("k", 10, "Marketing", "department", detector=True, light="switch", switch_door=(43, 23),
           windows=True, department=MARKETING)
    f.desk_rows(48, 51, (17, 21))
    f.door(43, 23, 43, 23, "D", "K")
    f.walls(43, 18, 47, 21, ":", "c")
    f.room("c", 19, "Składzik", "service", light="always")
    f.put(44, 19, 45, 19, "H")
    f.door(43, 20, 43, 20, "L", "c")                     # the cleaner's
    f.walls(43, 24, 53, 32, ",", "j")
    f.room("j", 11, "Sales", "department", detector=True, light="switch", switch_door=(43, 29),
           windows=True, department=SALES)
    f.desk_rows(46, 49, (26, 30))
    f.door(43, 29, 43, 29, "D", "K")
    f.walls(43, 32, 53, 38, ",", "o")
    f.room("o", 12, "Obsługa klienta", "department", detector=True, light="switch", switch_door=(43, 35),
           windows=True, department=SUPPORT)
    f.desk_rows(46, 49, (34,))
    f.door(43, 35, 43, 35, "D", "K")
    f.walls(43, 38, 53, 43, ":", "g")
    f.room("g", 13, "Magazynek", "storage", light="switch", switch_door=(48, 43), windows=True)
    f.put(44, 39, 47, 39, "H")
    f.put(49, 39, 52, 39, "H")

    # 13: the hall by the lifts (58: a locked wardrobe over them).
    f.area(30, 39, 42, 48, ".", "C")
    f.room("C", 1, "Hol windowy", "hall", detector=True, light="always")
    f.area(29, 38, 43, 38, "#", "-")
    f.door(32, 38, 33, 38, "D", "K")                     # corridor <-> hall
    elevators(f, "C")
    f.walls(35, 38, 43, 40, ":", "q")
    f.room("q", 23, "Szafa", "service", light="always")
    f.door(35, 39, 35, 39, "x", "q")                     # locked
    f.plants([(30, 39), (42, 48)])

    # Lower left wing: 24 stairs (above), 20 mobile, 21 bathroom (22, 23
    # stalls), 17 corridor, 19 and 18 team rooms, 16 finance.
    f.walls(2, 37, 18, 43, ",", "b")
    f.room("b", 24, "Mobile", "department", detector=True, light="switch", switch_door=(13, 43),
           windows=True, department=MOBILE)
    f.desk_rows(5, 10, (39,))
    f.put(13, 39, 15, 39, "W")
    f.walls(11, 43, 29, 49, ".", "D")
    f.room("D", 25, "Korytarz zachodni", "corridor", detector=True, light="always")
    f.door(13, 43, 14, 43, "D", "D")                     # mobile
    f.door(22, 43, 23, 43, "D", "D")                     # stairwell
    f.door(29, 45, 29, 46, "D", "D")                     # hall 13
    f.walls(2, 43, 11, 49, ":", "w")
    f.room("w", 26, "Łazienka damska (zachód)", "bathroom", gender="female", light="switch", switch_door=(11, 46),
           windows=True)
    f.door(11, 46, 11, 46, "D", "w")
    for i, y in enumerate((45, 47)):                     # 22, 23: stalls on the west wall
        key = "12"[i]
        stall(f, key, 40 + i, "Kabina %d (damska)" % (i + 1), "w", "female", (3, y), (4, y), (5, y),
              "Łazienka damska (zachód)")
    f.put(3, 44, 5, 44, "|")
    f.put(3, 46, 5, 46, "|")
    f.put(3, 48, 5, 48, "|")
    f.put(10, 44, 10, 45, "V")                           # sinks
    f.put(10, 48, 10, 48, "Y")
    f.walls(2, 49, 14, 60, ",", "e")
    f.room("e", 27, "Produkt / IT", "department", detector=True, light="switch", switch_door=(12, 49),
           windows=True, department=IT)
    f.desk_rows(5, 8, (52, 56))
    f.desk_rows(10, 12, (52, 56))
    f.door(12, 49, 13, 49, "D", "D")
    f.walls(14, 49, 27, 60, ",", "f")
    f.room("f", 28, "Produkt / IT", "department", detector=True, light="switch", switch_door=(21, 49),
           windows=True, department=IT)
    f.desk_rows(18, 21, (52, 56))
    f.door(21, 49, 22, 49, "D", "D")
    f.walls(27, 49, 43, 60, ",", "F")
    f.room("F", 29, "Finanse", "department", detector=True, light="switch", switch_door=(33, 49),
           windows=True, department=FINANCE)
    f.desk_rows(30, 34, (52, 56))
    f.desk_rows(37, 40, (52, 56))
    f.door(33, 49, 34, 49, "D", "C")

    # Lower right wing: 25 corridor, 41 above, 32 DevOps ("Mordor"), 26 a
    # one-desk room, 27 business, 28 AI, 29-31 the men's bathroom.
    f.walls(43, 43, 53, 49, ".", "U")
    f.area(48, 49, 57, 53, ".", "U")
    f.room("U", 37, "Korytarz wschodni", "corridor", detector=True, light="always")
    f.door(43, 45, 43, 46, "D", "U")                     # hall 13 <-> 25
    f.door(48, 43, 48, 43, "D", "U")                     # storeroom 41
    f.walls(53, 37, 65, 49, ",", "V")
    f.room("V", 31, "DevOps (Mordor)", "department", detector=True, light="switch", switch_door=(53, 45),
           windows=True, department=DEVOPS)
    f.desk_rows(56, 59, (40, 44))
    f.desk_rows(61, 63, (40, 44))
    f.door(53, 45, 53, 46, "D", "V")
    f.walls(43, 49, 47, 54, ",", "h")
    f.room("h", 32, "Pokój z biurkiem", "department", detector=True, light="switch", switch_door=(47, 51))
    f.put(44, 51, 44, 51, "W")
    f.door(47, 51, 47, 51, "D", "h")
    f.walls(43, 54, 53, 60, ",", "B")
    f.room("B", 6, "Biznes", "department", detector=True, light="switch", switch_door=(49, 54),
           windows=True, department=BUSINESS)
    f.desk_rows(46, 50, (57,))
    f.door(49, 54, 50, 54, "D", "B")
    f.walls(53, 54, 65, 60, ",", "i")
    f.room("i", 38, "AI team", "department", detector=True, light="switch", switch_door=(55, 54),
           windows=True, department=AI)
    f.desk_rows(56, 62, (57,))
    f.door(55, 54, 56, 54, "D", "i")
    f.walls(58, 49, 65, 54, ":", "u")
    f.room("u", 34, "Łazienka męska (wschód)", "bathroom", gender="male", light="switch", switch_door=(58, 51),
           windows=True)
    f.door(58, 51, 58, 51, "D", "u")
    f.put(59, 50, 60, 50, "V")                           # sinks
    f.put(59, 53, 59, 53, "Y")
    f.put(62, 50, 64, 50, "u")                           # 30: urinals
    f.put(62, 52, 64, 52, "|")                           # 31: the stall
    stall(f, "v", 42, "Kabina (męska)", "u", "male", (64, 53), (63, 53), (62, 53), "Łazienka męska (wschód)")

    f.npcs = [
        # 56: the receptionist behind the desk (guests come to its front,
        # row 36); takes newcomers to HR.
        {"kind": "receptionist", "name": "Recepcja", "home": [36, 34], "escort_to": [1, 47, 14]},
        {"kind": "hr", "name": "HR", "home": [47, 12]},
        # The board: the CEO and the co-founder at the table in 37.
        {"kind": "ceo", "name": "Prezes", "home": [20, 19]},
        {"kind": "cofounder", "name": "Wspólniczka", "home": [27, 20]},
    ]
    return f


# ----------------------------------------------------------------- floor 3
def floor3():
    """The third storey after the architect's plan ("rzut P3"), turned a
    quarter turn clockwise so the lifts' doors face south like on the other
    floors: the plan's west (the lounge) is north here, its east (the wing
    of meeting rooms) south. Numbers in the comments are the plan's."""
    f = Floor(3, "~")
    f.switch_doors = {}

    # The main block; balconies 3 and 4 (over the drive) off the lounge.
    f.area(17, 6, 54, 38, "#", "-")
    for key, rid, name, x0 in (("3", 2, "Balkon 3", 20), ("4", 3, "Balkon 4", 44)):
        f.area(x0 - 1, 1, x0 + 7, 5, "h", "-")
        f.area(x0, 2, x0 + 6, 5, "n", key)
        f.room(key, rid, name, "balcony", outdoor=True)
        f.put(x0 + 6, 2, x0 + 6, 2, "A")                 # ashtray

    # 312: the lounge - tables to the east, sofas to the west. Open to the
    # kitchen (313) at its east end; a passage down to the open space at its
    # west end.
    f.area(18, 7, 53, 14, ",", "W")
    f.room("W", 1, "Pokój wypoczynkowy", "common", see=["N"], detector=True, light="always", windows=True)
    f.door(23, 6, 23, 6, "G", "W")                       # out to balcony 3
    f.door(47, 6, 47, 6, "G", "W")                       # out to balcony 4
    for x in (31, 36, 41):
        f.put(x, 8, x + 1, 9, "T")
        f.put(x, 12, x + 1, 13, "T")
    f.put(49, 10, 50, 11, "T")
    f.put(20, 9, 22, 9, "Q")                             # sofas
    f.put(20, 12, 22, 12, "Q")
    f.put(25, 10, 25, 11, "Q")
    f.plants([(18, 7), (53, 14)])

    # The row between the lounge and the open space (the plan's middle
    # column): the passage, a meeting room, 311 server room, 310 and 309
    # storerooms, a toilet, 313 the kitchen.
    f.area(18, 15, 19, 20, ",", "W")                     # the passage (part of the lounge)
    f.walls(20, 15, 25, 21, ",", "S")
    f.room("S", 4, "Pokój spotkań", "meeting", detector=True, light="switch", switch_door=(22, 21), windows=True)
    f.put(22, 17, 23, 18, "T")
    f.walls(25, 15, 29, 21, ":", "Y")
    f.room("Y", 5, "Serwerownia", "service", light="always")
    f.put(26, 16, 28, 16, "R")
    f.put(26, 19, 27, 19, "R")
    f.walls(29, 15, 33, 21, ":", "m")
    f.room("m", 6, "Magazyn 1", "storage", light="switch", switch_door=(31, 21))
    f.put(30, 16, 32, 16, "H")
    f.walls(33, 15, 38, 21, ":", "n")
    f.room("n", 7, "Magazyn 2", "storage", light="switch", switch_door=(35, 21))
    f.put(34, 16, 37, 16, "H")
    f.walls(38, 15, 43, 21, ":", "t")
    f.room("t", 8, "Toaleta", "stall", light="always")
    f.put(39, 16, 39, 16, "U")
    f.put(42, 16, 42, 16, "V")
    f.put(42, 17, 42, 17, "Y")
    f.walls(43, 15, 54, 21, ":", "N")
    f.area(44, 15, 53, 15, ":", "N")                     # open to the lounge
    f.room("N", 9, "Kuchnia", "common", see=["W"], light="switch", switch_door=(48, 21), windows=True)
    f.put(44, 16, 45, 16, "J")
    f.put(46, 16, 46, 16, "C")                           # coffee machine
    f.put(47, 16, 47, 16, "J")
    f.put(48, 16, 48, 16, "O")                           # fruit bowl
    f.put(49, 16, 49, 16, "Y")
    f.put(51, 18, 52, 19, "T")

    # 304 open space: desks on both sides of the walkway (komunikacja),
    # with the toilets (308, 307, 306) on islands in it and 305 the reception
    # desk at its end. Balconies 1, 2 (west) and 5, 6 (east) are tiny.
    f.area(18, 22, 28, 37, ",", "1")
    f.room("1", 10, "Open space (zachód)", "department", see=["K", "2"], detector=True, light="always", windows=True)
    f.area(29, 22, 42, 37, ".", "K")
    f.room("K", 11, "Komunikacja", "corridor", see=["1", "2"], detector=True, light="always", windows=True)
    f.area(43, 22, 53, 37, ",", "2")
    f.room("2", 12, "Open space (wschód)", "department", see=["1", "K"], detector=True, light="always", windows=True)
    desk_ys = (23, 25, 28, 30, 33, 35)
    f.desk_rows(21, 24, desk_ys)
    f.desk_rows(47, 50, desk_ys)
    f.walls(33, 23, 38, 30, ":", "a")
    f.room("a", 13, "WC damski", "stall", gender="female", light="always")
    f.put(34, 24, 34, 24, "U")
    f.put(37, 26, 37, 26, "V")
    f.area(33, 25, 33, 25, "k", "a")
    f.area(34, 27, 37, 27, "#", "-")
    f.area(34, 28, 37, 29, ":", "x")
    f.room("x", 14, "WC męski", "stall", gender="male", light="always")
    f.put(34, 28, 34, 28, "U")
    f.put(34, 29, 34, 29, "u")
    f.put(37, 29, 37, 29, "V")
    f.area(38, 28, 38, 28, "k", "x")
    f.walls(33, 31, 38, 35, ":", "y")
    f.room("y", 15, "WC dla niepełnosprawnych", "stall", light="always")
    f.put(37, 34, 37, 34, "U")
    f.put(34, 32, 34, 32, "V")
    f.area(33, 33, 33, 33, "k", "y")
    f.put(35, 37, 38, 37, "K")                           # 305: reception desk
    f.plants([(28, 22), (28, 37), (43, 37), (53, 22)])
    f.door(18, 21, 19, 21, "D", "1")                     # the passage from the lounge
    f.door(22, 21, 22, 21, "D", "1")                     # meeting room
    f.door(27, 21, 27, 21, "x", "Y")                     # server room: locked
    f.door(31, 21, 31, 21, "D", "K")                     # storerooms
    f.door(35, 21, 35, 21, "D", "K")
    f.area(40, 21, 40, 21, "k", "t")                     # toilet
    f.door(48, 21, 49, 21, "D", "2")                     # kitchen
    for key, rid, name, x0, y0, door in (
        ("5", 16, "Balkon 1", 13, 23, (17, 25)),
        ("6", 17, "Balkon 2", 13, 31, (17, 33)),
        ("7", 18, "Balkon 5", 55, 23, (54, 25)),
        ("8", 19, "Balkon 6", 55, 31, (54, 33)),
    ):
        f.area(x0, y0, x0 + 3, y0 + 4, "h", "-")
        nx = x0 + 1 if x0 < 30 else x0                   # the open side is the building wall
        f.area(nx, y0 + 1, nx + 2, y0 + 3, "n", key)
        f.room(key, rid, name, "balcony", outdoor=True)
        f.put(nx + (0 if x0 < 30 else 2), y0 + 1, nx + (0 if x0 < 30 else 2), y0 + 1, "A")
        f.door(*door, *door, "G", "1" if x0 < 30 else "2")

    # The core: 318 meeting room and 319 ladies' at the west end, 301 the
    # stairwell, a passage past 303 (locked) to the lifts, 315 meeting room
    # at the east end; 302 the lift hall below with 317 reception desk, 316
    # gents' off it.
    f.walls(17, 38, 24, 45, ",", "k")
    f.room("k", 21, "Sala konferencyjna 6", "meeting", detector=True, light="switch", switch_door=(24, 44), windows=True)
    f.put(19, 41, 22, 41, "T")
    f.walls(17, 45, 24, 50, ":", "9")
    f.room("9", 23, "WC damski 2", "stall", gender="female", light="always")
    f.put(18, 46, 18, 46, "U")
    f.put(23, 49, 23, 49, "V")
    f.walls(24, 38, 32, 43, ".", "Q")
    f.room("Q", 24, "Klatka schodowa", "stairs", light="always")
    f.area(*STAIRS3, "S", "Q")
    f.area(33, 38, 34, 43, ".", "C")                     # passage to the lifts
    f.walls(35, 38, 43, 40, ":", "q")
    f.room("q", 25, "Pomieszczenie użytkowe", "service", light="always")
    f.put(40, 39, 42, 39, "H")
    f.walls(44, 38, 54, 43, ",", "s")
    f.room("s", 26, "Sala konferencyjna 5", "meeting", detector=True, light="switch", switch_door=(48, 43), windows=True)
    f.put(47, 40, 50, 41, "T")
    f.area(25, 44, 53, 49, ".", "C")
    f.room("C", 27, "Hol windowy", "hall", detector=True, light="always")
    elevators(f, "C")
    f.put(27, 47, 30, 47, "K")                           # 317: reception desk
    f.plants([(25, 49), (53, 49)])
    f.walls(54, 44, 59, 50, ":", "u")
    f.room("u", 28, "WC męski 2", "stall", gender="male", light="always")
    f.put(58, 45, 58, 45, "U")
    f.put(56, 45, 56, 45, "u")
    f.put(55, 49, 55, 49, "V")
    f.door(33, 38, 34, 38, "D", "K")                     # open space <-> passage
    f.door(24, 44, 24, 44, "D", "C")                     # 318
    f.area(24, 47, 24, 47, "k", "9")                     # 319
    f.door(28, 43, 28, 43, "D", "C")                     # stairwell
    f.door(35, 39, 35, 39, "x", "q")                     # 303: locked
    f.door(48, 43, 48, 43, "D", "C")                     # 315
    f.area(54, 47, 54, 47, "k", "u")                     # 316

    # The south wing: a corridor along the hall, the meeting rooms (nr 4, 3,
    # 1), the waiting area and a two-desk office off it; balcony 7 off the
    # big meeting room.
    f.area(9, 50, 61, 65, "#", "-")
    f.area(10, 51, 60, 53, ".", "D")
    f.room("D", 29, "Korytarz", "corridor", detector=True, light="always")
    f.door(30, 50, 31, 50, "D", "D")                     # hall <-> corridor
    f.door(46, 50, 47, 50, "D", "D")
    f.area(10, 55, 20, 64, ",", "d")
    f.room("d", 30, "Sala konferencyjna 4", "meeting", detector=True, light="switch", switch_door=(15, 54), windows=True)
    f.put(13, 58, 17, 59, "T")
    f.door(15, 54, 15, 54, "D", "D")
    f.area(22, 55, 30, 64, ",", "p")
    f.room("p", 31, "Poczekalnia", "common", detector=True, light="always", windows=True)
    f.put(23, 56, 25, 56, "Q")
    f.put(27, 56, 29, 56, "Q")
    f.put(25, 59, 27, 60, "T")
    f.put(23, 63, 25, 63, "Q")
    f.put(27, 63, 29, 63, "Q")
    f.plants([(22, 64), (30, 64)])
    f.door(25, 54, 26, 54, "D", "D")
    f.area(32, 55, 42, 64, ",", "c")
    f.room("c", 32, "Sala konferencyjna 3", "meeting", detector=True, light="switch", switch_door=(37, 54), windows=True)
    f.put(34, 58, 40, 59, "T")                           # 16 seats
    f.door(37, 54, 37, 54, "D", "D")
    f.area(33, 66, 41, 69, "h", "-")                     # balcony 7
    f.area(34, 66, 40, 68, "n", "f")
    f.room("f", 35, "Balkon 7", "balcony", outdoor=True)
    f.put(40, 68, 40, 68, "A")
    f.door(37, 65, 37, 65, "G", "c")
    f.area(44, 55, 50, 64, ",", "b")
    f.room("b", 33, "Pokój biurowy 2", "department", detector=True, light="switch", switch_door=(47, 54), windows=True)
    f.put(45, 58, 46, 58, "W")
    f.put(48, 58, 49, 58, "W")
    f.door(47, 54, 47, 54, "D", "D")
    f.area(52, 55, 60, 64, ",", "e")
    f.room("e", 34, "Sala konferencyjna 1", "meeting", detector=True, light="switch", switch_door=(56, 54), windows=True)
    f.put(54, 58, 57, 59, "T")
    f.door(56, 54, 56, 54, "D", "D")
    return f


def stairwell(floor, name):
    f = Floor(floor, "~")
    f.box(31, 4, 37, 15, ".", "P")
    f.room("P", 1, name, "stairs", light="always")
    f.area(*MID_FLIGHT_A, "s")
    f.area(*MID_FLIGHT_B, "s")
    f.area(34, 7, 34, 15, "#", "-")          # wall between the flights
    f.area(31, 15, 33, 15, "S")              # down to the lower floor
    f.area(35, 15, 37, 15, "S")              # on to the upper floor
    f.put(31, 4, 31, 4, "P")                 # a plant on the landing
    return f


def rect(r):
    x0, y0, x1, y1 = r
    return [x0, y0, x1 - x0 + 1, y1 - y0 + 1]


def links():
    lifts = [{"kind": "elevator", "id": "A", "area": rect(ELEV_A)}, {"kind": "elevator", "id": "B", "area": rect(ELEV_B)}]
    def stairs(area, to_floor, to):
        return {"kind": "stairs", "area": area, "to_floor": to_floor, "to": list(to)}

    return {
        0: lifts + [stairs(rect(STAIRS0), STAIRWELL_FLOOR, MID_ARRIVAL_A)],
        1: lifts + [
            stairs(rect(STAIRS1), STAIRWELL_FLOOR, MID_ARRIVAL_B),
            stairs(rect(STAIRS1_UP), STAIRWELL_UP_FLOOR, MID_ARRIVAL_A),
        ],
        3: lifts + [stairs(rect(STAIRS3), STAIRWELL_UP_FLOOR, MID_ARRIVAL_B)],
        STAIRWELL_FLOOR: [
            stairs([31, 15, 3, 1], 0, STAIRS0_ARRIVAL),
            stairs([35, 15, 3, 1], 1, STAIRS1_ARRIVAL),
        ],
        STAIRWELL_UP_FLOOR: [
            stairs([31, 15, 3, 1], 1, STAIRS1_UP_ARRIVAL),
            stairs([35, 15, 3, 1], 3, STAIRS3_ARRIVAL),
        ],
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
        "npcs": f.npcs,
        "places": f.places,
    }


def check(f):
    # Doors must connect two walkable tiles (catches walls drawn over doors).
    for y in range(1, H - 1):
        for x in range(1, W - 1):
            if f.t[y][x] in "DEGBgLZk":
                n = [f.t[y][x - 1], f.t[y][x + 1], f.t[y - 1][x], f.t[y + 1][x]]
                if sum(1 for c in n if not LEGEND[c]["solid"]) < 2:
                    sys.exit("floor %d: door at (%d,%d) leads nowhere" % (f.floor, x, y))
    # Every room key on the map has a definition and vice versa.
    keys = {c for row in f.r for c in row} - {"-"}
    missing = keys - set(f.rooms)
    if missing:
        sys.exit("floor %d: rooms without a definition: %s" % (f.floor, sorted(missing)))
    ids = [r["id"] for r in f.rooms.values()]
    if len(ids) != len(set(ids)):
        sys.exit("floor %d: duplicate room ids" % f.floor)


def main():
    floors = [
        floor0(),
        floor1(),
        floor3(),
        stairwell(STAIRWELL_FLOOR, "Półpiętro"),
        stairwell(STAIRWELL_UP_FLOOR, "Półpiętro (wyżej)"),
    ]
    for f in floors:
        f.finish()
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
            {"floor": 3, "file": "floor3.json", "name": "Piętro 3"},
            {"floor": 4, "file": "floor4.json", "name": "Klatka schodowa (półpiętro)", "stairwell": True},
            {"floor": 5, "file": "floor5.json", "name": "Klatka schodowa (półpiętro 1–3)", "stairwell": True},
        ],
    }
    with open(os.path.join(OUT, "building.json"), "w", encoding="utf-8") as fh:
        json.dump(building, fh, ensure_ascii=False, indent=1)
        fh.write("\n")
    print("written to", OUT)


if __name__ == "__main__":
    main()
