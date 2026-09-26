## A toilet stall door: closed panel with a free / occupied sign (green /
## red), drawn open while someone stands in the doorway. Locked doors are
## solid in the simulation (MapData.closed) and always drawn shut.
extends Node2D

var tile := Vector2i.ZERO
var locked := false
var open := false


func set_state(p_locked: bool, p_open: bool) -> void:
	p_open = p_open and not p_locked
	if p_locked != locked or p_open != open:
		locked = p_locked
		open = p_open
		queue_redraw()


func _draw() -> void:
	# Local origin = tile center; the door stands in the partition line.
	if open:
		# Swung open against the partition above: a thin edge only.
		draw_rect(Rect2(-8, -8, 3, 16), Color("#aab3be"))
		return
	draw_rect(Rect2(-3, -8, 6, 13), Color("#b3bcc7"))
	draw_rect(Rect2(-3, -8, 6, 2), Color("#d6dce3"))
	draw_rect(Rect2(-3, 5, 6, 3), Color("#7f8894"))
	# The latch sign: red "occupied" / green "free".
	draw_rect(Rect2(-1, -3, 2, 3), Color("#e74c3c") if locked else Color("#2ecc71"))
