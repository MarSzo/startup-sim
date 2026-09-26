## An item lying on the floor (entity kind ITEM).
extends Node2D

const ItemArt = preload("res://game/item_art.gd")

var kind := 0
var last_seen_tick := 0


func setup(p_kind: int) -> void:
	kind = p_kind
	queue_redraw()


func _draw() -> void:
	draw_rect(Rect2(-5, 1, 10, 2), Color(0, 0, 0, 0.25))
	ItemArt.draw(self, kind, Vector2(-5, -8), 0.62)
