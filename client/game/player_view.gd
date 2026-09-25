## Placeholder character: colored body + nick label. Position is in world px.
extends Node2D

const BODY := Vector2(10, 14)
const FACING_OFFSETS := [Vector2(0, 1), Vector2(0, -1), Vector2(-1, 0), Vector2(1, 0)]

var color := Color.WHITE
var outline := Color(0, 0, 0, 0.8)
var facing := 0
var nick_label := Label.new()


func setup(p_color: Color, nick: String, zoom: float) -> void:
	color = p_color
	nick_label.text = nick
	var ls := LabelSettings.new()
	ls.font_size = 16
	ls.outline_size = 4
	ls.outline_color = Color.BLACK
	nick_label.label_settings = ls
	nick_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	nick_label.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	# Render the label at screen resolution regardless of camera zoom.
	nick_label.scale = Vector2.ONE / zoom
	nick_label.size = Vector2(200, 24)
	nick_label.position = Vector2(-100 / zoom, -BODY.y - 26 / zoom)
	if nick_label.get_parent() == null:
		add_child(nick_label)
	queue_redraw()


func set_nick(nick: String) -> void:
	nick_label.text = nick


func set_facing(f: int) -> void:
	if f != facing:
		facing = f
		queue_redraw()


func _draw() -> void:
	# Feet are at the node origin (collision box center); body extends upward.
	var r := Rect2(-BODY.x / 2, -BODY.y + 4, BODY.x, BODY.y)
	draw_rect(Rect2(r.position + Vector2(0, 1), r.size), Color(0, 0, 0, 0.25))
	draw_rect(r, color)
	draw_rect(r, outline, false, 1.0)
	# Head/eyes hint facing direction.
	var eye: Vector2 = FACING_OFFSETS[facing]
	var c := Vector2(0, -BODY.y + 8) + eye * 2.5
	draw_rect(Rect2(c - Vector2(1, 1), Vector2(2, 2)), Color(0.1, 0.1, 0.1))
