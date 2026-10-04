## Hand-drawn floor art (Don't Starve-ish, matching the characters and the
## UI): muted colours, ink outlines, slightly wobbly edges. Drawn with vector
## calls in world pixels (16 per tile) inside a SubViewport rendered once at
## a higher resolution (see map_view.gd), so it costs nothing per frame.
## Deterministic: same map -> same picture.
extends Node2D

const TP := 16.0
const INK := Color("#2a2118")
const OL := 0.6

var map


func paint(p_map) -> void:
	map = p_map
	queue_redraw()


func _draw() -> void:
	if map == null:
		return
	for y in map.height:
		for x in map.width:
			_ground(x, y)
	_wall_shadows()
	for y in map.height:
		for x in map.width:
			if _ch(x, y) == "#":
				_wall(x, y)
	_wall_edges()
	for y in map.height:
		for x in map.width:
			_tile_object(x, y)
	_props()


# ----------------------------------------------------------------- helpers

func _ch(x: int, y: int) -> String:
	if x < 0 or y < 0 or x >= map.width or y >= map.height:
		return "#"
	return map.tile_chars[y * map.width + x]


func _type_of(c: String) -> String:
	return map.legend[c]["type"] if map.legend.has(c) else ""


func _type(x: int, y: int) -> String:
	return _type_of(_ch(x, y))


func _solid(c: String) -> bool:
	return map.legend.has(c) and map.legend[c]["solid"]


func _is_wall(x: int, y: int) -> bool:
	return _ch(x, y) == "#"


func _room_type(x: int, y: int) -> String:
	return map.room_types.get(map.room_at_tile(x, y), "")


func _h(x: int, y: int, salt := 0) -> int:
	return absi((x * 73856093) ^ (y * 19349663) ^ (salt * 83492791))


## Deterministic 0..1 from a position.
func _rf(x: float, y: float, salt := 0) -> float:
	return float(_h(int(x * 7.0), int(y * 7.0), salt) % 1000) / 1000.0


func _wob(p: Vector2, amt := 0.35) -> Vector2:
	return p + Vector2(_rf(p.x, p.y, 1) - 0.5, _rf(p.x, p.y, 2) - 0.5) * amt * 2.0


## A filled polygon with a wobbly ink outline.
func _poly(pts: PackedVector2Array, fill: Color, ink := true, w := OL, amt := 0.35) -> void:
	var wp := PackedVector2Array()
	for p in pts:
		wp.append(_wob(p, amt))
	draw_colored_polygon(wp, fill)
	if ink:
		var closed := wp.duplicate()
		closed.append(wp[0])
		draw_polyline(closed, INK, w, true)


## Rounded rectangle (corner cut) with an ink outline.
func _box(r: Rect2, fill: Color, ink := true, c := 0.8, w := OL) -> void:
	var p := r.position
	var e := r.end
	var small := minf(r.size.x, r.size.y)
	c = clampf(c, 0.05, small * 0.4)
	var amt := minf(0.35, minf(c * 0.3, small / 10.0))  # small shapes stay simple polygons
	_poly(PackedVector2Array([
		Vector2(p.x + c, p.y), Vector2(e.x - c, p.y), Vector2(e.x, p.y + c), Vector2(e.x, e.y - c),
		Vector2(e.x - c, e.y), Vector2(p.x + c, e.y), Vector2(p.x, e.y - c), Vector2(p.x, p.y + c)]), fill, ink, w, amt)


func _disc(c: Vector2, r: float, fill: Color, ink := true, w := OL) -> void:
	if ink:
		draw_circle(c, r + w * 0.5, INK)
	draw_circle(c, r, fill)


func _stroke(a: Vector2, b: Vector2, col: Color, w := OL) -> void:
	draw_line(a, b, col, w, true)


func _tile_rect(x: int, y: int) -> Rect2:
	return Rect2(x * TP, y * TP, TP, TP)


# ------------------------------------------------------------------ ground

const DOOR_TYPES := ["door", "glass_door", "card_gate", "garage_gate", "elevator_door", "board_door", "stall_door", "service_door", "locked_door", "storeroom_door"]


func _is_door(x: int, y: int) -> bool:
	return _type(x, y) in DOOR_TYPES


## Floor under a solid object / a door: the most common walkable neighbour.
## A door takes its own room's floor, so a wide doorway is all one floor.
func _floor_under(x: int, y: int) -> String:
	var own: int = map.room_at_tile(x, y) if _is_door(x, y) else -1
	var counts := _floor_counts(x, y, own)
	if counts.is_empty() and own != -1:
		counts = _floor_counts(x, y, -1)
	var best := "."
	var best_n := 0
	for c in counts:
		if counts[c] > best_n:
			best_n = counts[c]
			best = c
	return best


## Walkable floors around a tile (`room` != -1: only that room's).
func _floor_counts(x: int, y: int, room: int) -> Dictionary:
	var counts := {}
	var around := [Vector2i(0, 1), Vector2i(0, -1), Vector2i(1, 0), Vector2i(-1, 0), Vector2i(1, 1), Vector2i(-1, -1)]
	if room != -1:
		around += [Vector2i(1, -1), Vector2i(-1, 1)]
	for d in around:
		var c := _ch(x + d.x, y + d.y)
		if map.legend.has(c) and not _solid(c) and _type_of(c) not in DOOR_TYPES + ["stairs"]:
			if room == -1 or map.room_at_tile(x + d.x, y + d.y) == room:
				counts[c] = counts.get(c, 0) + 1
	return counts


func _ground(x: int, y: int) -> void:
	var c := _ch(x, y)
	if c == "#":
		return
	var t := _type_of(c)
	if t in ["void"]:
		return
	if t == "fence":
		_floor(x, y, "grass")
		return
	if _solid(c) or t in ["door", "glass_door", "card_gate", "garage_gate", "board_door", "stall_door", "service_door", "locked_door", "storeroom_door"]:
		t = _type_of(_floor_under(x, y))
	_floor(x, y, t)


func _floor(x: int, y: int, t: String) -> void:
	var r := _tile_rect(x, y)
	var v := _h(x, y) % 4
	var o := r.position
	match t:
		"floor":  # warm wooden planks
			var base := Color("#b39b78").darkened(v * 0.008)
			draw_rect(r, base)
			for i in 4:
				var py := o.y + i * 4.0
				draw_line(Vector2(o.x, py), Vector2(o.x + TP, py), Color("#8f7658"), 0.35)
				var j := o.x + float((_h(x, y * 4 + i) % 12) + 2)
				draw_line(Vector2(j, py), Vector2(j, py + 4.0), Color("#8f7658"), 0.35)
			if v == 2:
				draw_line(o + Vector2(3, 6.5), o + Vector2(9, 6.8), Color("#a08868"), 0.4)
		"carpet":
			draw_rect(r, Color("#667a9b"))
			for i in 6:
				var p := o + Vector2(_rf(x, y, i) * TP, _rf(y, x, i + 9) * TP)
				draw_circle(p, 0.35, Color("#556889") if i % 2 else Color("#7a8db0"))
		"tiles":
			draw_rect(r, Color("#dcdcd4"))
			for k in [0.0, 8.0]:
				draw_line(o + Vector2(0, k), o + Vector2(TP, k), Color("#b6b8ae"), 0.35)
				draw_line(o + Vector2(k, 0), o + Vector2(k, TP), Color("#b6b8ae"), 0.35)
		"lobby":
			draw_rect(r, Color("#cdbd98").darkened(v * 0.008))
			draw_line(o, o + Vector2(TP, 0), Color("#ad9d78"), 0.45)
			draw_line(o, o + Vector2(0, TP), Color("#ad9d78"), 0.45)
			if v == 1:
				draw_line(o + Vector2(4, 11), o + Vector2(9, 6), Color(1, 1, 1, 0.25), 0.8)
		"parking":
			draw_rect(r, Color("#5b5954"))
			for i in 5:
				draw_circle(o + Vector2(_rf(x, y, i) * TP, _rf(y, x, i) * TP), 0.4, Color("#66645e") if i % 2 else Color("#4f4d49"))
		"street":
			draw_rect(r, Color("#4c4a46"))
			draw_line(o + Vector2(0, 0.4), o + Vector2(TP, 0.4), Color("#7c7870"), 0.8)
			draw_line(o + Vector2(0, TP - 0.4), o + Vector2(TP, TP - 0.4), Color("#7c7870"), 0.8)
			if x % 2 == 0:
				_box(Rect2(o + Vector2(3, 7.2), Vector2(9, 1.6)), Color("#e3dcc0"), false, 0.4)
		"tram_track":
			draw_rect(r, Color("#6b6259"))
			for sx in [2.0, 7.0, 12.0]:
				_box(Rect2(o + Vector2(sx, 2), Vector2(2, 12)), Color("#4a3b2b"), false, 0.3)
			for ry in [4.5, 11.5]:
				draw_line(o + Vector2(0, ry), o + Vector2(TP, ry), Color("#b8bcbf"), 1.0)
				draw_line(o + Vector2(0, ry + 0.8), o + Vector2(TP, ry + 0.8), Color("#6f757a"), 0.5)
		"grass":
			draw_rect(r, Color("#718640"))
			for i in 7:
				var p := o + Vector2(_rf(x, y, i + 20) * TP, 2.0 + _rf(y, x, i + 30) * (TP - 2.0))
				var col := Color("#58692f") if i % 3 else Color("#8a9d52")
				draw_line(p, p + Vector2(-0.5, -1.8), col, 0.45)
				draw_line(p, p + Vector2(0.6, -1.5), col, 0.45)
			if v == 3:
				draw_circle(o + Vector2(5, 6), 0.7, Color("#e8d36a"))
		"sidewalk":
			draw_rect(r, Color("#a19b8e"))
			var off := 4.0 if y % 2 else 0.0
			draw_line(o + Vector2(0, 8), o + Vector2(TP, 8), Color("#86806f"), 0.4)
			for k in [0.0, 8.0]:
				draw_line(o + Vector2(fmod(k + off, TP), 0), o + Vector2(fmod(k + off, TP), 8), Color("#86806f"), 0.4)
				draw_line(o + Vector2(fmod(k + off + 4.0, TP), 8), o + Vector2(fmod(k + off + 4.0, TP), TP), Color("#86806f"), 0.4)
		"smoking_area":
			draw_rect(r, Color("#8a7f6b"))
			for i in 8:
				draw_circle(o + Vector2(_rf(x, y, i) * TP, _rf(y, x, i) * TP), 0.55, [Color("#9c917b"), Color("#726857"), Color("#a69c86")][i % 3])
		"elevator":
			draw_rect(r, Color("#9aa3a8"))
			for py in range(2, 16, 4):
				for px in range(2 + (py / 4 % 2) * 2, 16, 4):
					draw_line(o + Vector2(px, py), o + Vector2(px + 1.5, py + 1.0), Color("#c4cbcf"), 0.5)
		"stairs", "steps":
			var bands := 4 if t == "stairs" else 3
			for i in bands:
				var h := TP / bands
				var br := Rect2(o + Vector2(0, i * h), Vector2(TP, h))
				draw_rect(br, Color("#a88b68").lightened(i * 0.04))
				draw_line(br.position, br.position + Vector2(TP, 0), Color("#6d5438"), 0.6)
				draw_line(br.position + Vector2(0, 0.8), br.position + Vector2(TP, 0.8), Color("#c9ae88"), 0.4)
		"balcony":
			draw_rect(r, Color("#8a6a4a"))
			for i in 4:
				draw_line(o + Vector2(0, i * 4.0), o + Vector2(TP, i * 4.0), Color("#5e4630"), 0.5)
		_:
			draw_rect(r, Color(map.legend[_ch(x, y)]["color"]) if map.legend.has(_ch(x, y)) else Color("#b39b78"))


# ------------------------------------------------------------------- walls

func _is_floorish(x: int, y: int) -> bool:
	var c := _ch(x, y)
	return c != "#" and _type_of(c) not in ["void", "fence", "service_door", "locked_door", "storeroom_door"] and map.legend.has(c)


## Outside: beyond the map, open air, a hedge, or an outdoor room.
func _outside(x: int, y: int) -> bool:
	var c := _ch(x, y)
	if x < 0 or y < 0 or x >= map.width or y >= map.height:
		return true
	if c == "#":
		return false
	if _type_of(c) in ["void", "fence"]:
		return true
	return map.room_outdoor.has(map.room_at_tile(x, y))


## A window in this wall tile? Returns the direction to the inside, or zero.
func _window_dir(x: int, y: int) -> Vector2i:
	for d in [Vector2i(0, 1), Vector2i(0, -1), Vector2i(1, 0), Vector2i(-1, 0)]:
		var ix: int = x + d.x
		var iy: int = y + d.y
		if _ch(ix, iy) == "#" or not _outside(x - d.x, y - d.y):
			continue
		if not map.room_windows.has(map.room_at_tile(ix, iy)) or map.room_outdoor.has(map.room_at_tile(ix, iy)):
			continue
		# Along a straight stretch of wall, with a pillar every 4 tiles.
		var along := Vector2i(d.y, d.x)
		if not (_is_wall(x + along.x, y + along.y) and _is_wall(x - along.x, y - along.y)):
			continue
		if (x if along.x != 0 else y) % 4 == 0:
			continue
		return d
	return Vector2i.ZERO


## 3/4 view walls: a dark top cap; where a room is below, a plaster face
## with a skirting board; windows in outside walls of rooms that have them.
func _wall(x: int, y: int) -> void:
	var r := _tile_rect(x, y)
	var o := r.position
	var face := _is_floorish(x, y + 1) and not _outside(x, y + 1)
	draw_rect(r, Color("#3b332d"))
	if _h(x, y, 3) % 4 == 0:
		draw_line(o + Vector2(4, 5), o + Vector2(7, 5.4), Color("#4a413a"), 0.5)
	if face:
		var fy := o.y + 7.0
		draw_rect(Rect2(o.x, fy, TP, 9), Color("#b7a88c"))
		draw_rect(Rect2(o.x, o.y + TP - 2.2, TP, 2.2), Color("#6e5b44"))
		draw_line(Vector2(o.x, fy), Vector2(o.x + TP, fy), INK, OL)
		draw_line(Vector2(o.x, o.y + TP - 2.2), Vector2(o.x + TP, o.y + TP - 2.2), Color(INK, 0.6), 0.4)
		if _h(x, y, 7) % 9 == 0 and _window_dir(x, y) == Vector2i.ZERO:  # a picture / notice
			_box(Rect2(o + Vector2(4, 8.5), Vector2(8, 4.5)), Color("#6d5236"), true, 0.4, 0.45)
			draw_rect(Rect2(o + Vector2(5, 9.5), Vector2(6, 2.5)), [Color("#8fb1c7"), Color("#d9bf6f"), Color("#9fbf86")][_h(x, y) % 3])
	var wd := _window_dir(x, y)
	if wd != Vector2i.ZERO:
		var glass := Color("#9fc0cf")
		var shine := Color(1, 1, 1, 0.45)
		if wd.y != 0:  # wall runs left-right
			var gy := o.y + (7.0 if wd.y > 0 and face else 5.0)
			var g := Rect2(o.x, gy, TP, 5.0 if face else 6.0)
			draw_rect(g, glass)
			draw_line(g.position + Vector2(2, 1.2), g.position + Vector2(6, 1.2), shine, 0.7)
			draw_rect(g, INK, false, OL)
			draw_line(Vector2(o.x + TP / 2, g.position.y), Vector2(o.x + TP / 2, g.end.y), INK, 0.5)
		else:  # wall runs up-down
			var g := Rect2(o.x + 5.0, o.y, 6.0, TP)
			draw_rect(g, glass)
			draw_line(g.position + Vector2(1.2, 2), g.position + Vector2(1.2, 6), shine, 0.7)
			draw_rect(g, INK, false, OL)
			draw_line(Vector2(g.position.x, o.y + TP / 2), Vector2(g.end.x, o.y + TP / 2), INK, 0.5)


## Ink where walls meet anything else.
func _wall_edges() -> void:
	for y in map.height:
		for x in map.width:
			if _ch(x, y) != "#":
				continue
			var o := Vector2(x, y) * TP
			if not _is_wall(x, y - 1):
				draw_line(_wob(o), _wob(o + Vector2(TP, 0)), INK, OL * 1.4, true)
			if not _is_wall(x, y + 1):
				draw_line(_wob(o + Vector2(0, TP)), _wob(o + Vector2(TP, TP)), INK, OL * 1.4, true)
			if not _is_wall(x - 1, y):
				draw_line(_wob(o), _wob(o + Vector2(0, TP)), INK, OL * 1.4, true)
			if not _is_wall(x + 1, y):
				draw_line(_wob(o + Vector2(TP, 0)), _wob(o + Vector2(TP, TP)), INK, OL * 1.4, true)


func _wall_shadows() -> void:
	for y in map.height:
		for x in map.width:
			if _is_wall(x, y) or _type(x, y) in ["void", "fence"]:
				continue
			var o := Vector2(x, y) * TP
			if _is_wall(x, y - 1):
				draw_rect(Rect2(o, Vector2(TP, 2.5)), Color(0, 0, 0, 0.2))
				draw_rect(Rect2(o + Vector2(0, 2.5), Vector2(TP, 2.0)), Color(0, 0, 0, 0.08))
			if _is_wall(x - 1, y):
				draw_rect(Rect2(o, Vector2(2.0, TP)), Color(0, 0, 0, 0.12))


# ------------------------------------------------------------ tile objects

## Doors, gates, hedges, railings: things drawn per tile.
func _tile_object(x: int, y: int) -> void:
	var t := _type(x, y)
	var r := _tile_rect(x, y)
	var o := r.position
	match t:
		"door", "board_door":
			var across := _is_wall(x - 1, y) or _is_wall(x + 1, y) or _type(x - 1, y) == t or _type(x + 1, y) == t
			var wood := Color("#8a5a36") if t == "door" else Color("#5e2f22")
			if across:
				_box(Rect2(o + Vector2(0, 6), Vector2(TP, 4)), Color("#a6764c"), true, 0.3, 0.45)
				if _is_wall(x - 1, y):
					_box(Rect2(o, Vector2(2.2, TP)), wood, true, 0.3, 0.45)
				if _is_wall(x + 1, y):
					_box(Rect2(o + Vector2(TP - 2.2, 0), Vector2(2.2, TP)), wood, true, 0.3, 0.45)
			else:
				_box(Rect2(o + Vector2(6, 0), Vector2(4, TP)), Color("#a6764c"), true, 0.3, 0.45)
				if _is_wall(x, y - 1):
					_box(Rect2(o, Vector2(TP, 2.2)), wood, true, 0.3, 0.45)
				if _is_wall(x, y + 1):
					_box(Rect2(o + Vector2(0, TP - 2.2), Vector2(TP, 2.2)), wood, true, 0.3, 0.45)
		"glass_door":
			var g := Rect2(o + Vector2(0, 5), Vector2(TP, 6))
			draw_rect(g, Color("#a9d4e0", 0.85))
			draw_line(g.position + Vector2(2, 1.2), g.position + Vector2(7, 1.2), Color(1, 1, 1, 0.6), 0.7)
			draw_rect(g, INK, false, OL)
			draw_line(Vector2(o.x + TP / 2, g.position.y), Vector2(o.x + TP / 2, g.end.y), INK, 0.5)
		"card_gate":
			_box(Rect2(o + Vector2(0, 2), Vector2(3, 12)), Color("#6f7278"))
			_box(Rect2(o + Vector2(TP - 3, 2), Vector2(3, 12)), Color("#6f7278"))
			_box(Rect2(o + Vector2(3, 7), Vector2(7, 2)), Color("#d4a94a"), true, 0.3, 0.45)
			draw_circle(o + Vector2(1.5, 4), 0.6, Color("#6fd06b"))
		"garage_gate":
			for i in 5:
				draw_rect(Rect2(o + Vector2(i * 3.2, 0), Vector2(3.2, 3)), Color("#d4a94a") if i % 2 == 0 else Color("#2b2522"))
			draw_rect(Rect2(o, Vector2(TP, 3)), INK, false, 0.45)
		"service_door":
			_box(r.grow(-0.3), Color("#6b4128"))
			_box(Rect2(o + Vector2(4, 4), Vector2(8, 3)), Color("#a8402f"), true, 0.3, 0.45)
			draw_circle(o + Vector2(11, 9), 0.8, Color("#d4b870"))
		"storeroom_door":  # the storeroom: a door with a keyhole (the key is at the reception)
			_box(r.grow(-0.3), Color("#7a5a3a"))
			_box(Rect2(o + Vector2(3, 3), Vector2(10, 10)), Color("#8f6c48"), true, 0.3, 0.45)
			draw_circle(o + Vector2(8, 7.5), 1.3, INK)
			draw_rect(Rect2(o + Vector2(7.5, 7.5), Vector2(1, 2.6)), INK)
		"locked_door":  # shut for good: dark door, red band, a padlock
			_box(r.grow(-0.3), Color("#4a2f24"))
			_box(Rect2(o + Vector2(2, 6.5), Vector2(12, 3)), Color("#b0382c"), true, 0.3, 0.45)
			_box(Rect2(o + Vector2(6, 9.5), Vector2(4, 3.5)), Color("#d4b870"), true, 0.3, 0.45)
			draw_arc(o + Vector2(8, 9.5), 1.6, PI, TAU, 8, INK, 0.7)
		"fence":
			for i in 3:
				var c := o + Vector2(3 + i * 5.0, 8 + (i % 2) * 2.0)
				_disc(c, 4.2, Color("#3f5a2c").lerp(Color("#4d6a33"), _rf(x, y, i)), true, 0.5)
				draw_circle(c + Vector2(-1.2, -1.4), 1.4, Color("#5f7f3e"))
		"railing":
			var horiz := _type(x - 1, y) == "railing" or _type(x + 1, y) == "railing"
			var vert := _type(x, y - 1) == "railing" or _type(x, y + 1) == "railing"
			if horiz:
				for px in [2.0, 7.0, 12.0]:
					_stroke(o + Vector2(px, 8), o + Vector2(px, 14), Color("#2f2f36"), 1.0)
				_box(Rect2(o + Vector2(0, 5.5), Vector2(TP, 2.2)), Color("#44444e"), true, 0.3, 0.45)
			if vert:
				_box(Rect2(o + Vector2(6.5, 0), Vector2(2.2, TP)), Color("#44444e"), true, 0.3, 0.45)


# ------------------------------------------------------------------- props

const PROP_TYPES := ["desk", "counter", "shelf", "sofa", "table", "plant", "rack", "bench", "ashtray", "toilet", "sink",
	"car", "coffee_machine", "kitchen_counter", "fruit_bowl", "partition", "sanitizer", "bike_rack",
	"cupboard", "dishwasher", "kitchen_sink", "fridge", "urinal", "wardrobe", "bin", "armchair", "tv", "medicine_cabinet", "key_hook", "liquor_cabinet"]


## Connected tiles of the same furniture char = one object.
func _props() -> void:
	var seen := {}
	var index := 0
	for y in map.height:
		for x in map.width:
			var c := _ch(x, y)
			if _type_of(c) not in PROP_TYPES or seen.has(Vector2i(x, y)):
				continue
			var tiles: Array[Vector2i] = []
			var stack: Array[Vector2i] = [Vector2i(x, y)]
			seen[Vector2i(x, y)] = true
			while not stack.is_empty():
				var t: Vector2i = stack.pop_back()
				tiles.append(t)
				for d in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
					var n: Vector2i = t + d
					if not seen.has(n) and _ch(n.x, n.y) == c:
						seen[n] = true
						stack.append(n)
			var tr := Rect2i(tiles[0], Vector2i.ONE)
			for t in tiles:
				tr = tr.expand(t).expand(t + Vector2i.ONE)
			_prop(_type_of(c), tr, index)
			index += 1


func _shadow(r: Rect2) -> void:
	draw_rect(Rect2(r.position + Vector2(1.2, 1.6), r.size), Color(0, 0, 0, 0.2))


const KITCHEN := ["coffee_machine", "kitchen_counter", "cupboard", "dishwasher", "kitchen_sink", "fridge"]


func _prop(t: String, tr: Rect2i, index: int) -> void:
	var r := Rect2(Vector2(tr.position) * TP, Vector2(tr.size) * TP)
	var turn := _turn_from_wall(tr) if t in KITCHEN else 0.0
	if turn != 0.0:
		# Drawn as if against the north wall, then turned to face the room.
		var size := r.size if is_equal_approx(absf(turn), PI) else Vector2(r.size.y, r.size.x)
		draw_set_transform(r.get_center(), turn, Vector2.ONE)
		r = Rect2(-size / 2, size)
	match t:
		"desk": _desks(tr, r, index)
		"counter": _counter(r)
		"shelf": _shelf(r, index)
		"sofa": _sofa(r)
		"armchair": _armchair(r)
		"tv": _tv(r)
		"medicine_cabinet": _medicine_cabinet(r)
		"key_hook": _key_hook(r)
		"liquor_cabinet": _liquor_cabinet(r)
		"table": _table(tr, r)
		"plant": _plant(r, index)
		"rack": _racks(r, index)
		"bench": _bench(r)
		"ashtray": _ashtray(r)
		"toilet": _toilet(tr, r)
		"sink": _sink(tr, r)
		"car": _car(r, index)
		"coffee_machine": _coffee_machine(r)
		"kitchen_counter": _kitchen_counter(r)
		"fruit_bowl": _fruit_bowl(r)
		"partition": _partition(r)
		"sanitizer": _sanitizer(r)
		"bike_rack": _bike_rack(r)
		"cupboard": _cupboard(r)
		"dishwasher": _dishwasher(r)
		"kitchen_sink": _kitchen_sink(r)
		"fridge": _fridge(r)
		"urinal": _urinal(tr, r)
		"wardrobe": _wardrobe(r)
		"bin": _bin(r)
	if turn != 0.0:
		draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


## Kitchen units are drawn with their back to the north wall; this is the
## turn for one standing by another wall (north first, then south, west, east).
func _turn_from_wall(tr: Rect2i) -> float:
	var sides := [[Vector2i(0, -1), 0.0], [Vector2i(0, 1), PI], [Vector2i(-1, 0), -PI / 2], [Vector2i(1, 0), PI / 2]]
	for s in sides:
		var d: Vector2i = s[0]
		var all := true
		for y in range(tr.position.y, tr.end.y):
			for x in range(tr.position.x, tr.end.x):
				var n := Vector2i(x, y) + d
				if not tr.has_point(n) and not _is_wall(n.x, n.y):
					all = false
		if all:
			return s[1]
	return 0.0


func _chair(c: Vector2, facing_up: bool) -> void:
	_disc(c + Vector2(0, 0.5), 3.4, Color("#3b3a42"))
	_box(Rect2(c + Vector2(-3.6, (-4.6 if not facing_up else 2.4)), Vector2(7.2, 2.4)), Color("#2c2b31"), true, 0.8)


func _desks(tr: Rect2i, r: Rect2, index: int) -> void:
	var top := Rect2(r.position + Vector2(0.8, 2), Vector2(r.size.x - 1.6, r.size.y - 5))
	_shadow(top)
	_box(Rect2(top.position.x, top.end.y - 1, top.size.x, 3), Color("#6e4f33"), true, 0.4)
	_box(top, Color("#a8804f"), true, 1.0)
	draw_line(top.position + Vector2(1.5, 1.2), Vector2(top.end.x - 1.5, top.position.y + 1.2), Color("#c29a68"), 0.6)
	var screens := [Color("#7fb3cf"), Color("#8fc493"), Color("#d9bf6f"), Color("#b39ac9")]
	for i in tr.size.x:
		var tx := tr.position.x + i
		var px := tx * TP
		var py := r.position.y
		if _room_type(tx, tr.position.y) == "department":
			_box(Rect2(px + 3, py + 4, 10, 6.5), Color("#8b6a47"), true, 0.8, 0.4)  # desk mat
		else:
			_box(Rect2(px + 3, py + 2.5, 10, 6), Color("#2b2a30"), true, 0.6)
			draw_rect(Rect2(px + 4.2, py + 3.6, 7.6, 3.8), screens[_h(tx, tr.position.y, index) % screens.size()])
			_box(Rect2(px + 4, py + 10, 8, 2), Color("#dcd8cc"), true, 0.4, 0.4)
		if _h(tx, tr.position.y, 5) % 4 == 0:
			_box(Rect2(px + 12.5, py + 9, 2.2, 3), Color("#ece4d2"), true, 0.3, 0.35)  # papers
		var below := _ch(tx, tr.end.y)
		if map.legend.has(below) and not _solid(below):
			_chair(Vector2(px + 8, tr.end.y * TP + 5), false)


func _counter(r: Rect2) -> void:
	var top := Rect2(r.position + Vector2(0.8, 1), r.size - Vector2(1.6, 4))
	_shadow(top)
	_box(Rect2(top.position.x, top.end.y - 1, top.size.x, 3.2), Color("#8a6743"), true, 0.4)
	_box(top, Color("#c7a276"), true, 1.0)
	_box(Rect2(top.position + Vector2(3, 2), Vector2(7, 5)), Color("#2b2a30"), true, 0.5)
	draw_rect(Rect2(top.position + Vector2(4, 3), Vector2(5, 3)), Color("#7fb3cf"))
	_disc(Vector2(top.end.x - 4.5, top.position.y + 5), 1.2, Color("#d4b870"), true, 0.4)


func _shelf(r: Rect2, index: int) -> void:
	var body := Rect2(r.position + Vector2(0, 1), r.size - Vector2(0, 2))
	_shadow(body)
	_box(body, Color("#6e6a63"), true, 0.8)
	var goods := [Color("#c9553f"), Color("#d9b650"), Color("#5a8fc2"), Color("#6fa35a"), Color("#d98a3e"), Color("#8e6aa8"), Color("#ece6d6")]
	var n := 0
	var x := body.position.x + 1.5
	while x < body.end.x - 2.5:
		for row in 2:
			var c: Color = goods[(_h(int(x), row, index) + n) % goods.size()]
			var g := Rect2(x, body.position.y + 1.8 + row * 6, 2.6, 4.4)
			draw_rect(g, c)
			draw_rect(Rect2(g.position, Vector2(g.size.x, 0.8)), c.lightened(0.3))
			n += 1
		x += 3.0


func _sofa(r: Rect2) -> void:
	var c := Color("#6a7fa8")
	_shadow(r)
	_box(Rect2(r.position + Vector2(0, 1), r.size - Vector2(0, 2)), c.darkened(0.25), true, 2.0)
	_box(Rect2(r.position + Vector2(3, 6), r.size - Vector2(6, 9)), c, true, 1.5)
	var n := int(r.size.x / TP)
	for i in range(1, n):
		draw_line(Vector2(r.position.x + i * TP, r.position.y + 7), Vector2(r.position.x + i * TP, r.end.y - 4), c.darkened(0.2), 0.5)


## A deep old armchair (Paulina's): a high back, two armrests, a cushion.
func _armchair(r: Rect2) -> void:
	var c := Color("#8a4a5a")
	_shadow(r)
	_box(Rect2(r.position + Vector2(1, 0), Vector2(r.size.x - 2, 6)), c.darkened(0.25), true, 2.0)
	_box(Rect2(r.position + Vector2(0, 3), Vector2(3.5, r.size.y - 4)), c.darkened(0.15), true, 1.2)
	_box(Rect2(Vector2(r.end.x - 3.5, r.position.y + 3), Vector2(3.5, r.size.y - 4)), c.darkened(0.15), true, 1.2)
	_box(Rect2(r.position + Vector2(3.5, 5), Vector2(r.size.x - 7, r.size.y - 7)), c, true, 1.2)


## The first-aid cabinet: a white box with a red cross.
func _medicine_cabinet(r: Rect2) -> void:
	var b := r.grow(-1.5)
	_shadow(b)
	_box(b, Color("#f4f6f8"), true, 1.0)
	var c := b.get_center()
	draw_rect(Rect2(c - Vector2(1.2, 3.6), Vector2(2.4, 7.2)), Color("#d9443a"))
	draw_rect(Rect2(c - Vector2(3.6, 1.2), Vector2(7.2, 2.4)), Color("#d9443a"))


## The liquor cabinet: dark wood, glass doors, bottles behind them, a keyhole.
func _liquor_cabinet(r: Rect2) -> void:
	var b := r.grow(-1.0)
	_shadow(b)
	_box(b, Color("#5e2f22"), true, 1.0)
	var g := Rect2(b.position + Vector2(1.5, 1.5), b.size - Vector2(3, 5))
	draw_rect(g, Color("#a9d4e0", 0.5))
	for i in 3:
		draw_rect(Rect2(g.position + Vector2(1.5 + i * 3.5, 2.5), Vector2(2, 5)), [Color("#b8742a"), Color("#dfe8ee"), Color("#7a3a1a")][i])
	draw_circle(Vector2(b.get_center().x, b.end.y - 1.8), 0.6, Color("#d4b870"))


## The key hook: a little board, and the key on it (the server knows if it's there).
func _key_hook(r: Rect2) -> void:
	var b := Rect2(r.position + Vector2(3, 3), Vector2(10, 7))
	_box(b, Color("#8a6a45"), true, 0.6)
	draw_circle(b.position + Vector2(5, 2.5), 0.7, Color("#d4b870"))
	draw_line(b.position + Vector2(5, 3), b.position + Vector2(5, 7.5), Color("#d4b870"), 1.0)
	draw_line(b.position + Vector2(5, 6.5), b.position + Vector2(6.5, 6.5), Color("#d4b870"), 0.8)


## The TV's wall bracket; the set itself (and what's on) is game/tv_view.gd.
func _tv(r: Rect2) -> void:
	draw_rect(r, Color("#3b3b4f"))
	draw_rect(Rect2(r.get_center() - Vector2(4, 2), Vector2(8, 4)), Color("#2b2b33"))


func _table(tr: Rect2i, r: Rect2) -> void:
	for i in tr.size.x:
		var tx := tr.position.x + i
		for side in [-1, 1]:
			var ty := tr.position.y - 1 if side < 0 else tr.end.y
			var tc := _ch(tx, ty)
			if map.legend.has(tc) and not _solid(tc):
				var cy := ty * TP + (11.0 if side < 0 else 4.0)
				_box(Rect2(tx * TP + 4, cy - 3, 8, 6), Color("#6f513a"), true, 1.2)
	var top := Rect2(r.position + Vector2(0.8, 1), r.size - Vector2(1.6, 3))
	_shadow(top)
	_box(top, Color("#8f6e4c"), true, 1.2)
	draw_line(top.position + Vector2(2, 1.2), Vector2(top.end.x - 2, top.position.y + 1.2), Color("#a8865f"), 0.6)
	_box(Rect2(top.get_center() - Vector2(2, 1.5), Vector2(4, 3)), Color("#ece4d2"), true, 0.3, 0.35)


func _plant(r: Rect2, index: int) -> void:
	var c := r.position + Vector2(8, 13)
	draw_rect(Rect2(c + Vector2(-4, 0.5), Vector2(9, 1.8)), Color(0, 0, 0, 0.2))
	_poly(PackedVector2Array([c + Vector2(-3.5, -5), c + Vector2(3.5, -5), c + Vector2(2.6, 0.5), c + Vector2(-2.6, 0.5)]), Color("#b0643c"))
	var rr := RandomNumberGenerator.new()
	rr.seed = 91 + index
	for i in 7:
		var a := -PI / 2 + (i - 3) * 0.45
		var tip := c + Vector2(0, -5) + Vector2(cos(a), sin(a)) * (6.0 + rr.randf() * 2.0)
		var mid := (c + Vector2(0, -5) + tip) / 2.0 + Vector2(cos(a + PI / 2), sin(a + PI / 2)) * 1.4
		_poly(PackedVector2Array([c + Vector2(0, -5), mid, tip, (c + Vector2(0, -5) + tip) / 2.0 - Vector2(cos(a + PI / 2), sin(a + PI / 2)) * 1.4]),
			[Color("#4f7a3a"), Color("#638f45"), Color("#3f6530")][i % 3], true, 0.4)


func _racks(r: Rect2, index: int) -> void:
	_shadow(r)
	_box(r, Color("#26262b"), true, 0.8)
	var ty := r.position.y
	while ty < r.end.y:
		var tx := r.position.x
		while tx < r.end.x:
			for row in range(3, 14, 3):
				draw_line(Vector2(tx + 2.5, ty + row), Vector2(tx + 11, ty + row), Color("#3a3a42"), 0.8)
				draw_circle(Vector2(tx + 12.8, ty + row), 0.55, Color("#6fd06b") if _h(int(tx), int(ty) + row, index) % 3 else Color("#7fb3cf"))
			tx += TP
		ty += TP


func _bench(r: Rect2) -> void:
	_shadow(Rect2(r.position + Vector2(0, 4), Vector2(r.size.x, 8)))
	for i in 3:
		_box(Rect2(r.position.x, r.position.y + 4 + i * 3, r.size.x, 2.2), Color("#9a7550"), true, 0.5, 0.45)
	_stroke(r.position + Vector2(3, 12), r.position + Vector2(3, 15), Color("#3d3833"), 1.2)
	_stroke(Vector2(r.end.x - 3, r.position.y + 12), Vector2(r.end.x - 3, r.position.y + 15), Color("#3d3833"), 1.2)


func _ashtray(r: Rect2) -> void:
	var c := r.position + Vector2(8, 8)
	draw_rect(Rect2(c + Vector2(-3, 5), Vector2(8, 1.8)), Color(0, 0, 0, 0.25))
	_box(Rect2(c + Vector2(-3, -5), Vector2(6, 11)), Color("#7a7c80"), true, 1.2)
	_disc(c + Vector2(0, -4.5), 2.4, Color("#c4bca5"), true, 0.45)
	_stroke(c + Vector2(-1, -4.8), c + Vector2(1.2, -4.2), Color("#ece6d6"), 0.7)


func _toilet(tr: Rect2i, r: Rect2) -> void:
	var left_wall := _is_wall(tr.position.x - 1, tr.position.y)
	var tank_x := r.position.x + (1.0 if left_wall else 11.0)
	_box(Rect2(tank_x, r.position.y + 3, 4, 10), Color("#e8e8e2"), true, 0.8)
	var bx := r.position.x + (5.0 if left_wall else 3.0)
	var c := Vector2(bx + 4, r.position.y + 8)
	draw_set_transform(c, 0.0, Vector2(1.0, 0.8))
	_disc(Vector2.ZERO, 4.4, Color("#f4f4ee"))
	draw_circle(Vector2.ZERO, 2.4, Color("#c4d8e0"))
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


func _sink(tr: Rect2i, r: Rect2) -> void:
	var left_wall := _is_wall(tr.position.x - 1, tr.position.y)
	var ty := r.position.y
	while ty < r.end.y:
		var x0 := r.position.x + (1.0 if left_wall else 4.0)
		_box(Rect2(x0, ty + 3, 11, 10), Color("#eef0ec"), true, 2.0)
		draw_set_transform(Vector2(x0 + 5.5, ty + 8), 0.0, Vector2(1.0, 0.8))
		draw_circle(Vector2.ZERO, 3.2, Color("#bcd0da"))
		draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
		_stroke(Vector2(x0 + (1.0 if left_wall else 10.0), ty + 8), Vector2(x0 + (3.0 if left_wall else 8.0), ty + 8), Color("#8f969b"), 0.9)
		var mx := r.position.x + (0.4 if left_wall else 15.0)
		_box(Rect2(mx, ty + 2, 0.9, 12), Color("#cfe6ef"), true, 0.2, 0.35)  # mirror
		ty += TP


func _car(r: Rect2, index: int) -> void:
	var colors := [Color("#a8402f"), Color("#3f6a9e"), Color("#e6e0d0"), Color("#34312e"), Color("#8e948f"), Color("#5f8a4a"), Color("#c9913a")]
	var body: Color = colors[index % colors.size()]
	var line := Color(0.92, 0.9, 0.84, 0.7)
	draw_line(r.position + Vector2(0, 1), Vector2(r.end.x, r.position.y + 1), line, 0.6)
	draw_line(Vector2(r.position.x, r.end.y - 1.5), Vector2(r.end.x, r.end.y - 1.5), line, 0.6)
	draw_line(r.position + Vector2(1, 1), Vector2(r.position.x + 1, r.end.y - 1.5), line, 0.6)
	var b := Rect2(r.position + Vector2(2, 4), r.size - Vector2(4, 8))
	draw_rect(Rect2(b.position + Vector2(2, 3), b.size), Color(0, 0, 0, 0.22))
	for wx in [b.position.x + 6, b.end.x - 12]:
		_box(Rect2(wx, b.position.y - 2, 6, 3), Color("#1e1b19"), true, 0.8, 0.4)
		_box(Rect2(wx, b.end.y - 1, 6, 3), Color("#1e1b19"), true, 0.8, 0.4)
	_box(b, body, true, 3.0)
	var roof := Rect2(b.position + Vector2(12, 3), b.size - Vector2(22, 6))
	_box(roof, body.darkened(0.12), true, 2.0, 0.45)
	_box(Rect2(roof.end.x, roof.position.y, 5, roof.size.y), Color("#3e5266"), true, 1.0, 0.45)
	_box(Rect2(roof.position.x - 4, roof.position.y, 4, roof.size.y), Color("#3e5266"), true, 1.0, 0.45)
	draw_circle(Vector2(b.end.x - 1.2, b.position.y + 3.5), 0.9, Color("#f2e3a0"))
	draw_circle(Vector2(b.end.x - 1.2, b.end.y - 3.5), 0.9, Color("#f2e3a0"))
	draw_circle(Vector2(b.position.x + 1.2, b.position.y + 3.5), 0.9, Color("#c94a3a"))
	draw_circle(Vector2(b.position.x + 1.2, b.end.y - 3.5), 0.9, Color("#c94a3a"))


func _coffee_machine(r: Rect2) -> void:
	var x := r.position.x + 2
	var y := r.position.y
	draw_rect(Rect2(x + 2, y + 13, 12, 2), Color(0, 0, 0, 0.25))
	_box(Rect2(x, y + 1, 12, 13), Color("#2e2b2c"), true, 1.2)
	_box(Rect2(x + 1, y + 2, 10, 3), Color("#6b4a2e"), true, 0.8, 0.4)
	_box(Rect2(x + 1, y + 6, 10, 6), Color("#b8bdbf"), true, 0.8, 0.4)
	_box(Rect2(x + 4, y + 8.5, 4, 3), Color("#f1ece2"), true, 0.4, 0.35)
	draw_circle(Vector2(x + 10, y + 7.5), 0.6, Color("#d24b3a"))


func _kitchen_counter(r: Rect2) -> void:
	var top := Rect2(r.position + Vector2(0, 1), r.size - Vector2(0, 4))
	_shadow(top)
	_box(Rect2(top.position.x, top.end.y - 1, top.size.x, 3.2), Color("#8b8272"), true, 0.4)
	_box(top, Color("#d6cfbf"), true, 0.8)
	for i in 3:
		var mug: Color = [Color("#f1ece2"), Color("#4f7fb0"), Color("#d98a3e")][i]
		_disc(top.position + Vector2(4.5 + i * 4, 5.5), 1.5, mug, true, 0.4)
	if r.size.x >= 32:  # a chopping board
		_box(Rect2(top.position + Vector2(19, 2), Vector2(9, 6)), Color("#c79a62"), true, 1.0)


func _fruit_bowl(r: Rect2) -> void:
	var top := Rect2(r.position + Vector2(0, 1), r.size - Vector2(0, 4))
	_shadow(top)
	_box(Rect2(top.position.x, top.end.y - 1, top.size.x, 3.2), Color("#8b8272"), true, 0.4)
	_box(top, Color("#d6cfbf"), true, 0.8)
	var c := r.position + Vector2(8, 7)
	for f in [[-3, -1, "#c9553f"], [0, -2, "#d9b650"], [3, -1, "#6fa35a"], [-1.5, 1, "#d98a3e"], [1.5, 1, "#b4412f"]]:
		_disc(c + Vector2(f[0], f[1]), 1.6, Color(f[2]), true, 0.35)
	_poly(PackedVector2Array([c + Vector2(-5.5, 0.5), c + Vector2(5.5, 0.5), c + Vector2(3.5, 3.5), c + Vector2(-3.5, 3.5)]), Color("#9c6e3a"), true, 0.45)


func _partition(r: Rect2) -> void:
	var body := Rect2(r.position, r.size - Vector2(0, 3))
	_shadow(body)
	_box(Rect2(r.position.x, r.end.y - 3.5, r.size.x, 3.5), Color("#8f959b"), true, 0.3, 0.45)
	_box(body, Color("#c7ccce"), true, 0.4, 0.5)


func _sanitizer(r: Rect2) -> void:
	var p := r.position + Vector2(4, 2)
	draw_rect(Rect2(p + Vector2(1, 1), Vector2(8, 11)), Color(0, 0, 0, 0.18))
	_box(Rect2(p, Vector2(8, 10)), Color("#f1f1ea"), true, 1.2)
	_box(Rect2(p + Vector2(2, 3), Vector2(4, 3)), Color("#4f86c0"), true, 0.3, 0.35)
	_stroke(p + Vector2(4, 10), p + Vector2(4, 11.5), Color("#9aa3a8"), 1.0)


func _bike_rack(r: Rect2) -> void:
	var tx := r.position.x
	while tx < r.end.x:
		var y := r.position.y
		draw_rect(Rect2(tx + 3, y + 12, 11, 2), Color(0, 0, 0, 0.2))
		draw_arc(Vector2(tx + 8, y + 8), 3.6, PI, TAU, 10, INK, 2.0, true)
		draw_arc(Vector2(tx + 8, y + 8), 3.6, PI, TAU, 10, Color("#aab0b3"), 1.1, true)
		_stroke(Vector2(tx + 4.4, y + 8), Vector2(tx + 4.4, y + 13), Color("#aab0b3"), 1.1)
		_stroke(Vector2(tx + 11.6, y + 8), Vector2(tx + 11.6, y + 13), Color("#aab0b3"), 1.1)
		tx += TP


## Kitchenette: worktop units along the wall (seen from above, front edge
## towards the room).
func _worktop(r: Rect2, fill: Color) -> Rect2:
	var top := Rect2(r.position + Vector2(0, 0.5), r.size - Vector2(0, 3.5))
	_shadow(top)
	_box(Rect2(top.position.x, top.end.y - 1, top.size.x, 3.2), fill.darkened(0.3), true, 0.4)
	_box(top, fill, true, 0.8)
	return top


func _cupboard(r: Rect2) -> void:
	var top := _worktop(r, Color("#9c7650"))
	# Open shelf of mugs seen from above: a row of round mugs.
	for i in 3:
		_disc(top.position + Vector2(3.5 + i * 4.5, 5.5), 1.6, [Color("#f1ece2"), Color("#4f7fb0"), Color("#d98a3e")][i], true, 0.4)
	draw_line(top.position + Vector2(2, 9.5), top.position + Vector2(top.size.x - 2, 9.5), INK, 0.5)


func _dishwasher(r: Rect2) -> void:
	var top := _worktop(r, Color("#bfc4c6"))
	_box(Rect2(top.position + Vector2(2, 2), Vector2(top.size.x - 4, 3)), Color("#8f959b"), true, 0.3, 0.45)  # control panel
	draw_circle(top.position + Vector2(top.size.x - 4, 3.5), 0.6, Color("#6fd06b"))
	draw_line(top.position + Vector2(3, 9), top.position + Vector2(top.size.x - 3, 9), Color("#8f959b"), 1.0)  # handle


func _kitchen_sink(r: Rect2) -> void:
	var top := _worktop(r, Color("#d6cfbf"))
	_box(Rect2(top.position + Vector2(2.5, 2.5), Vector2(top.size.x - 5, 7)), Color("#b8c6cc"), true, 1.6)
	draw_circle(top.position + Vector2(top.size.x / 2, 6), 0.7, Color("#6f757a"))
	_stroke(top.position + Vector2(top.size.x / 2, 1), top.position + Vector2(top.size.x / 2, 3.2), Color("#8f959b"), 1.0)


func _fridge(r: Rect2) -> void:
	var body := Rect2(r.position + Vector2(0.5, -1), r.size - Vector2(1, 1.5))
	_shadow(body)
	_box(body, Color("#ecebe4"), true, 1.2)
	draw_line(body.position + Vector2(1.5, 5), Vector2(body.end.x - 1.5, body.position.y + 5), Color("#b9b8b0"), 0.6)
	_box(Rect2(body.end.x - 3.2, body.position.y + 6.5, 1.2, 5), Color("#9aa3a8"), true, 0.3, 0.35)  # handle
	_disc(body.position + Vector2(4, 9), 1.2, Color("#d98a3e"), true, 0.35)  # a magnet


## Urinals on the wall: white bowls seen from above, a flush button.
func _urinal(tr: Rect2i, r: Rect2) -> void:
	var up := _is_wall(tr.position.x, tr.position.y - 1)
	var x := r.position.x
	while x < r.end.x:
		var y0 := r.position.y + (1.0 if up else 7.0)
		_box(Rect2(x + 3, y0, 10, 8), Color("#f4f4ee"), true, 2.2)
		draw_set_transform(Vector2(x + 8, y0 + 4.5), 0.0, Vector2(1.0, 0.7))
		draw_circle(Vector2.ZERO, 2.6, Color("#c4d8e0"))
		draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
		draw_circle(Vector2(x + 8, y0 + (0.8 if up else 7.2)), 0.7, Color("#8f959b"))
		x += TP


## A tall wardrobe: two doors with handles.
func _wardrobe(r: Rect2) -> void:
	_shadow(r.grow(-1))
	var b := r.grow(-1)
	_box(b, Color("#8a6a4a"), true, 0.6)
	var mid := b.position.x + b.size.x / 2
	draw_line(Vector2(mid, b.position.y + 1), Vector2(mid, b.end.y - 1), INK, 0.6)
	draw_circle(Vector2(mid - 1.6, b.position.y + b.size.y / 2), 0.6, Color("#d4b870"))
	draw_circle(Vector2(mid + 1.6, b.position.y + b.size.y / 2), 0.6, Color("#d4b870"))


## A waste bin: a grey drum with a lid.
func _bin(r: Rect2) -> void:
	var c := r.get_center()
	_disc(c + Vector2(0.8, 1.0), 5.2, Color(0, 0, 0, 0.2), false)
	_disc(c, 5.0, Color("#5a5f66"))
	draw_circle(c, 3.4, Color("#72787f"))
	draw_line(c + Vector2(-2, 0), c + Vector2(2, 0), INK, 0.7)
