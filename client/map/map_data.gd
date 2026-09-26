## One floor of the building, loaded from the JSON file shared with the
## server (see server/src/map.rs). Collision, room zones and floor links must
## match exactly.
extends RefCounted

const NO_ROOM := 0

var id: String
var floor_index: int
var width: int
var height: int
var tile_px: int
var solid := PackedByteArray()
var tile_chars := PackedStringArray()
var room := PackedInt32Array()
var room_names := {}  # id -> name
var room_types := {}  # id -> type
var legend := {}      # char -> {type, solid, color, access?}
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
	var tiles: Array = data["tiles"]
	var rooms: Array = data["rooms"]
	if tiles.size() != height or rooms.size() != height:
		error = "row count mismatch"
		return
	solid.resize(width * height)
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
			room[i] = room_ids.get(rrow[x], NO_ROOM)


func is_blocked(tx: int, ty: int) -> bool:
	if tx < 0 or ty < 0 or tx >= width or ty >= height:
		return true
	return solid[ty * width + tx] != 0


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
