## The day's log (H): everything said near you and every notice, with the
## game time, scrollable - the corner log only keeps the last few lines.
extends PanelContainer

const Ink = preload("res://ui/ink_ui.gd")

const MAX_LINES := 400

var lines: Array[String] = []
var _list := VBoxContainer.new()
var _scroll := ScrollContainer.new()


func _ready() -> void:
	visible = false
	add_theme_stylebox_override("panel", Ink.box("paper"))
	set_anchors_preset(Control.PRESET_CENTER)
	custom_minimum_size = Vector2(640, 440)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 8)
	add_child(col)
	var title := Label.new()
	Ink.style_label(title, 22, Ink.TEXT_INK)
	title.text = "Dziennik dnia  (H — zamknij)"
	col.add_child(title)
	_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	col.add_child(_scroll)
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(_list)
	get_viewport().size_changed.connect(_place)


func _place() -> void:
	var vs := get_viewport_rect().size
	position = (vs - size) / 2


func add(line: String) -> void:
	lines.append(line)
	if lines.size() > MAX_LINES:
		lines.pop_front()
		if _list.get_child_count() > 0:
			_list.get_child(0).queue_free()
	var l := Label.new()
	Ink.style_label(l, 15, Ink.TEXT_INK)
	l.text = line
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_list.add_child(l)
	if visible:
		_to_bottom.call_deferred()


func _unhandled_input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
		visible = false
		get_viewport().set_input_as_handled()


func toggle() -> void:
	visible = not visible
	if visible:
		_place.call_deferred()
		_to_bottom.call_deferred()


func _to_bottom() -> void:
	_scroll.scroll_vertical = int(_scroll.get_v_scroll_bar().max_value)
