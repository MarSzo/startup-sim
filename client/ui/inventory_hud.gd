## Inventory bar (bottom right): hands + pockets with item icons.
## Keys: 1-3 take out / put back, Q drop, G give, F use (handled in game.gd).
extends Control

const ItemArt = preload("res://game/item_art.gd")

signal slot_clicked(pocket: int)

var slots: Array = []       # [{kind, id, label}] hands first, then pockets
var _boxes: Array[Control] = []
var _caption := Label.new()
var _keys := Label.new()


var _panel := PanelContainer.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var panel := _panel
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.07, 0.08, 0.12, 0.82)
	sb.set_corner_radius_all(8)
	sb.set_content_margin_all(10)
	panel.add_theme_stylebox_override("panel", sb)
	add_child(panel)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 6)
	panel.add_child(col)
	_caption.add_theme_font_size_override("font_size", 14)
	_caption.add_theme_color_override("font_color", Color.WHITE)
	_caption.custom_minimum_size = Vector2(250, 0)
	_caption.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	col.add_child(_caption)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	col.add_child(row)
	for i in 4:
		var box := Control.new()
		var hands := i == 0
		box.custom_minimum_size = Vector2(64, 64) if hands else Vector2(52, 52)
		box.size_flags_vertical = Control.SIZE_SHRINK_END
		var idx := i
		box.draw.connect(func(): _draw_slot(box, idx))
		box.gui_input.connect(func(ev):
			if ev is InputEventMouseButton and ev.pressed and ev.button_index == MOUSE_BUTTON_LEFT and idx > 0:
				slot_clicked.emit(idx - 1))
		row.add_child(box)
		_boxes.append(box)
		if hands:
			var sep := VSeparator.new()
			row.add_child(sep)
	_keys.text = "1–3 wyjmij/schowaj · Q upuść · G podaj · F użyj"
	_keys.add_theme_font_size_override("font_size", 12)
	_keys.add_theme_color_override("font_color", Color(1, 1, 1, 0.55))
	col.add_child(_keys)
	panel.resized.connect(_place)
	get_viewport().size_changed.connect(_place)
	update_slots([])
	_place.call_deferred()


## Bottom-right corner of the viewport (the parent is a CanvasLayer).
func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.position = vs - _panel.size - Vector2(16, 16)


func update_slots(p_slots: Array) -> void:
	slots = p_slots
	for i in _boxes.size():
		var s := _slot(i)
		var name: String = ItemArt.item_name(s.kind)
		_boxes[i].tooltip_text = ("%s\n%s" % [name, s.label]) if s.kind != 0 else ("Ręce — puste" if i == 0 else "Kieszeń %d — pusta" % i)
		_boxes[i].queue_redraw()
	var held := _slot(0)
	if held.kind != 0:
		_caption.text = "W rękach: %s%s" % [ItemArt.item_name(held.kind), (" — " + held.label) if held.label != "" else ""]
	else:
		_caption.text = "Ręce wolne"


func _slot(i: int) -> Dictionary:
	return slots[i] if i < slots.size() else {"kind": 0, "id": 0, "label": ""}


func _draw_slot(box: Control, i: int) -> void:
	var r := Rect2(Vector2.ZERO, box.size)
	box.draw_rect(r, Color(1, 1, 1, 0.08))
	box.draw_rect(r, Color(1, 1, 1, 0.35) if i == 0 else Color(1, 1, 1, 0.18), false, 2.0)
	var s := _slot(i)
	if s.kind != 0:
		var scale: float = (box.size.x - 12) / 16.0
		ItemArt.draw(box, s.kind, Vector2(6, 6), scale)
	var font := ThemeDB.fallback_font
	var caption := "ręce" if i == 0 else str(i)
	box.draw_string(font, Vector2(4, box.size.y - 4), caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 11, Color(1, 1, 1, 0.6))
