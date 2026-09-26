## Inventory bar (bottom right): hands + pockets with item icons.
## Keys: 1-3 take out / put back, Q drop, G give, F use (handled in game.gd).
extends Control

const ItemArt = preload("res://game/item_art.gd")
const PixelUI = preload("res://ui/pixel_ui.gd")

signal slot_clicked(pocket: int)

var slots: Array = []       # [{kind, id, label}] hands first, then pockets
var _boxes: Array[Control] = []
var _caption := Label.new()
var _keys := Label.new()


var _panel := PanelContainer.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var panel := _panel
	panel.add_theme_stylebox_override("panel", PixelUI.box("hud"))
	add_child(panel)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 6)
	panel.add_child(col)
	PixelUI.style_label(_caption, 16, PixelUI.TEXT)
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
			var sep := Control.new()
			sep.custom_minimum_size = Vector2(PixelUI.PX * 2, 0)
			sep.draw.connect(func(): sep.draw_rect(Rect2(0, 4, PixelUI.PX, sep.size.y - 8), PixelUI.NAVY_HI))
			row.add_child(sep)
	_keys.text = "1–3 wyjmij/schowaj · Q upuść · G podaj · F użyj"
	PixelUI.style_label(_keys, 16, PixelUI.TEXT_DIM)
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
	var s := _slot(i)
	PixelUI.box("slot_active" if i == 0 and s.kind != 0 else "slot").draw(box.get_canvas_item(), r)
	if s.kind != 0:
		var scale: float = (box.size.x - 12) / 16.0
		ItemArt.draw(box, s.kind, Vector2(6, 6), scale)
	var caption := "ręce" if i == 0 else str(i)
	box.draw_string(PixelUI.font(), Vector2(6, box.size.y - 6), caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 16, Color(PixelUI.TEXT_DIM, 0.8))
