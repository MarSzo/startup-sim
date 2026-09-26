## Elevator door tile: sliding steel panels when closed (solid: MapData.closed),
## nothing over the threshold when open. The middle door also carries the
## floor display: where the car is and which way it is going.
extends Node2D

var tile := Vector2i.ZERO
var closed := true
var display := false        # this door shows the floor indicator
var lift_floor := 0
var lift_target := 255


func set_state(p_closed: bool, p_floor: int, p_target: int) -> void:
	if p_closed != closed or p_floor != lift_floor or p_target != lift_target:
		closed = p_closed
		lift_floor = p_floor
		lift_target = p_target
		queue_redraw()


const GLYPHS := {
	"P": ["##.", "#.#", "##.", "#..", "#.."],
	"0": ["###", "#.#", "#.#", "#.#", "###"],
	"1": [".#.", "##.", ".#.", ".#.", "###"],
	"2": ["##.", "..#", ".#.", "#..", "###"],
	"3": ["##.", "..#", ".#.", "..#", "##."],
	"?": ["###", "..#", ".#.", "...", ".#."],
}


static func floor_label(f: int) -> String:
	return "P" if f == 0 else str(f)


func _draw() -> void:
	if closed:
		draw_rect(Rect2(-8, -8, 16, 16), Color("#b9c2c9"))
		draw_rect(Rect2(-8, -8, 16, 1), Color("#dfe5ea"))
		draw_rect(Rect2(-1, -8, 1, 16), Color("#6f7a83"))
		draw_rect(Rect2(0, -8, 1, 16), Color("#e6ebef"))
	if display:
		# Floor indicator on the wall right of the doors (hall side): dark
		# screen, amber pixel digits (3x5 glyphs), an arrow while moving.
		var o := Vector2(30, 1)
		draw_set_transform(o)
		draw_rect(Rect2(-6, -16, 12, 7), Color("#15171c"))
		var amber := Color("#ffb347")
		var rows: Array = GLYPHS.get(floor_label(lift_floor), GLYPHS["?"])
		for y in 5:
			for x in 3:
				if rows[y][x] == "#":
					draw_rect(Rect2(-5 + x, -15 + y, 1, 1), amber)
		if lift_target != 255 and lift_target != lift_floor:
			var up := lift_target > lift_floor
			for i in 3:
				var w := 1 + i * 2
				var yy := (-15 + i) if up else (-11 - i)
				draw_rect(Rect2(2.5 - i, yy, w, 1), amber)
		draw_set_transform(Vector2.ZERO)
