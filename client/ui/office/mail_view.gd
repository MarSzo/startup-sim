## Work mail on the office computer: the inbox (list + reading pane +
## writing a new mail) or, with `trash_mode`, the trash (restore / empty).
## The data lives in a MailBox shared by both windows.
extends HBoxContainer

const Ink = preload("res://ui/ink_ui.gd")
const Protocol = preload("res://net/protocol.gd")

const INK := Color("#1c2430")
const MUTED := Color("#6a7383")

var box            # MailBox
var trash_mode := false
## Colleagues one can write to (nicks).
var recipients: Array = []
var _selected := 0
var _composing := false
var _sig := ""

var _list := VBoxContainer.new()
var _pane := VBoxContainer.new()
var _compose := VBoxContainer.new()
var _to := OptionButton.new()
var _subject := LineEdit.new()
var _body := TextEdit.new()


func setup(p_box, p_trash: bool) -> void:
	box = p_box
	trash_mode = p_trash
	box.changed.connect(render)


func _ready() -> void:
	add_theme_constant_override("separation", 10)
	var left := VBoxContainer.new()
	left.custom_minimum_size = Vector2(260, 0)
	left.add_theme_constant_override("separation", 6)
	add_child(left)
	var top := HBoxContainer.new()
	top.add_theme_constant_override("separation", 6)
	if trash_mode:
		var empty := Ink.button("Opróżnij kosz", false, true)
		empty.pressed.connect(func():
			box.act(Protocol.MA_EMPTY_TRASH, 0)
			_selected = 0)
		top.add_child(empty)
	else:
		var write := Ink.button("✎ Napisz", true)
		write.pressed.connect(func(): _start_compose("", "", ""))
		top.add_child(write)
	left.add_child(top)
	var sc := ScrollContainer.new()
	sc.size_flags_vertical = Control.SIZE_EXPAND_FILL
	sc.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_list.add_theme_constant_override("separation", 2)
	sc.add_child(_list)
	left.add_child(sc)

	var right := PanelContainer.new()
	right.add_theme_stylebox_override("panel", Ink.box("card"))
	right.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_child(right)
	var rs := ScrollContainer.new()
	rs.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	right.add_child(rs)
	var rv := VBoxContainer.new()
	rv.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	rs.add_child(rv)
	_pane.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_pane.add_theme_constant_override("separation", 6)
	rv.add_child(_pane)
	_build_compose()
	rv.add_child(_compose)
	render()


func _build_compose() -> void:
	_compose.visible = false
	_compose.add_theme_constant_override("separation", 6)
	_compose.add_child(_label("Nowa wiadomość", 18, INK))
	var r := HBoxContainer.new()
	r.add_theme_constant_override("separation", 6)
	r.add_child(_label("Do:", 15, MUTED))
	_to.custom_minimum_size = Vector2(200, 0)
	r.add_child(_to)
	_compose.add_child(r)
	_subject.placeholder_text = "Temat"
	_subject.max_length = Protocol.MAIL_SUBJECT_MAX / 2
	_subject.add_theme_font_size_override("font_size", 15)
	_compose.add_child(_subject)
	_body.placeholder_text = "Treść…"
	_body.custom_minimum_size = Vector2(0, 150)
	_body.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	_body.add_theme_font_size_override("font_size", 15)
	_compose.add_child(_body)
	var b := HBoxContainer.new()
	b.add_theme_constant_override("separation", 8)
	var send := Ink.button("Wyślij", true)
	send.pressed.connect(_send)
	b.add_child(send)
	var cancel := Ink.button("Anuluj")
	cancel.pressed.connect(func():
		_composing = false
		_sig = ""
		render())
	b.add_child(cancel)
	_compose.add_child(b)


func _start_compose(to: String, subject: String, body: String) -> void:
	_composing = true
	_to.clear()
	for n in recipients:
		_to.add_item(n)
		if n == to:
			_to.select(_to.item_count - 1)
	if _to.item_count == 0:
		_to.add_item("(nikogo nie ma w firmie)")
		_to.disabled = true
	else:
		_to.disabled = false
	_subject.text = subject
	_body.text = body
	_sig = ""
	render()
	_subject.grab_focus.call_deferred()


func _send() -> void:
	if _to.disabled or _to.item_count == 0:
		return
	var to := _to.get_item_text(_to.selected)
	if _subject.text.strip_edges() == "" and _body.text.strip_edges() == "":
		_subject.grab_focus()
		return
	box.act(Protocol.MA_SEND, 0, to, _subject.text.strip_edges(), _body.text.strip_edges())
	_composing = false
	_sig = ""
	render()


func render() -> void:
	if box == null or not is_inside_tree():
		return
	var mails: Array = box.trash() if trash_mode else box.inbox()
	var sig := JSON.stringify([mails.map(func(m): return m.id), box.read.keys(), _selected, _composing, box.trashed.keys()])
	if sig == _sig:
		return
	_sig = sig
	for c in _list.get_children():
		c.queue_free()
	if mails.is_empty():
		_list.add_child(_label("Kosz jest pusty." if trash_mode else "Brak wiadomości.", 15, MUTED))
	for m in mails:
		var b := Button.new()
		var unread: bool = not box.read.has(m.id) and not trash_mode
		b.text = "%s%s\n%s" % ["● " if unread else "", m.from, m.subject]
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.clip_text = true
		b.custom_minimum_size = Vector2(250, 52)
		b.add_theme_font_size_override("font_size", 14)
		for st in ["normal", "hover", "pressed", "focus"]:
			b.add_theme_stylebox_override(st, Ink.box("card_hover" if m.id == _selected or st == "hover" else "card"))
		var id: int = m.id
		b.pressed.connect(func():
			_selected = id
			_composing = false
			box.read[id] = true
			box.changed.emit())
		_list.add_child(b)
	_compose.visible = _composing
	_pane.visible = not _composing
	for c in _pane.get_children():
		c.queue_free()
	var m: Dictionary = box.mails.get(_selected, {})
	if m.is_empty() or (box.trashed.has(_selected) != trash_mode):
		_pane.add_child(_label("Wybierz wiadomość z listy." if not mails.is_empty() else "", 15, MUTED))
		return
	_pane.add_child(_label(m.subject, 20, INK))
	_pane.add_child(_label("Od: %s   ·   godz. %02d:%02d" % [m.from, m.minute / 60, m.minute % 60], 13, MUTED))
	_pane.add_child(HSeparator.new())
	var body := _label(m.body, 16, INK)
	body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_pane.add_child(body)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	if trash_mode:
		var restore := Ink.button("Przywróć", true)
		restore.pressed.connect(func(): box.act(Protocol.MA_RESTORE, m.id))
		row.add_child(restore)
	else:
		if recipients.has(m.from):
			var reply := Ink.button("Odpowiedz", true)
			reply.pressed.connect(func(): _start_compose(m.from, "Re: " + m.subject, "\n\n> " + m.body.replace("\n", "\n> ")))
			row.add_child(reply)
		var del := Ink.button("Do kosza", false, true)
		del.pressed.connect(func():
			box.act(Protocol.MA_TRASH, m.id)
			_selected = 0)
		row.add_child(del)
	_pane.add_child(row)


func _label(text: String, size: int, color: Color) -> Label:
	var l := Label.new()
	Ink.style_label(l, size, color)
	l.text = text
	return l
