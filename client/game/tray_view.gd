## The tray of sweets on the chill-room table (entity kind TRAY): what's on
## it and how many pieces are left.
extends Node2D

const ItemArt = preload("res://game/item_art.gd")

var kind := 25
var pieces := 0
var last_seen_tick := 0


func set_state(p_kind: int, p_pieces: int) -> void:
	if p_kind != kind or p_pieces != pieces:
		kind = p_kind
		pieces = p_pieces
		queue_redraw()


func _draw() -> void:
	draw_circle(Vector2(1, 2), 11, Color(0, 0, 0, 0.2))
	draw_circle(Vector2.ZERO, 11, Color("#b8bec4"))
	draw_circle(Vector2.ZERO, 10, Color("#e6ebef"))
	for i in pieces:
		var a := TAU * i / maxf(pieces, 1) - PI / 2
		var o := Vector2(cos(a), sin(a)) * (6.0 if pieces > 1 else 0.0) - Vector2(3, 3)
		ItemArt.draw(self, kind, o, 0.38)
