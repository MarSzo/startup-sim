## A shop shelf (E at a shelf): what's on it, with prices. "Weź" (or keys
## 1-9) takes one - unpaid until you pay at the till.
extends Control

const ItemArt = preload("res://game/item_art.gd")

signal take(shelf: int, kind: int)

var shelf := 0
var goods: Array = []
var _panel := PanelContainer.new()
var _title := Label.new()
var _list := VBoxContainer.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color("#f4f1ea")
	sb.border_color = Color("#8f7a5a")
	sb.set_border_width_all(2)
	sb.set_corner_radius_all(10)
	sb.set_content_margin_all(14)
	sb.shadow_color = Color(0, 0, 0, 0.4)
	sb.shadow_size = 10
	_panel.add_theme_stylebox_override("panel", sb)
	add_child(_panel)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 8)
	_panel.add_child(col)
	_title.add_theme_font_size_override("font_size", 18)
	_title.add_theme_color_override("font_color", Color("#1c2430"))
	col.add_child(_title)
	_list.add_theme_constant_override("separation", 6)
	col.add_child(_list)
	var hint := Label.new()
	hint.text = "1–9 weź · Esc zamknij · płaci się przy kasie"
	hint.add_theme_font_size_override("font_size", 12)
	hint.add_theme_color_override("font_color", Color("#6b6456"))
	col.add_child(hint)
	_panel.resized.connect(_place)
	get_viewport().size_changed.connect(_place)


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.position = Vector2((vs.x - _panel.size.x) / 2, vs.y - _panel.size.y - 110)


static func zl(gr: int) -> String:
	return "%d,%02d zł" % [gr / 100, gr % 100]


func show_shelf(p: Dictionary) -> void:
	shelf = p.shelf
	goods = p.goods
	_title.text = "Półka: %s" % p.title
	for c in _list.get_children():
		c.queue_free()
	for i in goods.size():
		var g: Dictionary = goods[i]
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 10)
		var icon := Control.new()
		icon.custom_minimum_size = Vector2(32, 32)
		var kind: int = g.kind
		icon.draw.connect(func(): ItemArt.draw(icon, kind, Vector2(0, 0), 2.0))
		row.add_child(icon)
		var name := Label.new()
		name.text = "%d. %s" % [i + 1, g.name]
		name.custom_minimum_size = Vector2(200, 0)
		name.add_theme_font_size_override("font_size", 16)
		name.add_theme_color_override("font_color", Color("#1c2430"))
		name.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		row.add_child(name)
		var price := Label.new()
		price.text = zl(g.price)
		price.custom_minimum_size = Vector2(80, 0)
		price.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		price.add_theme_font_size_override("font_size", 16)
		price.add_theme_color_override("font_color", Color("#8f5a1a"))
		price.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		row.add_child(price)
		var b := Button.new()
		b.text = "Weź"
		b.focus_mode = Control.FOCUS_NONE
		b.pressed.connect(func(): take.emit(shelf, kind))
		row.add_child(b)
		_list.add_child(row)
	visible = true
	_panel.reset_size()
	_place.call_deferred()


func close() -> void:
	visible = false


func _unhandled_input(event: InputEvent) -> void:
	if not visible or not (event is InputEventKey and event.pressed and not event.echo):
		return
	if event.keycode == KEY_ESCAPE:
		close()
		get_viewport().set_input_as_handled()
	elif event.keycode >= KEY_1 and event.keycode <= KEY_9:
		var i: int = event.keycode - KEY_1
		if i < goods.size():
			take.emit(shelf, goods[i].kind)
			get_viewport().set_input_as_handled()
