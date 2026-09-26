## One floor of the building, loaded from the JSON file shared with the
## server (see server/src/map.rs). Collision, room zones and floor links must
## match exactly.
extends RefCounted

const NO_ROOM := 0

# Access rights (bitmask, same as map::access in Rust).
const ACCESS_GUEST := 1
const ACCESS_CARD := 2
const ACCESS_SERVICE := 4
const ACCESS_BOARD := 8
const ACCESS_REQUIRED := {"card": ACCESS_GUEST | ACCESS_CARD, "service": ACCESS_SERVICE, "board": ACCESS_BOARD}

# Movement directions (map::dir in Rust).
const DIR_UP := 1
const DIR_DOWN := 2
const DIR_LEFT := 3
const DIR_RIGHT := 4
const DIR_NAMES := {"up": DIR_UP, "down": DIR_DOWN, "left": DIR_LEFT, "right": DIR_RIGHT}

var id: String
var floor_index: int
var width: int
var height: int
var tile_px: int
var solid := PackedByteArray()
var need := PackedByteArray()       # rights that open the tile (0 = none)
var free_dir := PackedByteArray()   # direction always passable (exit), 0 = none
var tile_chars := PackedStringArray()
var closed := {}                     # tile index -> true: locked stall doors (dynamic)
var room := PackedInt32Array()
var room_names := {}  # id -> name
var room_types := {}  # id -> type
var room_outdoor := {}  # id -> true: under the open sky (weather)
var legend := {}      # char -> {type, solid, color, access?, free_dir?}
## [{kind: "stairs"|"elevator", rect: Rect2i, id, to_floor, to: Vector2i}]
var links: Array = []
var spawns: Array[Vector2i] = []
var error := ""


## Parse a floor file; on failure `error` is non-empty.
func parse(bytes: PackedByteArray) -> void:
	var data = JSON.parse_string(bytes.get_string_from_utf8())
	if typeof(data) != TYPE_DICTIONARY:
		error = "invalid map json"
		return
	id = data["id"]
	floor_index = int(data["floor"])
	width = int(data["width"])
	height = int(data["height"])
	tile_px = int(data["tile_px"])
	legend = data["legend"]
	for l in data.get("links", []):
		var a: Array = l["area"]
		var link := {"kind": l["kind"], "rect": Rect2i(int(a[0]), int(a[1]), int(a[2]), int(a[3]))}
		if l["kind"] == "stairs":
			link.to_floor = int(l["to_floor"])
			link.to = Vector2i(int(l["to"][0]), int(l["to"][1]))
		else:
			link.id = l["id"]
		links.append(link)
	for sp in data.get("spawns", []):
		spawns.append(Vector2i(int(sp[0]), int(sp[1])))
	var defs: Dictionary = data["room_defs"]
	var room_ids := {}
	for key in defs:
		var rid := int(defs[key]["id"])
		room_ids[key] = rid
		room_names[rid] = defs[key]["name"]
		room_types[rid] = defs[key]["type"]
		if defs[key].get("outdoor", false):
			room_outdoor[rid] = true
	var tiles: Array = data["tiles"]
	var rooms: Array = data["rooms"]
	if tiles.size() != height or rooms.size() != height:
		error = "row count mismatch"
		return
	solid.resize(width * height)
	need.resize(width * height)
	free_dir.resize(width * height)
	room.resize(width * height)
	tile_chars.resize(width * height)
	for y in height:
		var trow: String = tiles[y]
		var rrow: String = rooms[y]
		if trow.length() != width or rrow.length() != width:
			error = "row %d has wrong width" % y
			return
		for x in width:
			var i := y * width + x
			var c := trow[x]
			if not legend.has(c):
				error = "tile '%s' not in legend" % c
				return
			tile_chars[i] = c
			solid[i] = 1 if legend[c]["solid"] else 0
			need[i] = ACCESS_REQUIRED.get(legend[c].get("access", ""), 0)
			free_dir[i] = DIR_NAMES.get(legend[c].get("free_dir", ""), 0)
			room[i] = room_ids.get(rrow[x], NO_ROOM)


## Solid tile (walls, furniture); ignores access rules.
func is_blocked(tx: int, ty: int) -> bool:
	if tx < 0 or ty < 0 or tx >= width or ty >= height:
		return true
	return solid[ty * width + tx] != 0


## Collision rule used by the simulation (mirror of Map::blocks in Rust):
## solid, or needs rights the character lacks - unless moving in the tile's
## free direction (leaving through the gates).
func blocks(tx: int, ty: int, access: int, dir: int) -> bool:
	if tx < 0 or ty < 0 or tx >= width or ty >= height:
		return true
	var i := ty * width + tx
	if solid[i] != 0 or closed.has(i):
		return true
	return need[i] != 0 and (access & need[i]) == 0 and free_dir[i] != dir


## Doors locked right now (toilet stalls), from the server's Doors packet.
func set_closed_tiles(tiles: Array) -> void:
	closed.clear()
	for t in tiles:
		closed[t.y * width + t.x] = true


func is_closed(tx: int, ty: int) -> bool:
	return closed.has(ty * width + tx)


func need_at(tx: int, ty: int) -> int:
	if tx < 0 or ty < 0 or tx >= width or ty >= height:
		return 0
	return need[ty * width + tx]


func room_at_tile(tx: int, ty: int) -> int:
	if tx < 0 or ty < 0 or tx >= width or ty >= height:
		return NO_ROOM
	return room[ty * width + tx]


func room_name(rid: int) -> String:
	return room_names.get(rid, "-")


## Link covering a tile, or {} if none.
func link_at(tx: int, ty: int) -> Dictionary:
	for l in links:
		if (l.rect as Rect2i).has_point(Vector2i(tx, ty)):
			return l
	return {}


static func crc32(data: PackedByteArray) -> int:
	var table := PackedInt64Array()
	table.resize(256)
	for i in 256:
		var c := i
		for k in 8:
			c = (0xEDB88320 ^ (c >> 1)) if (c & 1) else (c >> 1)
		table[i] = c
	var c32 := 0xFFFFFFFF
	for b in data:
		c32 = table[(c32 ^ b) & 0xFF] ^ (c32 >> 8)
	return c32 ^ 0xFFFFFFFF
