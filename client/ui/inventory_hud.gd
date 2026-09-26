## Inventory bar (bottom centre, Don't Starve style): a dark wooden strip
## with the hands slot (bigger) and three pockets, keys 1-3 above the
## pockets, the name of what you hold above the bar.
## Keys: 1-3 take out / put back, Q drop, G give, F use (handled in game.gd).
extends Control

const ItemArt = preload("res://game/item_art.gd")
const Ink = preload("res://ui/ink_ui.gd")

signal slot_clicked(pocket: int)

var slots: Array = []       # [{kind, id, label}] hands first, then pockets
var _boxes: Array[Control] = []
var _caption := Label.new()
var _keys := Label.new()
var _panel := PanelContainer.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_panel.add_theme_stylebox_override("panel", Ink.box("hud"))
	add_child(_panel)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	_panel.add_child(row)
	for i in 4:
		var box := Control.new()
		var hands := i == 0
		box.custom_minimum_size = Vector2(76, 76) if hands else Vector2(62, 62)
		box.size_flags_vertical = Control.SIZE_SHRINK_END
		var idx := i
		box.draw.connect(func(): _draw_slot(box, idx))
		box.gui_input.connect(func(ev):
			if ev is InputEventMouseButton and ev.pressed and ev.button_index == MOUSE_BUTTON_LEFT and idx > 0:
				slot_clicked.emit(idx - 1))
		row.add_child(box)
		_boxes.append(box)
		if hands:
			var sep := Control.new()
			sep.custom_minimum_size = Vector2(8, 0)
			sep.draw.connect(func(): sep.draw_line(Vector2(4, 10), Vector2(4, sep.size.y - 10), Color(Ink.DARK_HI, 0.9), 2.0, true))
			row.add_child(sep)
	# Caption above, key help below the bar (outlined text over the world).
	for l in [_caption, _keys]:
		l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		l.add_theme_constant_override("outline_size", 6)
		l.add_theme_color_override("font_outline_color", Ink.INK)
		add_child(l)
	Ink.style_label(_caption, 18, Ink.TEXT)
	Ink.style_label(_keys, 14, Ink.TEXT_DIM)
	_keys.text = "1–3 wyjmij / schowaj  ·  Q upuść  ·  G podaj  ·  F użyj"
	_panel.resized.connect(_place)
	get_viewport().size_changed.connect(_place)
	update_slots([])
	_place.call_deferred()


## Bottom centre of the viewport (the parent is a CanvasLayer).
func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.reset_size()
	_panel.position = Vector2((vs.x - _panel.size.x) / 2, vs.y - _panel.size.y - 30)
	_keys.size = Vector2(vs.x, 24)
	_keys.position = Vector2(0, vs.y - 28)
	_caption.size = Vector2(vs.x, 28)
	_caption.position = Vector2(0, _panel.position.y - 32)


## Top of the bar (for placing hints above it).
func top() -> float:
	return _caption.position.y


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
		_caption.text = ""


func _slot(i: int) -> Dictionary:
	return slots[i] if i < slots.size() else {"kind": 0, "id": 0, "label": ""}


func _draw_slot(box: Control, i: int) -> void:
	var r := Rect2(Vector2.ZERO, box.size)
	var s := _slot(i)
	Ink.box("slot_active" if i == 0 and s.kind != 0 else "slot").draw(box.get_canvas_item(), r)
	if s.kind != 0:
		var scale: float = (box.size.x - 20) / 16.0
		ItemArt.draw(box, s.kind, Vector2(10, 10), scale)
	var f := Ink.font()
	var caption := "ręce" if i == 0 else str(i)
	var p := Vector2(8, box.size.y - 8)
	box.draw_string_outline(f, p, caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 16, 4, Ink.INK)
	box.draw_string(f, p, caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 16, Ink.GOLD if i > 0 else Ink.TEXT_DIM)
