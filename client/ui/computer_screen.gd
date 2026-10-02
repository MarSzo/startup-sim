## The screen of a computer on a desk: a desktop like the one at home (icons,
## windows, a taskbar) with the company messenger, work mail (+ trash), the
## browser (bookmarks: lunch ordering, the department's task board), the
## calendar and — for the founder — the company panel; or the lock screen.
## Shown while the server says we sit at a computer (self_status bit); the
## computer is logged in as its owner, whoever sits at it.
extends Control

const Protocol = preload("res://net/protocol.gd")
const ItemArt = preload("res://game/item_art.gd")
const Ink = preload("res://ui/ink_ui.gd")
const Desktop = preload("res://ui/desktop.gd")
const OsWindow = preload("res://ui/office/os_window.gd")
const KanbanView = preload("res://ui/office/kanban_view.gd")
const MailBox = preload("res://ui/office/mail_box.gd")
const MailView = preload("res://ui/office/mail_view.gd")

## ComputerAction to send: action, conversation, argument, text.
signal action(action: int, conv: int, arg: int, text: String)
## Calendar: book `start` for `topic` (0 = cancel).
signal book(start: int, topic: int)
## Lunch app: order `dish`.
signal order(dish: int)
## Company panel (founder): CompanyAction.
signal company_action(action: int, target: int, value: int, text: String)
## Task board: TaskAction.
signal task_action(nonce: int, action: int, task: int, arg: int, text: String)
## Work mail: MailAction.
signal mail_action(nonce: int, action: int, id: int, to: String, subject: String, body: String)

const Departments = preload("res://net/departments.gd")
const SYNC_MSEC := 1000
const RESEND_MSEC := 800
const MAX_TRIES := 5
const NICK_COLORS := [Color("#2e6bd9"), Color("#c0392b"), Color("#16a085"), Color("#8e44ad"), Color("#d35400"), Color("#2c3e50"), Color("#b7950b")]

var my_id := 0
## Name of a player id (the game's PlayerInfo cache).
var name_of: Callable = func(_id: int) -> String: return "?"

var seated := false
var state := {}              # last Computer packet
var chats := {}              # "owner:conv" -> Array of messages, by id
var current := Protocol.CONV_GENERAL
var _pending := {}           # message being sent: nonce, conv, text, msec, tries
var _last_sync := 0
var _sig := ""               # sidebar currently shown (re-render on change)
var _msg_sig := ""           # messages currently shown

var _dim := ColorRect.new()
var _frame := PanelContainer.new()
var _screen := VBoxContainer.new()
var _title := Label.new()
var _as_owner := Label.new()
var _body := HBoxContainer.new()
var _sidebar := VBoxContainer.new()
var _conv_title := Label.new()
var _scroll := ScrollContainer.new()
var _messages := VBoxContainer.new()
var _entry := LineEdit.new()
var _send_btn: Button
var _lock_view := VBoxContainer.new()
var _lock_owner := Label.new()
var _lock_hint := Label.new()
var _unlock_btn: Button
var _chat_view := VBoxContainer.new()
var tab := "chat"               # the last app opened by a dev command
var calendar := {}              # last Calendar packet
var _cal_view := VBoxContainer.new()
var _cal_mine := Label.new()
var _cal_topic := OptionButton.new()
var _cal_list := VBoxContainer.new()
var _cal_sig := ""
var lunch := {}                 # last LunchMenu packet
var _lunch_view := VBoxContainer.new()
var _lunch_status := Label.new()
var _lunch_list := VBoxContainer.new()
var _lunch_sig := ""
## Company panel: the server sends CompanyOffers / CompanyPeople only to the
## founder at their own computer; the tab shows while they keep coming.
const COMPANY_FRESH_MSEC := 3000
var company_offers := {}       # {name, sets, offers} once all parts are in
var _co_parts := {}            # parts arriving (CompanyOffers comes in pieces)
var _co_got := 0
var company_people := {}
var _company_msec := -COMPANY_FRESH_MSEC
var _co_view := VBoxContainer.new()
var _co_sig := ""
var _co_drafts := {}            # text typed into the panel's fields, by key
# The desktop: icons, windows, the taskbar; apps open in windows.
const WINDOW_TITLES := {"chat": "Komunikator", "mail": "Poczta", "trash": "Kosz", "browser": "Przeglądarka",
	"calendar": "Kalendarz zarządu", "company": "Panel firmy"}
var _desk := Control.new()
var _win_layer := Control.new()
var _windows := {}              # name -> OsWindow
var _views := {}                # name -> Control shown in that window
var _task_btns := HBoxContainer.new()
var _icons := {}                # name -> icon Button
var _badges := {}               # name -> Label
var _browser := VBoxContainer.new()
var _url := Label.new()
var _pages := {}                # page -> Control
var page := "home"              # browser page: home / lunch / tasks
var kanban := KanbanView.new()
var mailbox := MailBox.new()
var _mail_view := MailView.new()
var _trash_view := MailView.new()
var _start := MenuButton.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP
	visible = false
	get_viewport().size_changed.connect(_fit)
	_build()
	_fit()


func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size
	_dim.size = size
	var fs := Vector2(minf(1200, size.x - 40), minf(740, size.y - 40))
	_frame.size = fs
	_frame.position = (size - fs) / 2


# ------------------------------------------------------------------- state

func on_computer(p: Dictionary) -> void:
	if p.owner != mailbox.owner:
		# Another account (computer): its own mail and board.
		mailbox.reset(p.owner)
		kanban.reset(name_of.call(p.owner))
	state = p
	var ids: Array = p.convs.map(func(c): return c.conv)
	if not ids.is_empty() and not ids.has(current):
		current = Protocol.CONV_GENERAL
	_show()


func on_chat(p: Dictionary) -> void:
	if state.is_empty():
		return
	var key := _key(p.conv)
	var list: Array = chats.get(key, [])
	var known := {}
	for m in list:
		known[m.id] = true
	for m in p.messages:
		if not known.has(m.id):
			list.append(m)
		if not _pending.is_empty() and m.from == state.owner and m.text == _pending.text:
			_pending = {}
	list.sort_custom(func(a, b): return a.id < b.id)
	while list.size() > 100:
		list.pop_front()
	chats[key] = list
	if p.conv == current:
		_render_messages()


## Server's AT_COMPUTER status bit (every snapshot).
func set_seated(on: bool) -> void:
	if on == seated:
		return
	seated = on
	if not on:
		state = {}
		_pending = {}
		_sig = ""
		_msg_sig = ""
	_show()


func _show() -> void:
	var was := visible
	visible = seated and not state.is_empty()
	if not visible:
		return
	_render()
	if not was:
		_last_sync = 0
		if not state.locked and _windows.has("chat"):
			_entry.grab_focus.call_deferred()


func _key(conv: int) -> String:
	return "%d:%d" % [state.get("owner", 0), conv]


func _conv(conv: int) -> Dictionary:
	for c in state.get("convs", []):
		if c.conv == conv:
			return c
	return {}


func _process(_d: float) -> void:
	if not visible or state.is_empty() or state.locked:
		return
	mailbox.tick()
	if _windows.has("browser") and page == "tasks":
		kanban.tick()
	var now := Time.get_ticks_msec()
	if now - _last_sync >= SYNC_MSEC and _windows.has("chat"):
		_last_sync = now
		action.emit(Protocol.PC_SYNC, current, _last_id(current), "")
	if not _pending.is_empty() and now - _pending.msec >= RESEND_MSEC:
		if _pending.tries >= MAX_TRIES:
			_pending = {}
			_render_input()
		else:
			_pending.tries += 1
			_pending.msec = now
			action.emit(Protocol.PC_SEND, _pending.conv, _pending.nonce, _pending.text)


func _last_id(conv: int) -> int:
	var list: Array = chats.get(_key(conv), [])
	return list[-1].id if not list.is_empty() else 0


func _input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_ESCAPE:
		action.emit(Protocol.PC_CLOSE, 0, 0, "")
		get_viewport().set_input_as_handled()


# ------------------------------------------------------------------ actions

func _select(conv: int) -> void:
	current = conv
	_render()
	action.emit(Protocol.PC_SYNC, conv, _last_id(conv), "")
	_entry.grab_focus.call_deferred()


func _send() -> void:
	var text := _entry.text.strip_edges()
	if text == "" or not _pending.is_empty():
		return
	_pending = {"nonce": randi_range(1, 0x7fffffff), "conv": current, "text": text, "msec": Time.get_ticks_msec(), "tries": 1}
	action.emit(Protocol.PC_SEND, current, _pending.nonce, text)
	_entry.text = ""
	_render_input()


## Dev (--goto): pc:say:<conv>:<text>, pc:open:<conv>, pc:lock, pc:unlock,
## pc:take, pc:close; <conv> = general / dept / dm:<nick>; pc:win:<app>
## (chat / mail / trash / browser / calendar / company / lunch / tasks),
## pc:task:<title> (a new card), pc:mail:<nick>:<subject> (send a mail),
## pc:card:<n> (open the n-th card), pc:read:<n> (read the n-th mail),
## pc:comment:<text> (on the open card), pc:take-card (assign it to me).
func dev_command(cmd: String) -> void:
	var parts := cmd.split(":")
	if parts.size() > 2 and parts[1] == "dm":  # dm:<nick> is one token
		parts[1] = "dm:" + parts[2]
		parts.remove_at(2)
	if parts.size() > 3:  # the text may contain ':'
		parts[2] = ":".join(parts.slice(2))
		parts.resize(3)
	match parts[0]:
		"win":
			if parts.size() > 1:
				_set_tab(parts[1])
		"task":
			if parts.size() > 1:
				_set_tab("tasks")
				kanban._act(Protocol.TA_CREATE, 0, 2, cmd.substr(5) + "\nDodane z linii poleceń.")
		"mail":
			var mp := cmd.split(":", true, 2)
			if mp.size() > 2:
				_set_tab("mail")
				mailbox.act(Protocol.MA_SEND, 0, mp[1], mp[2], "Wiadomość testowa.")
		"card":
			if parts.size() > 1 and int(parts[1]) < kanban.tasks.size():
				kanban._open(kanban.tasks[int(parts[1])].id)
		"comment":
			kanban._p_comment.text = cmd.substr(8)
			kanban._comment()
		"take-card":
			kanban._act(Protocol.TA_ASSIGN, kanban.open_id, 0, kanban.me)
		"read":
			var list: Array = mailbox.inbox()
			if parts.size() > 1 and int(parts[1]) < list.size():
				_mail_view._selected = list[int(parts[1])].id
				mailbox.read[_mail_view._selected] = true
				mailbox.changed.emit()
		"lock": action.emit(Protocol.PC_LOCK, 0, 0, "")
		"unlock": action.emit(Protocol.PC_UNLOCK, 0, 0, "")
		"take": action.emit(Protocol.PC_TAKE, 0, 0, "")
		"close": action.emit(Protocol.PC_CLOSE, 0, 0, "")
		"company":  # company[:<action>:<target>:<value>[:<text>]]
			_set_tab("company")
			var co := cmd.split(":", true, 4)
			if co.size() > 1 and co[1] == "hire":  # the first candidate
				for c in company_people.get("candidates", []):
					company_action.emit(Protocol.CO_HIRE, c.id, 0, "")
					break
			elif co.size() > 3:
				company_action.emit(int(co[1]), int(co[2]), int(co[3]), co[4] if co.size() > 4 else "")
		"lunch":  # lunch:<dish kind>
			_set_tab("lunch")
			if parts.size() > 1:
				order.emit(int(parts[1]))
		"cal":  # cal:<hh*60+mm>:<topic>
			_set_tab("calendar")
			if parts.size() > 2:
				book.emit(int(parts[1]), int(parts[2]))
		"open", "say":
			if parts.size() < 2 or state.is_empty():
				return
			var conv := _conv_by_token(parts[1])
			if conv < 0:
				return
			_select(conv)
			if parts[0] == "say" and parts.size() > 2:
				_entry.text = parts[2]
				_send()


func _conv_by_token(tok: String) -> int:
	if tok == "general":
		return Protocol.CONV_GENERAL
	for c in state.convs:
		if tok == "dept" and c.conv >= Protocol.CONV_DEPARTMENT_BASE and c.conv < Protocol.CONV_DM:
			return c.conv
		if tok.begins_with("dm:") and c.conv & Protocol.CONV_DM and c.title == tok.substr(3):
			return c.conv
	return -1


# ------------------------------------------------------------------ layout

func _build() -> void:
	_dim.color = Color(0.02, 0.03, 0.06, 0.6)
	_dim.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(_dim)
	# A pixel monitor: dark bezel, light screen, blue title bar.
	_frame.add_theme_stylebox_override("panel", Ink.box("screen"))
	add_child(_frame)
	var screen_bg := PanelContainer.new()
	var ssb := StyleBoxFlat.new()
	ssb.bg_color = Ink.PAPER
	screen_bg.add_theme_stylebox_override("panel", ssb)
	_frame.add_child(screen_bg)
	_screen.add_theme_constant_override("separation", 0)
	screen_bg.add_child(_screen)

	# The desktop (wallpaper, icons, windows) over the taskbar.
	_desk.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_desk.clip_contents = true
	_screen.add_child(_desk)
	var wall := TextureRect.new()
	wall.set_anchors_preset(Control.PRESET_FULL_RECT)
	wall.texture = Desktop.wallpaper()
	wall.stretch_mode = TextureRect.STRETCH_SCALE
	wall.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_desk.add_child(wall)
	var icons := VBoxContainer.new()
	icons.position = Vector2(14, 12)
	icons.add_theme_constant_override("separation", 6)
	_desk.add_child(icons)
	for ic in [["mail", "Poczta", "mail"], ["browser", "Przeglądarka", "browser"], ["chat", "Komunikator", "chat"],
			["calendar", "Kalendarz", "calendar"], ["company", "Firma", "company"], ["trash", "Kosz", "trash"]]:
		var b := _desk_icon(ic[1], ic[2])
		var name: String = ic[0]
		b.pressed.connect(func(): _open(name))
		icons.add_child(b)
		_icons[name] = b
	_win_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	_win_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_desk.add_child(_win_layer)
	# Taskbar: StartOS (lock / take the laptop / close), open windows, account.
	var bar := PanelContainer.new()
	bar.add_theme_stylebox_override("panel", Ink.box("hud"))
	_screen.add_child(bar)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	bar.add_child(row)
	_start.text = "◆ StartOS"
	_start.flat = false
	for st in ["normal", "hover", "pressed", "focus"]:
		_start.add_theme_stylebox_override(st, Ink.button_box(st if st != "focus" else "normal", true))
	for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		_start.add_theme_color_override(k, Color.WHITE)
	var pm := _start.get_popup()
	pm.add_item("Zablokuj komputer", 1)
	pm.add_item("Zabierz laptop", 2)
	pm.add_separator()
	pm.add_item("Zamknij (Esc)", 3)
	pm.id_pressed.connect(_start_menu)
	row.add_child(_start)
	_task_btns.add_theme_constant_override("separation", 6)
	_task_btns.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(_task_btns)
	_style_label(_as_owner, 14, Color("#ffcf6e"))
	row.add_child(_as_owner)
	_style_label(_title, 14, Color(1, 1, 1, 0.85))
	row.add_child(_title)

	# Browser: address, bookmarks, the page.
	_browser.add_theme_constant_override("separation", 6)
	var addr := PanelContainer.new()
	addr.add_theme_stylebox_override("panel", Ink.box("input"))
	_style_label(_url, 14, Color("#4a5566"))
	addr.add_child(_url)
	_browser.add_child(addr)
	var marks := HBoxContainer.new()
	marks.add_theme_constant_override("separation", 6)
	marks.add_child(_mini_label("Ulubione:"))
	for bm in [["home", "⌂ Start"], ["lunch", "★ Obiady do biura"], ["tasks", "★ Tablica zadań"]]:
		var mb := Ink.button(bm[1])
		mb.add_theme_font_size_override("font_size", 14)
		var pg: String = bm[0]
		mb.pressed.connect(func(): _go(pg))
		marks.add_child(mb)
	_browser.add_child(marks)
	_browser.add_child(HSeparator.new())
	var home := VBoxContainer.new()
	home.add_theme_constant_override("separation", 12)
	home.add_child(_mini_label("Strona startowa"))
	var ht := Label.new()
	_style_label(ht, 22, Color("#1c2430"))
	ht.text = "Ulubione"
	home.add_child(ht)
	var tiles := HBoxContainer.new()
	tiles.add_theme_constant_override("separation", 14)
	for bm in [["lunch", "Obiady do biura", "lunchbox.example — dostawa na recepcję", "company"],
			["tasks", "Tablica zadań", "tasks.startup — zadania działu (kanban)", "tasks"]]:
		var tb := Button.new()
		tb.custom_minimum_size = Vector2(260, 130)
		for st in ["normal", "hover", "pressed", "focus"]:
			tb.add_theme_stylebox_override(st, Ink.box("card_hover" if st == "hover" else "card"))
		var pic := Control.new()
		pic.position = Vector2(88, 6)
		pic.size = Vector2(84, 52)
		pic.mouse_filter = Control.MOUSE_FILTER_IGNORE
		var kind: String = bm[3]
		pic.draw.connect(func(): Desktop.draw_icon(pic, kind))
		tb.add_child(pic)
		var tl := Label.new()
		_style_label(tl, 17, Color("#1c2430"))
		tl.text = bm[1] + "\n" + bm[2]
		tl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		tl.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		tl.position = Vector2(8, 62)
		tl.size = Vector2(244, 60)
		tl.mouse_filter = Control.MOUSE_FILTER_IGNORE
		tb.add_child(tl)
		var pg: String = bm[0]
		tb.pressed.connect(func(): _go(pg))
		tiles.add_child(tb)
	home.add_child(tiles)
	_pages["home"] = home
	_pages["tasks"] = kanban
	kanban.send.connect(func(n: int, a: int, t: int, arg: int, text: String): task_action.emit(n, a, t, arg, text))
	mailbox.send.connect(func(n: int, a: int, id: int, to: String, subj: String, body: String): mail_action.emit(n, a, id, to, subj, body))
	mailbox.arrived.connect(_on_new_mail)
	mailbox.changed.connect(_update_badges)
	_mail_view.setup(mailbox, false)
	_trash_view.setup(mailbox, true)
	_views["mail"] = _mail_view
	_views["trash"] = _trash_view
	_views["browser"] = _browser

	# Messenger.
	_chat_view.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_views["chat"] = _chat_view
	_body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_body.add_theme_constant_override("separation", 0)
	_chat_view.add_child(_body)
	var side := PanelContainer.new()
	var sdb := StyleBoxFlat.new()
	sdb.bg_color = Ink.DARK
	sdb.set_content_margin_all(10)
	side.add_theme_stylebox_override("panel", sdb)
	side.custom_minimum_size = Vector2(230, 0)
	_body.add_child(side)
	var side_scroll := ScrollContainer.new()
	side_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	side.add_child(side_scroll)
	_sidebar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_sidebar.add_theme_constant_override("separation", 2)
	side_scroll.add_child(_sidebar)
	var main := VBoxContainer.new()
	main.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	main.add_theme_constant_override("separation", 0)
	_body.add_child(main)
	var head := MarginContainer.new()
	for s in ["left", "right", "top", "bottom"]:
		head.add_theme_constant_override("margin_" + s, 12)
	_style_label(_conv_title, 18, Color("#1c2430"))
	head.add_child(_conv_title)
	main.add_child(head)
	main.add_child(HSeparator.new())
	_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	main.add_child(_scroll)
	var pad := MarginContainer.new()
	pad.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for s in ["left", "right", "top", "bottom"]:
		pad.add_theme_constant_override("margin_" + s, 14)
	_scroll.add_child(pad)
	_messages.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_messages.add_theme_constant_override("separation", 10)
	pad.add_child(_messages)
	var in_row := HBoxContainer.new()
	in_row.add_theme_constant_override("separation", 8)
	var in_pad := MarginContainer.new()
	for s in ["left", "right", "top", "bottom"]:
		in_pad.add_theme_constant_override("margin_" + s, 10)
	in_pad.add_child(in_row)
	main.add_child(in_pad)
	_entry.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_entry.custom_minimum_size = Vector2(0, 40)
	_entry.max_length = 200
	_entry.add_theme_font_size_override("font_size", 16)
	_entry.add_theme_stylebox_override("normal", Ink.box("input"))
	_entry.add_theme_stylebox_override("focus", Ink.box("input_focus"))
	_entry.add_theme_color_override("font_color", Color("#1c2430"))
	_entry.add_theme_color_override("font_placeholder_color", Color("#8a93a3"))
	_entry.add_theme_color_override("caret_color", Color("#1c2430"))
	_entry.text_submitted.connect(func(_t): _send())
	in_row.add_child(_entry)
	_send_btn = _button("Wyślij", true)
	_send_btn.pressed.connect(_send)
	in_row.add_child(_send_btn)

	# Calendar (the board's meetings, today).
	_cal_view.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_cal_view.add_theme_constant_override("separation", 10)
	var cal_pad := MarginContainer.new()
	cal_pad.size_flags_vertical = Control.SIZE_EXPAND_FILL
	for sd in ["left", "right", "top", "bottom"]:
		cal_pad.add_theme_constant_override("margin_" + sd, 18)
	cal_pad.add_child(_cal_view)
	_views["calendar"] = cal_pad
	var ch := Label.new()
	_style_label(ch, 20, Color("#1c2430"))
	ch.text = "Kalendarz zarządu — spotkania na dziś"
	_cal_view.add_child(ch)
	_style_label(_cal_mine, 15, Ink.ACCENT)
	_cal_view.add_child(_cal_mine)
	var trow := HBoxContainer.new()
	trow.add_theme_constant_override("separation", 10)
	var tl := Label.new()
	_style_label(tl, 15, Color("#1c2430"))
	tl.text = "Temat:"
	trow.add_child(tl)
	for t in Protocol.TOPICS:
		_cal_topic.add_item("%s (%s)" % [Protocol.TOPICS[t], Protocol.TOPIC_WITH[t]], t)
	trow.add_child(_cal_topic)
	_cal_view.add_child(trow)
	var cs := ScrollContainer.new()
	cs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	cs.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_cal_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_cal_list.add_theme_constant_override("separation", 4)
	cs.add_child(_cal_list)
	_cal_view.add_child(cs)
	cal_pad.name = "cal_pad"

	# Lunch app.
	var lunch_pad := MarginContainer.new()
	lunch_pad.size_flags_vertical = Control.SIZE_EXPAND_FILL
	for sd in ["left", "right", "top", "bottom"]:
		lunch_pad.add_theme_constant_override("margin_" + sd, 18)
	lunch_pad.name = "lunch_pad"
	_lunch_view.add_theme_constant_override("separation", 10)
	lunch_pad.add_child(_lunch_view)
	_pages["lunch"] = lunch_pad
	var lh := Label.new()
	_style_label(lh, 20, Color("#1c2430"))
	lh.text = "Obiady do biura — dostawa na recepcję (piętro 4)"
	_lunch_view.add_child(lh)
	_style_label(_lunch_status, 15, Ink.ACCENT)
	_lunch_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_lunch_status.custom_minimum_size = Vector2(600, 0)
	_lunch_view.add_child(_lunch_status)
	_lunch_list.add_theme_constant_override("separation", 6)
	_lunch_view.add_child(_lunch_list)

	# Company panel (founder).
	var co_pad := MarginContainer.new()
	co_pad.size_flags_vertical = Control.SIZE_EXPAND_FILL
	for sd in ["left", "right", "top", "bottom"]:
		co_pad.add_theme_constant_override("margin_" + sd, 18)
	co_pad.name = "company_pad"
	var co_scroll := ScrollContainer.new()
	co_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	co_pad.add_child(co_scroll)
	_co_view.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_co_view.add_theme_constant_override("separation", 8)
	co_scroll.add_child(_co_view)
	_views["company"] = co_pad

	# Browser pages under the bookmarks.
	for pg in _pages:
		_pages[pg].size_flags_vertical = Control.SIZE_EXPAND_FILL
		_pages[pg].visible = pg == page
		_browser.add_child(_pages[pg])

	# Lock screen: over the whole desktop.
	var lock_bg := PanelContainer.new()
	var lsb := StyleBoxFlat.new()
	lsb.bg_color = Ink.PAPER
	lock_bg.add_theme_stylebox_override("panel", lsb)
	lock_bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	lock_bg.name = "lock_bg"
	_desk.add_child(lock_bg)
	_lock_view.alignment = BoxContainer.ALIGNMENT_CENTER
	_lock_view.add_theme_constant_override("separation", 14)
	lock_bg.add_child(_lock_view)
	var icon := Control.new()
	icon.custom_minimum_size = Vector2(0, 72)
	icon.draw.connect(func(): _draw_lock(icon))
	_lock_view.add_child(icon)
	_style_label(_lock_owner, 26, Color("#1c2430"))
	_lock_owner.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_lock_view.add_child(_lock_owner)
	_style_label(_lock_hint, 16, Color("#5a6475"))
	_lock_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_lock_view.add_child(_lock_hint)
	var brow := HBoxContainer.new()
	brow.alignment = BoxContainer.ALIGNMENT_CENTER
	_unlock_btn = _button("Odblokuj (odcisk palca)", true)
	_unlock_btn.pressed.connect(func(): action.emit(Protocol.PC_UNLOCK, 0, 0, ""))
	brow.add_child(_unlock_btn)
	_lock_view.add_child(brow)


func _draw_lock(c: Control) -> void:
	var cx := c.size.x / 2
	c.draw_arc(Vector2(cx, 30), 14, PI, TAU, 16, Color("#5a6475"), 6)
	c.draw_line(Vector2(cx - 14, 30), Vector2(cx - 14, 38), Color("#5a6475"), 6)
	c.draw_line(Vector2(cx + 14, 30), Vector2(cx + 14, 38), Color("#5a6475"), 6)
	c.draw_rect(Rect2(cx - 24, 36, 48, 34), Color("#e0a82e"))
	c.draw_rect(Rect2(cx - 3, 46, 6, 12), Color("#7a5a10"))


func _render() -> void:
	if state.is_empty():
		return
	var owner_name: String = name_of.call(state.owner)
	var mine: bool = state.owner == my_id
	_title.text = "Konto: %s" % owner_name
	_as_owner.text = "" if mine else "Uwaga: działasz jako %s!" % owner_name
	_desk.get_node("lock_bg").visible = state.locked
	_start.disabled = false
	_icons["company"].visible = _company_fresh()
	if not _company_fresh() and _windows.has("company"):
		_close("company")
	_update_badges()
	if state.locked:
		_lock_owner.text = "%s — zablokowany" % owner_name
		_lock_hint.text = "Przyłóż palec do czytnika, żeby odblokować." if mine else "Tylko %s może go odblokować. Laptop możesz najwyżej zabrać." % owner_name
		_unlock_btn.visible = mine
		return
	if _windows.has("calendar"):
		_render_calendar()
	if _windows.has("company"):
		_render_company()
	if _windows.has("browser") and page == "lunch":
		_render_lunch()
	if _windows.has("chat"):
		_render_sidebar()
		var c := _conv(current)
		_conv_title.text = c.get("title", "")
		_render_messages()
		_render_input()
	# Colleagues to write to: the messenger's private conversations.
	var who := []
	for c in state.get("convs", []):
		if c.conv & Protocol.CONV_DM:
			who.append(c.title)
	_mail_view.recipients = who


## Open (or bring to the front) an app window.
func _open(name: String) -> void:
	if state.is_empty() or state.locked:
		return
	if not _windows.has(name):
		var w := OsWindow.new()
		var view: Control = _views[name]
		if view.get_parent():
			view.get_parent().remove_child(view)
		w.setup(WINDOW_TITLES[name], view, 10)
		var k := _windows.size()
		var ds := _desk.size
		w.position = Vector2(118 + k * 22, 8 + k * 16)
		w.size = Vector2(maxf(420, ds.x - 132 - k * 22), maxf(300, ds.y - 16 - k * 16))
		w.closed.connect(func(): _close(name))
		w.focused.connect(func(): _win_layer.move_child(w, -1))
		_win_layer.add_child(w)
		_windows[name] = w
		var tb := Ink.button(WINDOW_TITLES[name])
		tb.add_theme_font_size_override("font_size", 14)
		tb.name = "task_" + name
		tb.pressed.connect(func(): _open(name))
		_task_btns.add_child(tb)
		if name == "chat":
			_select(current)
		elif name == "calendar":
			_cal_sig = ""
		elif name == "company":
			_co_sig = ""
		elif name == "browser":
			_go(page)
	_win_layer.move_child(_windows[name], -1)
	_render()


func _close(name: String) -> void:
	if not _windows.has(name):
		return
	var w: Control = _windows[name]
	var view: Control = _views[name]
	view.get_parent().remove_child(view)  # the app keeps its state
	w.queue_free()
	_windows.erase(name)
	var t := _task_btns.get_node_or_null("task_" + name)
	if t:
		_task_btns.remove_child(t)
		t.queue_free()


## Browser: show a page (home / lunch / tasks).
func _go(p: String) -> void:
	page = p
	for k in _pages:
		_pages[k].visible = k == p
	_url.text = {"home": "  🔒  start.os/ulubione", "lunch": "  🔒  https://lunchbox.example/biuro", "tasks": "  🔒  https://tasks.startup/tablica"}[p]
	if _windows.has("browser"):
		_windows["browser"].set_title("Przeglądarka — %s" % {"home": "Start", "lunch": "Obiady do biura", "tasks": "Tablica zadań"}[p])
	_lunch_sig = ""
	_render()


func _desk_icon(caption: String, kind: String) -> Button:
	var b := Button.new()
	b.flat = true
	b.focus_mode = Control.FOCUS_NONE
	b.custom_minimum_size = Vector2(90, 74)
	var pic := Control.new()
	pic.position = Vector2(3, 0)
	pic.size = Vector2(84, 50)
	pic.mouse_filter = Control.MOUSE_FILTER_IGNORE
	pic.draw.connect(func(): Desktop.draw_icon(pic, kind))
	b.add_child(pic)
	var l := Label.new()
	Ink.style_label(l, 15, Color.WHITE)
	l.text = caption
	l.position = Vector2(0, 50)
	l.size = Vector2(90, 22)
	l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	l.add_theme_constant_override("outline_size", 4)
	l.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.6))
	l.mouse_filter = Control.MOUSE_FILTER_IGNORE
	b.add_child(l)
	var badge := Label.new()
	Ink.style_label(badge, 14, Color.WHITE)
	var bs := StyleBoxFlat.new()
	bs.bg_color = Ink.RED
	bs.border_color = Ink.INK
	bs.set_border_width_all(2)
	bs.set_corner_radius_all(9)
	bs.content_margin_left = 5
	bs.content_margin_right = 5
	badge.add_theme_stylebox_override("normal", bs)
	badge.position = Vector2(58, -2)
	badge.visible = false
	badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
	b.add_child(badge)
	_badges[kind] = badge
	return b


## StartOS menu: lock / take the laptop / close.
func _start_menu(id: int) -> void:
	var what := {1: Protocol.PC_LOCK, 2: Protocol.PC_TAKE, 3: Protocol.PC_CLOSE}
	if what.has(id):
		action.emit(what[id], 0, 0, "")


func _mini_label(text: String) -> Label:
	var l := Label.new()
	_style_label(l, 14, Color("#6a7383"))
	l.text = text
	return l


## Unread mail / chat messages on the icons.
func _update_badges() -> void:
	var counts := {"mail": mailbox.unread(), "chat": 0, "trash": mailbox.trash().size()}
	for c in state.get("convs", []):
		counts.chat += c.unread
	for k in counts:
		if _badges.has(k):
			_badges[k].text = str(counts[k])
			_badges[k].visible = counts[k] > 0 and k != "trash"


func _on_new_mail(_m: Dictionary) -> void:
	var audio = preload("res://audio/audio.gd").inst
	if audio:
		audio.play("notify", -4.0)
	_update_badges()


# ------------------------------------------------------------ office apps

func on_task_board(p: Dictionary) -> void:
	kanban.on_board(p)


func on_task_detail(p: Dictionary) -> void:
	kanban.on_detail(p)


func on_work_mail(p: Dictionary) -> void:
	mailbox.on_mail(p)


func on_mail_state(p: Dictionary) -> void:
	mailbox.on_state(p)


func _set_tab(t: String) -> void:
	tab = t
	match t:
		"lunch":
			_open("browser")
			_go("lunch")
		"tasks":
			_open("browser")
			_go("tasks")
		_:
			_open(t)


# ----------------------------------------------------------------- company

func _company_fresh() -> bool:
	return not company_offers.is_empty() and Time.get_ticks_msec() - _company_msec < COMPANY_FRESH_MSEC


func on_company(p: Dictionary) -> void:
	var was := _company_fresh()
	if p.type == Protocol.T_COMPANY_OFFERS:
		# The positions come in parts: put them together first.
		if p.part == 0:
			_co_parts = {"name": p.name, "sets": p.sets, "offers": []}
			_co_got = 0
		elif _co_parts.is_empty():
			return
		_co_parts.offers.append_array(p.offers)
		_co_got += 1
		if _co_got < p.parts:
			return
		company_offers = _co_parts.duplicate(true)
		_co_parts = {}
	else:
		company_people = p
	_company_msec = Time.get_ticks_msec()
	if not visible:
		return
	if not was:
		_render()  # show the tab
	elif _windows.has("company"):
		_render_company()


func _co_label(text: String, size: int, color := Color("#1c2430"), wrap := false) -> Label:
	var l := Label.new()
	_style_label(l, size, color)
	l.text = text
	if wrap:
		l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		l.custom_minimum_size = Vector2(600, 0)
	return l


## A LineEdit whose typed text survives re-renders.
func _co_edit(key: String, value: String, max_len: int, width: int) -> LineEdit:
	var e := LineEdit.new()
	e.text = _co_drafts.get(key, value)
	e.max_length = max_len
	e.custom_minimum_size = Vector2(width, 32)
	e.add_theme_font_size_override("font_size", 14)
	e.add_theme_color_override("font_color", Color("#1c2430"))
	e.add_theme_color_override("font_placeholder_color", Color("#8a93a3"))
	e.add_theme_stylebox_override("normal", Ink.box("input"))
	e.add_theme_stylebox_override("focus", Ink.box("input_focus"))
	e.text_changed.connect(func(t: String): _co_drafts[key] = t)
	return e


## One position in the founder's panel: name, department, question set,
## places, description, remove.
func _position_card(o: Dictionary) -> Control:
	var id: int = o.id
	var places: int = o.places
	var card := Ink.panel("card")
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	card.add_child(box)
	var r1 := HBoxContainer.new()
	r1.add_theme_constant_override("separation", 8)
	box.add_child(r1)
	var tkey := "title:%d" % id
	var title := _co_edit(tkey, o.title, 40, 240)
	r1.add_child(title)
	var rename := _button("Zmień nazwę", false)
	rename.add_theme_color_override("font_color", Color("#1c2430"))
	rename.pressed.connect(func():
		_co_drafts.erase(tkey)
		company_action.emit(Protocol.CO_SET_TITLE, id, 0, title.text.strip_edges()))
	r1.add_child(rename)
	var dept := _dept_select(o.department)
	dept.item_selected.connect(func(i: int): company_action.emit(Protocol.CO_SET_DEPARTMENT, id, dept.get_item_id(i), ""))
	r1.add_child(dept)
	var qs := _set_select(o.set)
	qs.item_selected.connect(func(i: int): company_action.emit(Protocol.CO_SET_QUESTIONS, id, 0, qs.get_item_metadata(i)))
	r1.add_child(qs)
	var del := Ink.button("Usuń", false, true)
	del.tooltip_text = "Zamyka rekrutację (zatrudnieni zostają)"
	del.pressed.connect(func(): company_action.emit(Protocol.CO_REMOVE_POSITION, id, 0, ""))
	r1.add_child(del)
	var r2 := HBoxContainer.new()
	r2.add_theme_constant_override("separation", 8)
	box.add_child(r2)
	var minus := _button("−", false)
	minus.add_theme_color_override("font_color", Color("#1c2430"))
	minus.disabled = places == 0
	minus.pressed.connect(func(): company_action.emit(Protocol.CO_SET_PLACES, id, places - 1, ""))
	r2.add_child(minus)
	var n := _co_label("%d miejsc" % places if places != 1 else "1 miejsce", 15, Color("#16a085") if places else Color("#8a93a3"))
	n.custom_minimum_size = Vector2(80, 0)
	n.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	r2.add_child(n)
	var plus := _button("+", false)
	plus.add_theme_color_override("font_color", Color("#1c2430"))
	plus.disabled = places >= Protocol.CO_MAX_PLACES
	plus.pressed.connect(func(): company_action.emit(Protocol.CO_SET_PLACES, id, places + 1, ""))
	r2.add_child(plus)
	var key := "desc:%d" % id
	var desc := _co_edit(key, o.description, 200, 380)
	desc.placeholder_text = "Opis stanowiska"
	r2.add_child(desc)
	var save := _button("Zapisz opis", false)
	save.add_theme_color_override("font_color", Color("#1c2430"))
	save.pressed.connect(func():
		_co_drafts.erase(key)
		company_action.emit(Protocol.CO_SET_DESCRIPTION, id, 0, desc.text.strip_edges()))
	r2.add_child(save)
	return card


## "Nowe stanowisko": name, department, question set, description.
func _new_position_card(count: int) -> Control:
	var card := Ink.panel("card")
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	card.add_child(box)
	box.add_child(_co_label("Nowe stanowisko", 16, Ink.ACCENT))
	var r1 := HBoxContainer.new()
	r1.add_theme_constant_override("separation", 8)
	box.add_child(r1)
	var title := _co_edit("new:title", "", 40, 240)
	title.placeholder_text = "Nazwa, np. Office manager"
	r1.add_child(title)
	var dept := _dept_select(int(_co_drafts.get("new:dept", 1)))
	dept.item_selected.connect(func(i: int): _co_drafts["new:dept"] = dept.get_item_id(i))
	r1.add_child(dept)
	var qs := _set_select(str(_co_drafts.get("new:set", "general")))
	qs.item_selected.connect(func(i: int): _co_drafts["new:set"] = qs.get_item_metadata(i))
	r1.add_child(qs)
	var r2 := HBoxContainer.new()
	r2.add_theme_constant_override("separation", 8)
	box.add_child(r2)
	var desc := _co_edit("new:desc", "", 200, 500)
	desc.placeholder_text = "Opis stanowiska (opcjonalnie)"
	r2.add_child(desc)
	var add := _button("Dodaj stanowisko", true)
	add.disabled = count >= Protocol.CO_MAX_POSITIONS
	add.pressed.connect(func():
		var t := title.text.strip_edges()
		if t.length() < 3:
			title.grab_focus()
			return
		var set_id: String = qs.get_item_metadata(qs.selected)
		company_action.emit(Protocol.CO_ADD_POSITION, 0, dept.get_item_id(dept.selected), "%s\n%s\n%s" % [t, set_id, desc.text.strip_edges()])
		for k in ["new:title", "new:desc"]:
			_co_drafts.erase(k))
	r2.add_child(add)
	if count >= Protocol.CO_MAX_POSITIONS:
		box.add_child(_co_label("Limit 10 stanowisk — usuń któreś, żeby dodać nowe.", 14, Color("#8a93a3")))
	return card


func _dept_select(selected: int) -> OptionButton:
	var o := OptionButton.new()
	for d in Departments.for_positions():
		o.add_item(Departments.name_of(d), d)
		if d == selected:
			o.select(o.item_count - 1)
	o.add_theme_font_size_override("font_size", 14)
	return o


func _set_select(selected: String) -> OptionButton:
	var o := OptionButton.new()
	for s in company_offers.get("sets", []):
		o.add_item("Pytania: " + s.name)
		o.set_item_metadata(o.item_count - 1, s.id)
		if s.id == selected:
			o.select(o.item_count - 1)
	o.add_theme_font_size_override("font_size", 14)
	return o


func _co_row() -> HBoxContainer:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	_co_view.add_child(row)
	return row


func _render_company() -> void:
	if company_offers.is_empty():
		return
	var sig := JSON.stringify([company_offers, company_people])
	if sig == _co_sig:
		return
	_co_sig = sig
	for c in _co_view.get_children():
		c.queue_free()
	var titles := {}
	for o in company_offers.offers:
		titles[o.id] = o.title

	_co_view.add_child(_co_label("Panel założyciela", 22))
	var row := _co_row()
	row.add_child(_co_label("Nazwa firmy:", 15, Color("#4a5566")))
	var name_edit := _co_edit("name", company_offers.name, 40, 360)
	row.add_child(name_edit)
	var rename := _button("Zmień", true)
	rename.pressed.connect(func():
		_co_drafts.erase("name")
		company_action.emit(Protocol.CO_RENAME, 0, 0, name_edit.text.strip_edges()))
	row.add_child(rename)

	var positions: Array = company_offers.offers
	_co_view.add_child(_co_label("Stanowiska (%d/%d) — ogłoszenia na portalu" % [positions.size(), Protocol.CO_MAX_POSITIONS], 18, Ink.ACCENT))
	for o in positions:
		_co_view.add_child(_position_card(o))
	_co_view.add_child(_new_position_card(positions.size()))

	_co_view.add_child(_co_label("Kandydaci po rozmowie", 18, Ink.ACCENT))
	var cands: Array = company_people.get("candidates", [])
	if cands.is_empty():
		_co_view.add_child(_co_label("Nikt nie czeka. Kandydaci, o których nie zdecydujesz w 30 min, są zatrudniani automatycznie.", 14, Color("#8a93a3"), true))
	for c in cands:
		var pid: int = c.id
		row = _co_row()
		var l := _co_label("%s — %s, wynik rozmowy %d/%d" % [c.nick, titles.get(c.offer, "?"), c.score, c.total], 15)
		l.custom_minimum_size = Vector2(430, 0)
		row.add_child(l)
		var hire := _button("Zatrudnij", true)
		hire.pressed.connect(func(): company_action.emit(Protocol.CO_HIRE, pid, 0, ""))
		row.add_child(hire)
		var rej := Ink.button("Odrzuć", false, true)
		rej.pressed.connect(func(): company_action.emit(Protocol.CO_REJECT, pid, 0, ""))
		row.add_child(rej)

	_co_view.add_child(_co_label("Zespół", 18, Ink.ACCENT))
	var staff: Array = company_people.get("staff", [])
	if staff.is_empty():
		_co_view.add_child(_co_label("Na razie tylko Ty.", 14, Color("#8a93a3")))
	for s in staff:
		var pid: int = s.id
		row = _co_row()
		var l := _co_label("%s — %s, od dnia %d" % [s.nick, Departments.name_of(s.department), s.day], 15)
		l.custom_minimum_size = Vector2(430, 0)
		row.add_child(l)
		if pid == my_id:
			row.add_child(_co_label("(Ty)", 14, Color("#8a93a3")))
			continue
		var fire := Ink.button("Zwolnij", false, true)
		fire.pressed.connect(func(): company_action.emit(Protocol.CO_FIRE, pid, 0, ""))
		row.add_child(fire)


func on_lunch(p: Dictionary) -> void:
	lunch = p
	if visible and _windows.has("browser") and page == "lunch":
		_render_lunch()


func _render_lunch() -> void:
	if lunch.is_empty():
		_lunch_status.text = "Ładowanie menu…"
		return
	var sig := JSON.stringify(lunch)
	if sig == _lunch_sig:
		return
	_lunch_sig = sig
	var dish_name := ""
	for d in lunch.dishes:
		if d.kind == lunch.dish:
			dish_name = d.name
	match lunch.state:
		Protocol.LUNCH_ORDERED:
			_lunch_status.text = "Zamówione: %s — kurier będzie ok. %s." % [dish_name, _hhmm(lunch.arrives)]
		Protocol.LUNCH_WAITING:
			_lunch_status.text = "%s czeka na recepcji — odbierz przy biurku recepcji (E)." % dish_name
		Protocol.LUNCH_CLOSED:
			_lunch_status.text = "Zamówienia przyjmujemy od 10:00 do 15:00."
		_:
			_lunch_status.text = "Wybierz danie — płaci konto właściciela komputera (%s)." % name_of.call(state.get("owner", 0))
	for c in _lunch_list.get_children():
		c.queue_free()
	for d in lunch.dishes:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 12)
		var icon := Control.new()
		icon.custom_minimum_size = Vector2(32, 32)
		var k: int = d.kind
		icon.draw.connect(func(): ItemArt.draw(icon, k, Vector2.ZERO, 2.0))
		row.add_child(icon)
		var info := VBoxContainer.new()
		info.custom_minimum_size = Vector2(320, 0)
		info.add_theme_constant_override("separation", 0)
		var n := Label.new()
		_style_label(n, 16, Color("#1c2430"))
		n.text = d.name
		info.add_child(n)
		var r := Label.new()
		_style_label(r, 13, Color("#8a93a3"))
		r.text = "%s · ok. %d min" % [d.restaurant, d.eta]
		info.add_child(r)
		row.add_child(info)
		var price := Label.new()
		_style_label(price, 16, Color("#8f5a1a"))
		price.text = "%d,%02d zł" % [d.price / 100, d.price % 100]
		price.custom_minimum_size = Vector2(90, 0)
		row.add_child(price)
		var b := _button("Zamów", true)
		b.disabled = lunch.state != Protocol.LUNCH_NONE
		b.modulate = Color(1, 1, 1, 0.4 if b.disabled else 1.0)
		b.pressed.connect(func(): order.emit(k))
		row.add_child(b)
		_lunch_list.add_child(row)


func on_calendar(p: Dictionary) -> void:
	calendar = p
	if visible and _windows.has("calendar"):
		_render_calendar()


static func _hhmm(m: int) -> String:
	return "%02d:%02d" % [m / 60, m % 60]


func _render_calendar() -> void:
	if calendar.is_empty():
		_cal_mine.text = "Ładowanie…"
		return
	var sig := JSON.stringify(calendar)
	if sig == _cal_sig:
		return
	_cal_sig = sig
	if calendar.mine_start != Protocol.NO_TIME:
		_cal_mine.text = "Twoje spotkanie: %s — %s (%s). Drzwi zarządu otworzą się 10 min wcześniej." % [
			_hhmm(calendar.mine_start), Protocol.TOPICS.get(calendar.mine_topic, "?"), Protocol.TOPIC_WITH.get(calendar.mine_topic, "?")]
	else:
		_cal_mine.text = "Nie masz dziś spotkania z zarządem."
	for c in _cal_list.get_children():
		c.queue_free()
	for s in calendar.slots:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 12)
		var t := Label.new()
		_style_label(t, 16, Color("#1c2430"))
		t.text = _hhmm(s.start)
		t.custom_minimum_size = Vector2(60, 0)
		row.add_child(t)
		var st := Label.new()
		st.custom_minimum_size = Vector2(160, 0)
		var start: int = s.start
		match s.state:
			Protocol.SLOT_FREE:
				_style_label(st, 15, Color("#27ae60"))
				st.text = "wolne"
				row.add_child(st)
				var b := _button("Umów", true)
				b.pressed.connect(func(): book.emit(start, _cal_topic.get_selected_id()))
				row.add_child(b)
			Protocol.SLOT_TAKEN:
				_style_label(st, 15, Color("#8a93a3"))
				st.text = "zajęte"
				row.add_child(st)
			Protocol.SLOT_MINE:
				_style_label(st, 15, Ink.ACCENT)
				st.text = "Twoje spotkanie"
				row.add_child(st)
				var b2 := _button("Odwołaj", true)
				b2.pressed.connect(func(): book.emit(start, 0))
				row.add_child(b2)
			_:
				_style_label(st, 15, Color("#c3c9d1"))
				st.text = "—"
				row.add_child(st)
		_cal_list.add_child(row)


func _render_sidebar() -> void:
	var sig := JSON.stringify([state.convs, current])
	if sig == _sig:
		return
	_sig = sig
	for ch in _sidebar.get_children():
		ch.queue_free()
	var header := func(text: String):
		var l := Label.new()
		_style_label(l, 12, Color(1, 1, 1, 0.45))
		l.text = text
		_sidebar.add_child(l)
	header.call("KANAŁY")
	var dm_header := false
	for c in state.convs:
		if c.conv & Protocol.CONV_DM and not dm_header:
			dm_header = true
			var gap := Control.new()
			gap.custom_minimum_size = Vector2(0, 10)
			_sidebar.add_child(gap)
			header.call("WIADOMOŚCI PRYWATNE")
		var b := Button.new()
		b.text = c.title + ("   (%d)" % c.unread if c.unread > 0 and c.conv != current else "")
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.add_theme_font_size_override("font_size", 15)
		var sb := StyleBoxFlat.new()
		sb.set_content_margin_all(6)
		sb.content_margin_left = 10
		sb.bg_color = Ink.ACCENT if c.conv == current else Color(0, 0, 0, 0)
		var hover := sb.duplicate()
		hover.bg_color = Ink.ACCENT_HI if c.conv == current else Color(1, 1, 1, 0.08)
		for st in ["normal", "focus"]:
			b.add_theme_stylebox_override(st, sb)
		b.add_theme_stylebox_override("hover", hover)
		b.add_theme_stylebox_override("pressed", hover)
		var bold: bool = c.unread > 0 and c.conv != current
		var fc := Color.WHITE if bold or c.conv == current else Color(1, 1, 1, 0.72)
		for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
			b.add_theme_color_override(k, fc)
		var conv: int = c.conv
		b.pressed.connect(func(): _select(conv))
		_sidebar.add_child(b)


func _render_messages() -> void:
	var list: Array = chats.get(_key(current), [])
	var sig := "%s/%d/%d" % [_key(current), list.size(), list[-1].id if not list.is_empty() else 0]
	if sig == _msg_sig:
		return
	_msg_sig = sig
	for ch in _messages.get_children():
		ch.queue_free()
	if list.is_empty():
		var l := Label.new()
		_style_label(l, 15, Color("#8a93a3"))
		l.text = "Jeszcze nic tu nie ma. Napisz coś jako pierwszy!"
		_messages.add_child(l)
	for m in list:
		var box := VBoxContainer.new()
		box.add_theme_constant_override("separation", 1)
		var who := Label.new()
		_style_label(who, 14, NICK_COLORS[m.from % NICK_COLORS.size()])
		who.text = m.nick + ("  (Ty)" if m.from == my_id else "")
		box.add_child(who)
		var text := Label.new()
		_style_label(text, 16, Color("#1c2430"))
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		text.text = m.text
		box.add_child(text)
		_messages.add_child(box)
	_scroll_to_end.call_deferred()


func _scroll_to_end() -> void:
	await get_tree().process_frame
	_scroll.scroll_vertical = int(_scroll.get_v_scroll_bar().max_value)


func _render_input() -> void:
	var sending := not _pending.is_empty()
	_send_btn.disabled = sending
	_send_btn.text = "Wysyłanie…" if sending else "Wyślij"
	_entry.placeholder_text = "Napisz na %s…" % _conv(current).get("title", "")


func _style_label(l: Label, size: int, color: Color) -> void:
	Ink.style_label(l, size, color)


func _button(text: String, primary: bool) -> Button:
	return Ink.button(text, primary)
