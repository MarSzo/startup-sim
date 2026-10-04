## Tab: what can be done here and now - a list of actions with their keys;
## pick one with its number (or a click), Tab / Esc closes it. The game
## gives the list (game.gd `_actions_here`).
extends Control

const Ink = preload("res://ui/ink_ui.gd")

var _panel := PanelContainer.new()
var _list := VBoxContainer.new()
var _runs: Array[Callable] = []


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	_panel.add_theme_stylebox_override("panel", Ink.box("paper"))
	_panel.custom_minimum_size = Vector2(380, 0)
	add_child(_panel)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 8)
	_panel.add_child(col)
	var title := Label.new()
	Ink.style_label(title, 16, Ink.ACCENT)
	title.text = "Co mogę teraz zrobić?  (Tab — zamknij)"
	col.add_child(title)
	_list.add_theme_constant_override("separation", 6)
	col.add_child(_list)
	get_viewport().size_changed.connect(_place)


## `actions`: [{"key": "E", "text": "Porozmawiaj: HR", "run": Callable}, ...]
func open(actions: Array) -> void:
	for c in _list.get_children():
		c.queue_free()
	_runs.clear()
	for a in actions:
		var i := _runs.size()
		_runs.append(a.run)
		var b := Ink.button("%d.  [%s]  %s" % [i + 1, a.key, a.text])
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.focus_mode = Control.FOCUS_NONE
		b.pressed.connect(func(): _pick(i))
		_list.add_child(b)
	if actions.is_empty():
		var l := Label.new()
		Ink.style_label(l, 16, Ink.TEXT_MUTED)
		l.text = "Nic tu nie ma do zrobienia — podejdź do czegoś albo do kogoś."
		_list.add_child(l)
	visible = true
	_place.call_deferred()


func close() -> void:
	visible = false


func _pick(i: int) -> void:
	if i < _runs.size():
		var run := _runs[i]
		close()
		run.call()


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.reset_size()
	_panel.size = _panel.get_combined_minimum_size()
	_panel.position = Vector2((vs.x - _panel.size.x) / 2, (vs.y - _panel.size.y) / 2)


func _unhandled_input(event: InputEvent) -> void:
	if not visible or not (event is InputEventKey and event.pressed and not event.echo):
		return
	if event.physical_keycode in [KEY_TAB, KEY_ESCAPE]:
		close()
		get_viewport().set_input_as_handled()
	elif event.physical_keycode >= KEY_1 and event.physical_keycode <= KEY_9:
		_pick(event.physical_keycode - KEY_1)
		get_viewport().set_input_as_handled()
