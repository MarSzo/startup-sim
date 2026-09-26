## While riding the elevator: everything but the cabin goes dark (you only
## see the inside of the car).
extends Node2D

var hole := Rect2()   # cabin in world pixels
const FAR := 100000.0


func _init() -> void:
	visible = false  # drawn over the map but under the people (see game.gd)


func show_cabin(r: Rect2) -> void:
	hole = r
	visible = true
	queue_redraw()


func _draw() -> void:
	var c := Color("#0b0c10")
	var h := hole
	draw_rect(Rect2(-FAR, -FAR, 2 * FAR, FAR + h.position.y), c)          # above
	draw_rect(Rect2(-FAR, h.end.y, 2 * FAR, FAR), c)                        # below
	draw_rect(Rect2(-FAR, h.position.y, FAR + h.position.x, h.size.y), c)   # left
	draw_rect(Rect2(h.end.x, h.position.y, FAR, h.size.y), c)               # right
	# Brushed-steel walls around the car.
	draw_rect(h.grow(1.5), Color("#8a939c"), false, 3.0)
