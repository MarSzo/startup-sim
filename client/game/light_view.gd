## How bright each room of the shown floor is (Don't Starve-ish darkness):
## outdoors = time of day x weather; a room with windows gets some of that;
## a room without windows is dark; a lamp switched on (the server's Lights)
## lights it up with a warm tone. Common areas are always lit. Also draws the
## light switches by the doors. Over the world, under the smoke.
extends Node2D

const Protocol = preload("res://net/protocol.gd")

var building
var floor_shown := 0
var minute := 12 * 60
var weather := Protocol.WEATHER_SUNNY
var _on := {}        # room -> true (lamps on, this floor)
var _tiles := {}     # floor -> {room: Array[Vector2i]}
var _walls := {}     # floor -> Array[Vector2i]
var _shown := {}     # room -> eased darkness 0..1


func setup(b) -> void:
	building = b
	z_index = 9
	for f in b.floors.size():
		var m = b.get_floor(f)
		if m == null:
			continue
		var rooms := {}
		var walls := []
		for y in m.height:
			for x in m.width:
				var ch: String = m.tile_chars[y * m.width + x]
				if ch == "#":
					walls.append(Vector2i(x, y))
					continue
				if ch == "~":
					continue
				var r: int = m.room_at_tile(x, y)
				if not rooms.has(r):
					rooms[r] = []
				rooms[r].append(Vector2i(x, y))
		_tiles[f] = rooms
		_walls[f] = walls


func set_floor(f: int) -> void:
	if f != floor_shown:
		floor_shown = f
		_on.clear()
		_shown.clear()


func on_lights(p: Dictionary) -> void:
	if p.floor != floor_shown:
		return
	_on.clear()
	for r in p.rooms:
		_on[r] = true


## Daylight 0..1 at a minute of the day.
static func daylight(m: int) -> float:
	if m < 6 * 60 or m >= 23 * 60:
		return 0.12
	if m < 8 * 60:
		return lerpf(0.3, 1.0, (m - 6 * 60) / 120.0)
	if m < 18 * 60:
		return 1.0
	if m < 21 * 60 + 30:
		return lerpf(1.0, 0.2, (m - 18 * 60) / 210.0)
	return 0.18


static func weather_factor(w: int) -> float:
	match w:
		Protocol.WEATHER_CLOUDY: return 0.75
		Protocol.WEATHER_FOG: return 0.7
		Protocol.WEATHER_RAIN: return 0.55
		Protocol.WEATHER_STORM: return 0.4
	return 1.0


## How lit a room is, 0..1.
func light_of(m, r: int) -> float:
	var sky := daylight(minute) * weather_factor(weather)
	var lamp_room: int = m.room_lit_by.get(r, r)
	var kind: String = m.room_light.get(lamp_room, "")
	if m.room_outdoor.has(r) or (kind == "" and not m.room_lit_by.has(r)):
		return maxf(sky, 0.15)
	if kind == "always" or _on.has(lamp_room):
		return 0.95
	return sky * 0.85 if m.room_windows.has(r) else 0.08


func lamp_on(m, r: int) -> bool:
	var lamp_room: int = m.room_lit_by.get(r, r)
	return _on.has(lamp_room) or m.room_light.get(lamp_room, "") == "always"


func _process(delta: float) -> void:
	var m = building.get_floor(floor_shown) if building else null
	if m == null:
		return
	for r in _tiles.get(floor_shown, {}):
		var want := 1.0 - light_of(m, r)
		_shown[r] = move_toward(_shown.get(r, want), want, delta * 3.0)
	queue_redraw()


func _draw() -> void:
	var m = building.get_floor(floor_shown) if building else null
	if m == null:
		return
	var px: float = m.tile_px
	var night := Color(0.03, 0.04, 0.1)
	var rooms: Dictionary = _tiles.get(floor_shown, {})
	for r in rooms:
		var dark: float = _shown.get(r, 0.0)
		var col := Color(night, dark * 0.9)
		var warm: bool = lamp_on(m, r) and m.room_light.get(m.room_lit_by.get(r, r), "") == "switch"
		for t in rooms[r]:
			var rect := Rect2(Vector2(t) * px, Vector2(px, px))
			if dark > 0.01:
				draw_rect(rect, col)
			if warm:
				draw_rect(rect, Color(1.0, 0.82, 0.5, 0.07))
	# Walls follow the sky (so the building sinks into the night too).
	var sky_dark := 1.0 - maxf(daylight(minute) * weather_factor(weather), 0.15)
	if sky_dark > 0.01:
		for t in _walls.get(floor_shown, []):
			draw_rect(Rect2(Vector2(t) * px, Vector2(px, px)), Color(night, sky_dark * 0.7))
	# Light switches: a small plate on the wall by the switch tile.
	for r in m.room_switch:
		var t: Vector2i = m.room_switch[r]
		var wall_dir := Vector2i.ZERO
		for d in [Vector2i(0, -1), Vector2i(-1, 0), Vector2i(1, 0), Vector2i(0, 1)]:
			var n: Vector2i = t + d
			if n.x >= 0 and n.y >= 0 and n.x < m.width and n.y < m.height and m.tile_chars[n.y * m.width + n.x] == "#":
				wall_dir = d
				break
		var c := (Vector2(t) + Vector2(0.5, 0.5)) * px + Vector2(wall_dir) * px * 0.42
		var plate := Rect2(c - Vector2(2.2, 2.8), Vector2(4.4, 5.6))
		draw_rect(plate.grow(0.6), Color(0.12, 0.09, 0.07))
		draw_rect(plate, Color(0.95, 0.93, 0.86))
		var up := _on.has(r)
		draw_rect(Rect2(c + Vector2(-0.9, -1.8 if up else 0.2), Vector2(1.8, 1.6)), Color(0.9, 0.7, 0.25) if up else Color(0.45, 0.45, 0.45))
