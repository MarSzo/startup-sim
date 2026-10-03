## Typed chat (Enter): a line at the bottom of the screen. Enter sends it,
## Esc closes. "/s text" whispers to the person next to you, "/k text"
## shouts to the whole floor.
extends PanelContainer

const Ink = preload("res://ui/ink_ui.gd")

signal sent(text: String)

const MAX_CHARS := 120

var _input := LineEdit.new()


func _ready() -> void:
	visible = false
	add_theme_stylebox_override("panel", Ink.box("hud"))
	set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	custom_minimum_size = Vector2(560, 0)
	var row := HBoxContainer.new()
	add_child(row)
	var l := Label.new()
	Ink.style_label(l, 15, Color(1, 1, 1, 0.8))
	l.text = "Czat:"
	row.add_child(l)
	_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_input.max_length = MAX_CHARS
	_input.placeholder_text = "do pokoju · /s szept · /k krzyk · Esc — zamknij"
	_input.text_submitted.connect(_submit)
	_input.gui_input.connect(func(e: InputEvent):
		if e is InputEventKey and e.pressed and e.keycode == KEY_ESCAPE:
			close()
			accept_event())
	row.add_child(_input)
	get_viewport().size_changed.connect(_place)


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2((vs.x - size.x) / 2, vs.y - 200)


## Typing: the game doesn't walk or react to keys meanwhile.
func typing() -> bool:
	return visible


func open() -> void:
	visible = true
	_input.text = ""
	_place.call_deferred()
	_input.grab_focus.call_deferred()


func close() -> void:
	visible = false
	_input.release_focus()


func _submit(text: String) -> void:
	text = text.strip_edges()
	close()
	if text != "":
		sent.emit(text)
