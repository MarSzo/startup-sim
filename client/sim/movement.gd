## Deterministic movement, wall collision and floor transitions.
## Mirror of server/src/sim.rs - keep both in sync (golden test checks parity).
extends RefCounted

const SUBPIXELS := 16
const TILE_UNITS := 256
const INPUT_HZ := 60
const SPEED := 24
const SPEED_DIAG := 17
const HALF_W := 5 * SUBPIXELS
const HALF_H := 4 * SUBPIXELS

const IN_UP := 1
const IN_DOWN := 2
const IN_LEFT := 4
const IN_RIGHT := 8
const IN_INTERACT := 16
const IN_MOVE_MASK := 0x0f

const LOCK_NONE := 0
const LOCK_HELD := 1
const LOCK_RELEASED := 2


static func floor_div(a: int, b: int) -> int:
	if a >= 0:
		return a / b
	return -((-a + b - 1) / b)


static func tile_of(v: int) -> int:
	return floor_div(v, TILE_UNITS)


static func tile_center(tx: int, ty: int) -> Vector2i:
	return Vector2i(tx * TILE_UNITS + TILE_UNITS / 2, ty * TILE_UNITS + TILE_UNITS / 2)


static func input_dir(input: int) -> Vector2i:
	var dx := int(input & IN_RIGHT != 0) - int(input & IN_LEFT != 0)
	var dy := int(input & IN_DOWN != 0) - int(input & IN_UP != 0)
	return Vector2i(dx, dy)


## A character's full simulated state (see Body in sim.rs).
static func body(floor_i: int, pos: Vector2i, prev_input := 0, lock := LOCK_NONE) -> Dictionary:
	return {"floor": floor_i, "pos": pos, "prev": prev_input, "lock": lock}


static func tile_of_pos(p: Vector2i) -> Vector2i:
	return Vector2i(tile_of(p.x), tile_of(p.y))


## One input step (1/60 s) in the building: movement, then floor links.
## Stairs move you when you step onto them (unless locked right after
## arriving); the elevator needs an interact press inside the cabin.
static func step(building, b: Dictionary, input: int) -> Dictionary:
	var map = building.get_floor(b.floor)
	if map == null:
		return b
	var n := b.duplicate()
	n.pos = move_on(map, b.pos, input)
	if n.lock == LOCK_HELD and (input & IN_MOVE_MASK) != (b.prev & IN_MOVE_MASK):
		n.lock = LOCK_RELEASED
	var t := tile_of_pos(n.pos)
	var link: Dictionary = map.link_at(t.x, t.y)
	if n.lock == LOCK_RELEASED and link.is_empty():
		n.lock = LOCK_NONE
	if not link.is_empty():
		if link.kind == "stairs":
			if n.lock == LOCK_NONE and building.get_floor(link.to_floor) != null:
				n.floor = link.to_floor
				n.pos = tile_center(link.to.x, link.to.y)
				n.lock = LOCK_HELD
		elif link.kind == "elevator":
			if (input & IN_INTERACT) != 0 and (b.prev & IN_INTERACT) == 0:
				var f: int = building.next_elevator_floor(n.floor, link.id)
				if f >= 0:
					n.floor = f
					n.lock = LOCK_HELD
	n.prev = input
	return n


## Move by one input step on a single floor. Resolves X then Y so the player
## slides along walls.
static func move_on(map, pos: Vector2i, input: int) -> Vector2i:
	var d := input_dir(input)
	if d == Vector2i.ZERO:
		return pos
	var speed := SPEED_DIAG if (d.x != 0 and d.y != 0) else SPEED
	var p := pos
	if d.x != 0:
		p.x = _move_x(map, p, d.x * speed)
	if d.y != 0:
		p.y = _move_y(map, p, d.y * speed)
	return p


static func _move_x(map, p: Vector2i, mx: int) -> int:
	var nx := p.x + mx
	var ty0 := tile_of(p.y - HALF_H)
	var ty1 := tile_of(p.y + HALF_H - 1)
	if mx > 0:
		var tx := tile_of(nx + HALF_W - 1)
		for ty in range(ty0, ty1 + 1):
			if map.is_blocked(tx, ty):
				return tx * TILE_UNITS - HALF_W
	else:
		var tx := tile_of(nx - HALF_W)
		for ty in range(ty0, ty1 + 1):
			if map.is_blocked(tx, ty):
				return (tx + 1) * TILE_UNITS + HALF_W
	return nx


static func _move_y(map, p: Vector2i, my: int) -> int:
	var ny := p.y + my
	var tx0 := tile_of(p.x - HALF_W)
	var tx1 := tile_of(p.x + HALF_W - 1)
	if my > 0:
		var ty := tile_of(ny + HALF_H - 1)
		for tx in range(tx0, tx1 + 1):
			if map.is_blocked(tx, ty):
				return ty * TILE_UNITS - HALF_H
	else:
		var ty := tile_of(ny - HALF_H)
		for tx in range(tx0, tx1 + 1):
			if map.is_blocked(tx, ty):
				return (ty + 1) * TILE_UNITS + HALF_H
	return ny


## Sub-pixel units -> world pixels.
static func to_px(p: Vector2i) -> Vector2:
	return Vector2(p) / float(SUBPIXELS)
