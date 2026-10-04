## A door plaque read up close (E by a door): the name of the room behind it
## (toilets: just the sign), big in the middle of the screen. Goes away with
## E / Esc or on walking off.
extends Control

const Ink = preload("res://ui/ink_ui.gd")

const PLATE := Color("#d9c27a")
const PLATE_LO := Color("#a88f4a")

var room := 0
var _icon := ""
var _plate := PanelContainer.new()
var _label := Label.new()
var _sign := Control.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	var box := StyleBoxFlat.new()
	box.bg_color = PLATE
	box.border_color = Ink.INK
	box.set_border_width_all(3)
	box.set_corner_radius_all(6)
	box.shadow_color = Color(0, 0, 0, 0.35)
	box.shadow_size = 8
	box.shadow_offset = Vector2(3, 4)
	box.content_margin_left = 48
	box.content_margin_right = 48
	box.content_margin_top = 22
	box.content_margin_bottom = 22
	_plate.add_theme_stylebox_override("panel", box)
	_plate.custom_minimum_size = Vector2(360, 0)
	add_child(_plate)
	Ink.style_label(_label, 40, Ink.TEXT_INK)
	_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	var col := VBoxContainer.new()
	col.alignment = BoxContainer.ALIGNMENT_CENTER
	_plate.add_child(col)
	col.add_child(_label)
	_sign.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	_sign.draw.connect(_draw_sign)
	col.add_child(_sign)
	_plate.draw.connect(_screws)
	get_viewport().size_changed.connect(_place)


## `icon`: "female" / "male" / "accessible" / "unisex" = a sign instead of
## the name.
func open(rid: int, room_name: String, icon := "") -> void:
	room = rid
	_icon = icon
	_label.text = room_name
	_label.visible = icon == ""
	_sign.visible = icon != ""
	_sign.custom_minimum_size = Vector2(190 if icon == "unisex" else 100, 130)
	_plate.custom_minimum_size = Vector2(360 if icon == "" else 0, 0)
	_sign.queue_redraw()
	visible = true
	_place.call_deferred()


func close() -> void:
	visible = false
	room = 0


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_plate.reset_size()
	_plate.position = (vs - _plate.size) / 2


## Four screws in the corners and an engraved line.
func _screws() -> void:
	var s := _plate.size
	for c in [Vector2(14, 14), Vector2(s.x - 14, 14), Vector2(14, s.y - 14), Vector2(s.x - 14, s.y - 14)]:
		_plate.draw_circle(c, 4.0, PLATE_LO)
		_plate.draw_arc(c, 4.0, 0, TAU, 12, Ink.INK, 1.2, true)
		_plate.draw_line(c - Vector2(2.5, 2.5), c + Vector2(2.5, 2.5), Ink.INK, 1.0)
	_plate.draw_rect(Rect2(Vector2(8, 8), s - Vector2(16, 16)), Color(PLATE_LO, 0.8), false, 1.5)


func _draw_sign() -> void:
	var s := _sign.size
	match _icon:
		"female": _woman(s.x / 2)
		"male": _man(s.x / 2)
		"accessible": _wheelchair(s.x / 2)
		"unisex":
			_man(s.x / 2 - 48)
			_sign.draw_line(Vector2(s.x / 2, 8), Vector2(s.x / 2, s.y - 8), Ink.INK, 4.0)
			_woman(s.x / 2 + 48)


func _man(cx: float) -> void:
	_sign.draw_circle(Vector2(cx, 16), 13, Ink.INK)
	_sign.draw_rect(Rect2(cx - 15, 34, 30, 44), Ink.INK)
	_sign.draw_rect(Rect2(cx - 25, 34, 7, 40), Ink.INK)  # arms
	_sign.draw_rect(Rect2(cx + 18, 34, 7, 40), Ink.INK)
	_sign.draw_rect(Rect2(cx - 15, 76, 13, 50), Ink.INK)
	_sign.draw_rect(Rect2(cx + 2, 76, 13, 50), Ink.INK)


func _woman(cx: float) -> void:
	_sign.draw_circle(Vector2(cx, 16), 13, Ink.INK)
	_sign.draw_colored_polygon(PackedVector2Array([Vector2(cx - 11, 34), Vector2(cx + 11, 34), Vector2(cx + 26, 92), Vector2(cx - 26, 92)]), Ink.INK)
	_sign.draw_line(Vector2(cx - 14, 37), Vector2(cx - 27, 70), Ink.INK, 7.0)  # arms
	_sign.draw_line(Vector2(cx + 14, 37), Vector2(cx + 27, 70), Ink.INK, 7.0)
	_sign.draw_rect(Rect2(cx - 12, 90, 9, 36), Ink.INK)
	_sign.draw_rect(Rect2(cx + 3, 90, 9, 36), Ink.INK)


func _wheelchair(cx: float) -> void:
	_sign.draw_circle(Vector2(cx - 6, 14), 12, Ink.INK)
	_sign.draw_line(Vector2(cx - 10, 32), Vector2(cx - 10, 72), Ink.INK, 12.0)   # back
	_sign.draw_line(Vector2(cx - 14, 70), Vector2(cx + 20, 70), Ink.INK, 10.0)   # seat
	_sign.draw_line(Vector2(cx + 18, 70), Vector2(cx + 30, 104), Ink.INK, 10.0)  # shin
	_sign.draw_line(Vector2(cx - 10, 48), Vector2(cx + 14, 48), Ink.INK, 8.0)    # arm
	_sign.draw_arc(Vector2(cx - 6, 92), 30, deg_to_rad(-200), deg_to_rad(20), 24, Ink.INK, 7.0, true)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and event.physical_keycode == KEY_ESCAPE:
		close()
		get_viewport().set_input_as_handled()
