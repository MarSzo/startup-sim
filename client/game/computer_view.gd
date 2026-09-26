## A laptop standing on a desk (entity kind COMPUTER): open lid with the
## screen facing the chair below. Locked = dark screen with a padlock.
extends Node2D

const Protocol = preload("res://net/protocol.gd")

var flags := 0
var last_seen_tick := 0


func set_flags(f: int) -> void:
	if f != flags:
		flags = f
		queue_redraw()


func _draw() -> void:
	var locked := (flags & Protocol.PC_FLAG_LOCKED) != 0
	var in_use := (flags & Protocol.PC_FLAG_IN_USE) != 0
	# Lid (seen from behind the screen's back edge), screen, keyboard base.
	draw_rect(Rect2(-6, -7, 12, 7), Color("#23262e"))
	var screen := Color("#1b2436") if locked else (Color("#e9f1fb") if in_use else Color("#5fb2e8"))
	draw_rect(Rect2(-5, -6, 10, 5), screen)
	if locked:
		draw_rect(Rect2(-1, -4, 2, 2), Color("#e0a82e"))
		draw_rect(Rect2(-1, -5, 2, 1), Color("#9aa4ab"))
	elif in_use:
		# Chat lines on the screen.
		draw_rect(Rect2(-4, -5, 5, 1), Color("#2e6bd9"))
		draw_rect(Rect2(-4, -3, 7, 1), Color("#8a93a3"))
	draw_rect(Rect2(-6, 0, 12, 3), Color("#8a939c"))
	draw_rect(Rect2(-6, 0, 12, 1), Color("#b4bcc3"))
	draw_rect(Rect2(-2, 2, 4, 1), Color("#6d757d"))
