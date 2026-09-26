## Cigarette smoke in the rooms of the shown floor (the server's Smoke
## packets) and the smoke detectors on the ceilings, blinking red during a
## fire alarm. Drawn over the world.
extends Node2D

var building
var floor_shown := 0
var alarm := false
var _target := {}   # room -> 0..1 (last packet)
var _shown := {}    # room -> 0..1 (eased)
var _tiles := {}    # floor -> {room: Array[Vector2i]}
var _detectors := {}  # floor -> Array[Vector2] (px)


func setup(b) -> void:
	building = b
	z_index = 10
	for f in b.floors.size():
		var m = b.get_floor(f)
		if m == null:
			continue
		var rooms := {}
		for y in m.height:
			for x in m.width:
				if m.is_blocked(x, y):
					continue
				var r: int = m.room_at_tile(x, y)
				if not rooms.has(r):
					rooms[r] = []
				rooms[r].append(Vector2i(x, y))
		_tiles[f] = rooms
		var dets := []
		for r in m.room_detector:
			if not rooms.has(r):
				continue
			# On the ceiling in the middle of the room (snapped to a floor tile).
			var sum := Vector2.ZERO
			for t in rooms[r]:
				sum += Vector2(t)
			var mid: Vector2 = sum / rooms[r].size()
			var best: Vector2i = rooms[r][0]
			for t in rooms[r]:
				if Vector2(t).distance_squared_to(mid) < Vector2(best).distance_squared_to(mid):
					best = t
			dets.append((Vector2(best) + Vector2(0.5, 0.5)) * m.tile_px)
		_detectors[f] = dets


func set_floor(f: int) -> void:
	if f != floor_shown:
		floor_shown = f
		_target.clear()
		_shown.clear()


func on_smoke(p: Dictionary) -> void:
	if p.floor != floor_shown:
		return
	_target.clear()
	for e in p.rooms:
		_target[e[0]] = e[1] / 255.0


func _process(delta: float) -> void:
	for r in _target:
		if not _shown.has(r):
			_shown[r] = 0.0
	for r in _shown.keys():
		var want: float = _target.get(r, 0.0)
		_shown[r] = move_toward(_shown[r], want, delta * 0.5)
		if _shown[r] <= 0.0 and want <= 0.0:
			_shown.erase(r)
	queue_redraw()  # drifting smoke, blinking detectors


## How opaque the haze is for a smoke level 0..1: thin smoke is a light
## veil, thick smoke (a small room, a long cigarette) hides everything.
static func opacity(level: float) -> float:
	return clampf(pow(level, 0.55) * 1.15, 0.0, 0.97)


func _draw() -> void:
	var m = building.get_floor(floor_shown) if building else null
	if m == null:
		return
	var px: float = m.tile_px
	var t := Time.get_ticks_msec() / 1000.0
	var rooms: Dictionary = _tiles.get(floor_shown, {})
	for r in _shown:
		var a: float = _shown[r]
		if a <= 0.01 or not rooms.has(r):
			continue
		var op := opacity(a)
		var tiles: Array = rooms[r]
		# The veil: every tile of the room.
		var haze := Color(0.66, 0.63, 0.58, op)
		for tile in tiles:
			draw_rect(Rect2(Vector2(tile) * px, Vector2(px, px)), haze)
		# Billowing puffs (Don't Starve-like): one ink rim around the whole
		# cloud (all rims first, then all fills), soft shading inside.
		var puffs := []
		var step := maxi(2, int(round(5.0 - a * 3.0)))
		for k in range(0, tiles.size(), step):
			var tile: Vector2i = tiles[k]
			var ph := float(tile.x * 7 + tile.y * 13)
			var off := Vector2(sin(t * 0.35 + ph) * px * 0.7, cos(t * 0.27 + ph * 1.3) * px * 0.6)
			var rad := px * (0.75 + 0.45 * a + 0.15 * sin(t * 0.8 + ph))
			puffs.append([(Vector2(tile) + Vector2(0.5, 0.5)) * px + off, rad])
		var rim := Color(0.14, 0.11, 0.08, minf(0.7, 0.2 + op * 0.55))
		var base := Color(0.62, 0.6, 0.56, minf(0.97, 0.3 + op * 0.7))
		for pf in puffs:
			draw_circle(pf[0], pf[1] + 0.8, rim)
		for pf in puffs:
			draw_circle(pf[0], pf[1], base)
		# Light from above-left, shadow bottom-right, both soft.
		var lit := Color(0.72, 0.7, 0.66, base.a * 0.6)
		var shade := Color(0.5, 0.48, 0.45, base.a * 0.35)
		for pf in puffs:
			draw_circle(pf[0] + Vector2(pf[1] * 0.18, pf[1] * 0.22), pf[1] * 0.75, shade)
		for pf in puffs:
			draw_circle(pf[0] - Vector2(pf[1] * 0.2, pf[1] * 0.25), pf[1] * 0.55, lit)
	# Smoke detectors: white disc, red LED (a blink now and then; fast in an alarm).
	for pos in _detectors.get(floor_shown, []):
		draw_circle(pos, 3.5, Color("#6b6f78"))
		draw_circle(pos, 3.0, Color("#eceef1"))
		var on := (int(t * 6.0) % 2 == 0) if alarm else (fmod(t, 3.0) < 0.15)
		draw_circle(pos, 1.0, Color("#ff2d2d") if on else Color("#7a2222"))
