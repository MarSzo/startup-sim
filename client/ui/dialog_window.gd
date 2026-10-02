## A conversation with an NPC (board meeting) or the panel of floor buttons
## in a lift (npc 0): the question and the answers as buttons (or keys 1-4).
## The server drives it; id 0 closes it.
extends Control

const Ink = preload("res://ui/ink_ui.gd")

signal answer(id: int, choice: int)

var dialog_id := 0
var name_of: Callable = func(_id: int) -> String: return "?"
var _panel := PanelContainer.new()
var _who := Label.new()
var _text := Label.new()
var _opts := VBoxContainer.new()
var _answered := -1   # id already answered (wait for the next question)
var _answer_choice := 0
var _answered_at := 0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	_panel.add_theme_stylebox_override("panel", Ink.box("paper"))
	_panel.custom_minimum_size = Vector2(560, 0)
	add_child(_panel)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 10)
	_panel.add_child(col)
	Ink.style_label(_who, 16, Ink.ACCENT)
	col.add_child(_who)
	Ink.style_label(_text, 20, Ink.TEXT_INK)
	_text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_text.custom_minimum_size = Vector2(528, 0)  # wrap width (else it measures as a tall column)
	col.add_child(_text)
	_opts.add_theme_constant_override("separation", 6)
	col.add_child(_opts)
	_panel.resized.connect(_place)
	get_viewport().size_changed.connect(_place)


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.position = Vector2((vs.x - _panel.size.x) / 2, vs.y - _panel.size.y - 120)


func on_dialog(p: Dictionary) -> void:
	if p.id == 0:
		visible = false
		dialog_id = 0
		return
	if p.id == dialog_id and visible:
		# Resend of what is shown; if our answer got lost, send it again.
		if _answered == p.id and Time.get_ticks_msec() - _answered_at > 1500:
			_answered_at = Time.get_ticks_msec()
			answer.emit(dialog_id, _answer_choice)
		return
	dialog_id = p.id
	_who.text = "Winda" if p.npc == 0 else "Spotkanie — %s" % name_of.call(p.npc)
	_text.text = p.text
	for c in _opts.get_children():
		c.queue_free()
	for i in p.options.size():
		var b := Ink.button("%d. %s" % [i + 1, p.options[i]])
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.focus_mode = Control.FOCUS_NONE
		var choice: int = i
		b.pressed.connect(func(): _choose(choice))
		_opts.add_child(b)
	visible = true
	_panel.reset_size()
	_place.call_deferred()


func _choose(choice: int) -> void:
	if dialog_id != 0 and _answered != dialog_id:
		_answered = dialog_id
		_answer_choice = choice
		_answered_at = Time.get_ticks_msec()
		answer.emit(dialog_id, choice)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and not event.echo:
		if event.keycode >= KEY_1 and event.keycode <= KEY_4:
			_choose(event.keycode - KEY_1)
			get_viewport().set_input_as_handled()
