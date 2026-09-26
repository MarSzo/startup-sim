## A vehicle outside (entity kind VEHICLE): car, bike, taxi, tram or patrol car, drawn
## top-down; follows the server's positions smoothly.
extends Node2D

const CAR := 1
const BIKE := 2
const TAXI := 3
const TRAM := 4
const POLICE := 5
const FIRE_ENGINE := 6
const CAR_COLORS := [Color("#c0392b"), Color("#2e5fa8"), Color("#ecf0f1"), Color("#2c2f36"), Color("#27ae60"), Color("#8e44ad")]

var kind := CAR
var color := Color("#c0392b")
var facing := 2
var target := Vector2.ZERO
var last_seen_tick := 0
var _placed := false


func setup(p_kind: int, id: int) -> void:
	if p_kind != kind:
		kind = p_kind
		queue_redraw()
	color = CAR_COLORS[id % CAR_COLORS.size()]


func push(pos: Vector2, flags: int) -> void:
	target = pos
	if not _placed:
		position = pos
		_placed = true
	if (flags & 3) != facing:
		facing = flags & 3
		queue_redraw()


func _process(delta: float) -> void:
	position = position.lerp(target, minf(1.0, delta * 12.0))
	if kind == POLICE or kind == FIRE_ENGINE:
		queue_redraw()  # flashing lights


func _draw() -> void:
	var flip := facing == 3  # facing right: mirror
	match kind:
		CAR, TAXI, POLICE:
			var body := color if kind == CAR else (Color("#f4c20d") if kind == TAXI else Color("#eef1f4"))
			var r := Rect2(-22, -11, 44, 20)
			draw_rect(Rect2(r.position + Vector2(2, 3), r.size), Color(0, 0, 0, 0.25))
			draw_rect(r, body.darkened(0.25))
			draw_rect(r.grow(-1), body)
			# Windscreen at the front (the side it drives to), rear window.
			var front := 10.0 if flip else -18.0
			var back := -16.0 if flip else 12.0
			draw_rect(Rect2(front, -8, 8, 14), Color("#1f2a38"))
			draw_rect(Rect2(back, -7, 5, 12), Color("#2b3a4d"))
			draw_rect(Rect2(-8, -9, 16, 16), body.lightened(0.12))  # roof
			for wx in [-16, 12]:
				draw_rect(Rect2(wx, -12, 6, 2), Color("#15171c"))
				draw_rect(Rect2(wx, 8, 6, 2), Color("#15171c"))
			if kind == TAXI:
				draw_rect(Rect2(-5, -4, 10, 5), Color("#15171c"))
				draw_rect(Rect2(-4, -3, 8, 3), Color("#ffe066"))
			if kind == POLICE:
				draw_rect(Rect2(-21, -2, 42, 3), Color("#1f3a8a"))  # stripe
				var blink := (Time.get_ticks_msec() / 250) % 2 == 0
				draw_rect(Rect2(-6, -5, 6, 4), Color("#2f6bff") if blink else Color("#15306e"))
				draw_rect(Rect2(0, -5, 6, 4), Color("#15306e") if blink else Color("#2f6bff"))
		FIRE_ENGINE:
			var r := Rect2(-34, -13, 68, 24)
			draw_rect(Rect2(r.position + Vector2(2, 3), r.size), Color(0, 0, 0, 0.25))
			draw_rect(r, Color("#8e1b1b"))
			draw_rect(r.grow(-1), Color("#d32f2f"))
			var cab := 20.0 if flip else -32.0
			draw_rect(Rect2(cab, -10, 12, 18), Color("#b71c1c"))
			draw_rect(Rect2(cab + (8.0 if flip else 0.0), -8, 4, 14), Color("#1f2a38"))  # windscreen
			# Ladder on the roof.
			var lx := -24.0 if not flip else -14.0
			draw_rect(Rect2(lx, -6, 38, 2), Color("#cfd4da"))
			draw_rect(Rect2(lx, 2, 38, 2), Color("#cfd4da"))
			for i in 7:
				draw_rect(Rect2(lx + i * 6, -6, 1, 10), Color("#cfd4da"))
			draw_rect(Rect2(-33, 6, 66, 2), Color("#f1e05a"))  # stripe
			var blink := (Time.get_ticks_msec() / 200) % 2 == 0
			draw_rect(Rect2(cab + 2, -13, 4, 3), Color("#2f6bff") if blink else Color("#15306e"))
			draw_rect(Rect2(cab + 7, -13, 4, 3), Color("#15306e") if blink else Color("#2f6bff"))
		TRAM:
			var r := Rect2(-48, -12, 96, 22)
			draw_rect(Rect2(r.position + Vector2(2, 3), r.size), Color(0, 0, 0, 0.25))
			draw_rect(r, Color("#8e1b1b"))
			draw_rect(r.grow(-1), Color("#c62828"))
			draw_rect(Rect2(-47, -6, 94, 8), Color("#f4e9d0"))
			for i in 8:
				draw_rect(Rect2(-44 + i * 11, -5, 8, 6), Color("#2b3a4d"))
			draw_rect(Rect2(-2, -12, 4, 22), Color("#7a1414"))  # articulation
		BIKE:
			var d := -1.0 if flip else 1.0
			draw_circle(Vector2(-4, 3), 3, Color("#15171c"))
			draw_circle(Vector2(4, 3), 3, Color("#15171c"))
			draw_circle(Vector2(-4, 3), 2, Color("#9aa4ab"))
			draw_circle(Vector2(4, 3), 2, Color("#9aa4ab"))
			draw_line(Vector2(-4, 3), Vector2(0, -1), color, 1.5)
			draw_line(Vector2(0, -1), Vector2(4, 3), color, 1.5)
			draw_line(Vector2(0, -1), Vector2(3 * d, -3), color, 1.5)
			draw_rect(Rect2(-2, -3, 3, 1), Color("#15171c"))  # saddle
