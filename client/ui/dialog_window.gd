## A conversation with an NPC (board meeting), the breathalyser's question,
## the R menu or the kitchen cupboard: the text and the answers as buttons
## (or keys 1-9). The server drives it; id 0 closes it.
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


## Above the inventory bar, by the panel's full height (all the answers -
## new buttons only report their size a frame later).
func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	var want := _panel.get_combined_minimum_size()
	if _panel.size != want:
		_panel.size = want
	_panel.position = Vector2((vs.x - want.x) / 2, maxf(8.0, vs.y - want.y - 120))


func _process(_delta: float) -> void:
	if visible and _panel.size != _panel.get_combined_minimum_size():
		_place()


func on_dialog(p: Dictionary) -> void:
	if p.id == 0:
		visible = false
		dialog_id = 0
		_answered = -1  # the same id may come again (the menu, the cupboard)
		return
	if p.id == dialog_id and visible:
		# Resend of what is shown; if our answer got lost, send it again.
		if _answered == p.id and Time.get_ticks_msec() - _answered_at > 1500:
			_answered_at = Time.get_ticks_msec()
			answer.emit(dialog_id, _answer_choice)
		return
	dialog_id = p.id
	_who.text = _title(p)
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


## Ids (server): 1..199 board talks, 200..249 the breathalyser, 250 the R
## menu, 251 the kitchen cupboard.
func _title(p: Dictionary) -> String:
	if p.id == 250:
		return "Co zrobić?"
	if p.id == 251:
		return "Szafka w kuchni"
	if p.id == 255:
		for t in ["Apteczka", "Barek"]:
			if str(p.text).begins_with(t):
				return t
		return "Magazynek"
	if p.id == 253:
		return "Telewizor"
	if p.id == 254:
		return "Boombox"
	if p.id == 252:
		return "Umowa — %s" % name_of.call(p.npc)
	if p.id >= 200:
		return "Alkomat — %s" % name_of.call(p.npc)
	var who: String = name_of.call(p.npc)
	if who == "Przechodzień":
		return "Ktoś pyta o drogę"
	return "Spotkanie — %s" % who


## The header shown (for tests).
func _title_text() -> String:
	return _who.text


func _choose(choice: int) -> void:
	if dialog_id != 0 and _answered != dialog_id:
		_answered = dialog_id
		_answer_choice = choice
		_answered_at = Time.get_ticks_msec()
		answer.emit(dialog_id, choice)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and not event.echo:
		if event.keycode >= KEY_1 and event.keycode <= KEY_9:
			_choose(event.keycode - KEY_1)
			get_viewport().set_input_as_handled()
		elif event.keycode == KEY_ESCAPE and dialog_id in [250, 251, 253, 254, 255]:
			_choose(_opts.get_child_count() - 1)  # the last option: never mind / close
			get_viewport().set_input_as_handled()
