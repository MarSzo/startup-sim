## Placeholder character: colored body + nick label (+ speech bubble).
## Position is in world px.
extends Node2D

const BODY := Vector2(10, 14)
const FACING_OFFSETS := [Vector2(0, 1), Vector2(0, -1), Vector2(-1, 0), Vector2(1, 0)]
const BUBBLE_WIDTH := 260.0

var color := Color.WHITE
var outline := Color(0, 0, 0, 0.8)
var facing := 0
## Uniform + cap instead of a plain body (NPC staff).
var uniform := false
var nick_label := Label.new()
var bubble := PanelContainer.new()
var bubble_label := Label.new()
var _bubble_time := 0.0
var _zoom := 1.0


func setup(p_color: Color, nick: String, zoom: float) -> void:
	color = p_color
	_zoom = zoom
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
		_build_bubble()
	queue_redraw()


func _build_bubble() -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(1, 1, 1, 0.95)
	sb.border_color = Color(0.1, 0.1, 0.1)
	sb.set_border_width_all(2)
	sb.set_corner_radius_all(8)
	sb.set_content_margin_all(8)
	bubble.add_theme_stylebox_override("panel", sb)
	bubble_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	bubble_label.custom_minimum_size = Vector2(BUBBLE_WIDTH, 0)
	bubble_label.add_theme_color_override("font_color", Color(0.1, 0.1, 0.1))
	bubble_label.add_theme_font_size_override("font_size", 15)
	bubble.add_child(bubble_label)
	bubble.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	bubble.scale = Vector2.ONE / _zoom
	bubble.visible = false
	bubble.z_index = 10
	add_child(bubble)


func set_nick(nick: String) -> void:
	nick_label.text = nick


## Show a speech bubble above the head for a few seconds.
func say(text: String) -> void:
	bubble_label.text = text
	bubble.reset_size()
	bubble.visible = true
	_bubble_time = 3.0 + text.length() * 0.05
	_place_bubble()


func _place_bubble() -> void:
	var sz := bubble.get_combined_minimum_size() / _zoom
	bubble.position = Vector2(-sz.x / 2, -BODY.y - 30 / _zoom - sz.y)


func _process(delta: float) -> void:
	if bubble.visible:
		_bubble_time -= delta
		if _bubble_time <= 0.0:
			bubble.visible = false
		else:
			_place_bubble()


func set_facing(f: int) -> void:
	if f != facing:
		facing = f
		queue_redraw()


func _draw() -> void:
	# Feet are at the node origin (collision box center); body extends upward.
	var r := Rect2(-BODY.x / 2, -BODY.y + 4, BODY.x, BODY.y)
	draw_rect(Rect2(r.position + Vector2(0, 1), r.size), Color(0, 0, 0, 0.25))
	draw_rect(r, color)
	if uniform:
		# Dark trousers, light shirt collar and a peaked cap.
		draw_rect(Rect2(r.position.x, r.end.y - 5, r.size.x, 5), color.darkened(0.5))
		draw_rect(Rect2(-2, r.position.y + 5, 4, 2), Color(0.95, 0.95, 0.9))
		draw_rect(Rect2(r.position.x - 1, r.position.y - 2, r.size.x + 2, 3), Color(0.1, 0.12, 0.25))
		draw_rect(Rect2(-2, r.position.y - 1, 4, 1), Color(0.9, 0.75, 0.2))
	draw_rect(r, outline, false, 1.0)
	# Head/eyes hint facing direction.
	var eye: Vector2 = FACING_OFFSETS[facing]
	var c := Vector2(0, -BODY.y + 8) + eye * 2.5
	draw_rect(Rect2(c - Vector2(1, 1), Vector2(2, 2)), Color(0.1, 0.1, 0.1))
