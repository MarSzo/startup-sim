## The department's task board (a small kanban) in the office browser:
## three columns of cards (priority stripe, assignee, comments), a form for
## new cards and a detail pane (priority, assignee, description, comments).
## Actions carry a nonce and are resent until the board comes back with it
## as `done`; while shown, the board is re-synced every SYNC_MSEC.
extends VBoxContainer

const Ink = preload("res://ui/ink_ui.gd")
const Protocol = preload("res://net/protocol.gd")

## TaskAction to send.
signal send(nonce: int, action: int, task: int, arg: int, text: String)

const SYNC_MSEC := 1500
const RESEND_MSEC := 800
const COLUMNS := ["Do zrobienia", "W toku", "Zrobione"]
const PRIORITIES := ["Niski", "Średni", "Pilny"]
const PRIO_COLORS := [Color("#8fa37a"), Color("#e0a82e"), Color("#c0392b")]
const Departments = preload("res://net/departments.gd")
const INK := Color("#1c2430")
const MUTED := Color("#6a7383")

var me := ""                 # the account's nick (the computer's owner)
var dept := 0
var members: Array = []
var tasks: Array = []        # cards, as sent (sorted by the server)
var detail := {}             # the open card's desc + comments
var open_id := 0
var _parts := {}             # part -> tasks, until all arrived
var _nonce := randi_range(1, 30000)
var _queue: Array = []       # [nonce, action, task, arg, text, msec]
var _last_sync := -SYNC_MSEC
var _sig := ""
var _detail_sig := ""

var _head := Label.new()
var _cols: Array[VBoxContainer] = []
var _col_titles: Array[Label] = []
var _form := PanelContainer.new()
var _f_title := LineEdit.new()
var _f_desc := TextEdit.new()
var _f_prio := OptionButton.new()
var _pane := PanelContainer.new()
var _p_title := Label.new()
var _p_meta := Label.new()
var _p_prio := OptionButton.new()
var _p_who := OptionButton.new()
var _p_desc := TextEdit.new()
var _p_comments := VBoxContainer.new()
var _p_comment := LineEdit.new()
var _p_move: Array[Button] = []


func _ready() -> void:
	add_theme_constant_override("separation", 8)
	var top := HBoxContainer.new()
	top.add_theme_constant_override("separation", 10)
	Ink.style_label(_head, 20, INK)
	_head.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(_head)
	var add := Ink.button("+ Nowe zadanie", true)
	add.pressed.connect(func():
		_form.visible = not _form.visible
		if _form.visible:
			_f_title.grab_focus())
	top.add_child(add)
	add_child(top)

	# New card form.
	_form.add_theme_stylebox_override("panel", Ink.box("card"))
	_form.visible = false
	var fc := VBoxContainer.new()
	fc.add_theme_constant_override("separation", 6)
	_form.add_child(fc)
	var fr := HBoxContainer.new()
	fr.add_theme_constant_override("separation", 8)
	_line(_f_title, "Tytuł zadania", Protocol.TASK_TITLE_MAX)
	_f_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	fr.add_child(_f_title)
	for i in PRIORITIES.size():
		_f_prio.add_item(PRIORITIES[i], i)
	_f_prio.select(1)
	fr.add_child(_f_prio)
	fc.add_child(fr)
	_text(_f_desc, "Opis (opcjonalnie)")
	fc.add_child(_f_desc)
	var fb := HBoxContainer.new()
	fb.add_theme_constant_override("separation", 8)
	var ok := Ink.button("Dodaj", true)
	ok.pressed.connect(_create)
	fb.add_child(ok)
	var cancel := Ink.button("Anuluj")
	cancel.pressed.connect(func(): _form.visible = false)
	fb.add_child(cancel)
	fc.add_child(fb)
	add_child(_form)

	# Columns + the detail pane on the right.
	var main := HBoxContainer.new()
	main.size_flags_vertical = Control.SIZE_EXPAND_FILL
	main.add_theme_constant_override("separation", 8)
	add_child(main)
	for i in COLUMNS.size():
		var col := VBoxContainer.new()
		col.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		col.add_theme_constant_override("separation", 4)
		var t := Label.new()
		Ink.style_label(t, 16, Ink.ACCENT)
		col.add_child(t)
		_col_titles.append(t)
		var sc := ScrollContainer.new()
		sc.size_flags_vertical = Control.SIZE_EXPAND_FILL
		sc.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		var list := VBoxContainer.new()
		list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		list.add_theme_constant_override("separation", 6)
		sc.add_child(list)
		col.add_child(sc)
		_cols.append(list)
		var bg := PanelContainer.new()
		var sb := StyleBoxFlat.new()
		sb.bg_color = Color(0, 0, 0, 0.05)
		sb.set_content_margin_all(6)
		sb.set_corner_radius_all(4)
		bg.add_theme_stylebox_override("panel", sb)
		bg.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		bg.add_child(col)
		main.add_child(bg)
	_build_pane()
	main.add_child(_pane)


func _build_pane() -> void:
	_pane.add_theme_stylebox_override("panel", Ink.box("card"))
	_pane.custom_minimum_size = Vector2(300, 0)
	_pane.visible = false
	var sc := ScrollContainer.new()
	sc.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_pane.add_child(sc)
	var v := VBoxContainer.new()
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	v.add_theme_constant_override("separation", 6)
	sc.add_child(v)
	var top := HBoxContainer.new()
	Ink.style_label(_p_title, 18, INK)
	_p_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_p_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(_p_title)
	var x := Ink.button("X")
	x.pressed.connect(func(): _open(0))
	top.add_child(x)
	v.add_child(top)
	Ink.style_label(_p_meta, 13, MUTED)
	v.add_child(_p_meta)
	var mv := HBoxContainer.new()
	mv.add_theme_constant_override("separation", 4)
	for i in COLUMNS.size():
		var b := Ink.button(COLUMNS[i])
		b.add_theme_font_size_override("font_size", 14)
		var col: int = i
		b.pressed.connect(func(): _act(Protocol.TA_MOVE, open_id, col, ""))
		mv.add_child(b)
		_p_move.append(b)
	v.add_child(mv)
	var pr := HBoxContainer.new()
	pr.add_theme_constant_override("separation", 6)
	pr.add_child(_small("Priorytet:"))
	for i in PRIORITIES.size():
		_p_prio.add_item(PRIORITIES[i], i)
	_p_prio.item_selected.connect(func(i: int): _act(Protocol.TA_PRIORITY, open_id, i, ""))
	pr.add_child(_p_prio)
	v.add_child(pr)
	var wr := HBoxContainer.new()
	wr.add_theme_constant_override("separation", 6)
	wr.add_child(_small("Kto:"))
	_p_who.item_selected.connect(func(i: int): _act(Protocol.TA_ASSIGN, open_id, 0, _p_who.get_item_metadata(i)))
	wr.add_child(_p_who)
	var mine := Ink.button("Biorę")
	mine.add_theme_font_size_override("font_size", 14)
	mine.pressed.connect(func(): _act(Protocol.TA_ASSIGN, open_id, 0, me))
	wr.add_child(mine)
	v.add_child(wr)
	v.add_child(_small("Opis:"))
	_text(_p_desc, "Brak opisu")
	v.add_child(_p_desc)
	var save := Ink.button("Zapisz opis")
	save.add_theme_font_size_override("font_size", 14)
	save.pressed.connect(func(): _act(Protocol.TA_EDIT, open_id, 0, "%s\n%s" % [_card(open_id).get("title", ""), _p_desc.text]))
	v.add_child(save)
	v.add_child(_small("Komentarze:"))
	_p_comments.add_theme_constant_override("separation", 4)
	v.add_child(_p_comments)
	var cr := HBoxContainer.new()
	_line(_p_comment, "Napisz komentarz…", Protocol.TASK_COMMENT_MAX)
	_p_comment.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_p_comment.text_submitted.connect(func(_t): _comment())
	cr.add_child(_p_comment)
	var cb := Ink.button("Dodaj", true)
	cb.pressed.connect(_comment)
	cr.add_child(cb)
	v.add_child(cr)
	var del := Ink.button("Usuń zadanie", false, true)
	del.pressed.connect(func():
		_act(Protocol.TA_DELETE, open_id, 0, "")
		_open(0))
	v.add_child(del)


# ------------------------------------------------------------------ network

## Every TaskBoard part; the board is applied when all parts are in.
func on_board(p: Dictionary) -> void:
	if p.part == 0:
		_parts.clear()
		members = p.members
		dept = p.dept
	_parts[p.part] = p.tasks
	if _parts.size() < p.parts:
		return
	var all := []
	for i in p.parts:
		all.append_array(_parts.get(i, []))
	tasks = all
	# Everything up to `done` got through.
	while not _queue.is_empty() and _nonce_le(_queue[0][0], p.done):
		_queue.pop_front()
	if open_id != 0 and _card(open_id).is_empty():
		_open(0)  # deleted
	_render()


func on_detail(p: Dictionary) -> void:
	if p.id == open_id:
		detail = p
		_render_pane()


## Called every frame while the board is on screen.
func tick() -> void:
	var now := Time.get_ticks_msec()
	if not _queue.is_empty() and now - _queue[0][5] >= RESEND_MSEC:
		var q: Array = _queue[0]
		q[5] = now
		send.emit(q[0], q[1], q[2], q[3], q[4])
	elif now - _last_sync >= SYNC_MSEC:
		_last_sync = now
		send.emit(0, Protocol.TA_SYNC, open_id, 0, "")


## The account changed (another computer): forget the board.
func reset(nick: String) -> void:
	me = nick
	tasks = []
	members = []
	detail = {}
	_queue.clear()
	_open(0)
	_sig = ""
	_render()


static func _nonce_le(a: int, b: int) -> bool:
	return ((b - a) & 0xffff) < 0x8000


func _act(action: int, task: int, arg: int, text: String) -> void:
	if task == 0 and action != Protocol.TA_CREATE:
		return
	_nonce = _nonce % 65535 + 1
	var q := [_nonce, action, task, arg, text, Time.get_ticks_msec()]
	_queue.append(q)
	if _queue.size() == 1:
		send.emit(q[0], q[1], q[2], q[3], q[4])
	_last_sync = Time.get_ticks_msec()


func _create() -> void:
	var title := _f_title.text.strip_edges()
	if title == "":
		_f_title.grab_focus()
		return
	_act(Protocol.TA_CREATE, 0, _f_prio.get_selected_id(), "%s\n%s" % [title, _f_desc.text.strip_edges()])
	_f_title.text = ""
	_f_desc.text = ""
	_form.visible = false


func _comment() -> void:
	var t := _p_comment.text.strip_edges()
	if t != "":
		_act(Protocol.TA_COMMENT, open_id, 0, t)
		_p_comment.text = ""


func _open(id: int) -> void:
	open_id = id
	detail = {}
	_detail_sig = ""
	_pane.visible = id != 0
	if id != 0:
		_last_sync = -SYNC_MSEC  # the details at once
		_render_pane()


func _card(id: int) -> Dictionary:
	for t in tasks:
		if t.id == id:
			return t
	return {}


# ------------------------------------------------------------------- render

func _render() -> void:
	_head.text = "Tablica zadań — %s" % Departments.name_of(dept, "dział")
	var sig := JSON.stringify([tasks, open_id])
	if sig != _sig:
		_sig = sig
		for i in _cols.size():
			for c in _cols[i].get_children():
				c.queue_free()
		var counts := [0, 0, 0]
		for t in tasks:
			var col: int = clampi(t.column, 0, 2)
			counts[col] += 1
			_cols[col].add_child(_card_view(t))
		for i in _cols.size():
			_col_titles[i].text = "%s (%d)" % [COLUMNS[i], counts[i]]
			if counts[i] == 0:
				_cols[i].add_child(_small("—"))
	_render_pane()


func _card_view(t: Dictionary) -> Control:
	var panel := PanelContainer.new()
	panel.add_theme_stylebox_override("panel", Ink.box("card_hover" if t.id == open_id else "card"))
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	panel.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	var id: int = t.id
	panel.gui_input.connect(func(ev):
		if ev is InputEventMouseButton and ev.pressed and ev.button_index == MOUSE_BUTTON_LEFT:
			_open(0 if open_id == id else id))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	panel.add_child(row)
	var stripe := ColorRect.new()
	stripe.color = PRIO_COLORS[clampi(t.priority, 0, 2)]
	stripe.custom_minimum_size = Vector2(5, 0)
	stripe.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(stripe)
	var v := VBoxContainer.new()
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	v.add_theme_constant_override("separation", 0)
	v.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(v)
	var title := Label.new()
	Ink.style_label(title, 15, INK)
	title.text = t.title
	title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	title.mouse_filter = Control.MOUSE_FILTER_IGNORE
	v.add_child(title)
	var meta := "@%s" % t.assignee if t.assignee != "" else "nieprzypisane"
	if t.comments > 0:
		meta += "  ·  %d kom." % t.comments
	var m := _small(meta)
	m.mouse_filter = Control.MOUSE_FILTER_IGNORE
	v.add_child(m)
	# Quick moves: ← →.
	var arrows := HBoxContainer.new()
	arrows.alignment = BoxContainer.ALIGNMENT_END
	for dir in [-1, 1]:
		var col: int = t.column + dir
		if col < 0 or col > 2:
			continue
		var b := Ink.button("←" if dir < 0 else "→")
		b.add_theme_font_size_override("font_size", 13)
		b.tooltip_text = COLUMNS[col]
		b.pressed.connect(func(): _act(Protocol.TA_MOVE, id, col, ""))
		arrows.add_child(b)
	v.add_child(arrows)
	return panel


func _render_pane() -> void:
	if open_id == 0:
		return
	var t := _card(open_id)
	if t.is_empty():
		return
	var sig := JSON.stringify([t, detail, members])
	if sig == _detail_sig:
		return
	_detail_sig = sig
	_p_title.text = t.title
	_p_meta.text = "Autor: %s · %s" % [t.author, COLUMNS[clampi(t.column, 0, 2)]]
	for i in _p_move.size():
		_p_move[i].disabled = i == t.column
	_p_prio.select(clampi(t.priority, 0, 2))
	_p_who.clear()
	_p_who.add_item("— nikt —")
	_p_who.set_item_metadata(0, "")
	var names: Array = members.duplicate()
	if t.assignee != "" and not names.has(t.assignee):
		names.append(t.assignee)
	for n in names:
		_p_who.add_item(n)
		_p_who.set_item_metadata(_p_who.item_count - 1, n)
		if n == t.assignee:
			_p_who.select(_p_who.item_count - 1)
	if t.assignee == "":
		_p_who.select(0)
	if not _p_desc.has_focus():
		_p_desc.text = detail.get("desc", "")
	for c in _p_comments.get_children():
		c.queue_free()
	var comments: Array = detail.get("comments", [])
	if comments.is_empty():
		_p_comments.add_child(_small("Jeszcze bez komentarzy."))
	for c in comments:
		var l := Label.new()
		Ink.style_label(l, 14, INK)
		l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		l.text = "%s: %s" % [c[0], c[1]]
		_p_comments.add_child(l)


# ------------------------------------------------------------------ widgets

func _small(text: String) -> Label:
	var l := Label.new()
	Ink.style_label(l, 13, MUTED)
	l.text = text
	return l


func _line(e: LineEdit, placeholder: String, max_bytes: int) -> void:
	e.placeholder_text = placeholder
	e.max_length = max_bytes / 2
	e.custom_minimum_size = Vector2(0, 34)
	e.add_theme_font_size_override("font_size", 15)


func _text(e: TextEdit, placeholder: String) -> void:
	e.placeholder_text = placeholder
	e.custom_minimum_size = Vector2(0, 70)
	e.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	e.add_theme_font_size_override("font_size", 15)
