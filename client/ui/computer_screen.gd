## The screen of a computer on a desk (GDD 9a, step 4): the company messenger
## (#ogólny, the department channel, private messages) or the lock screen.
## Shown while the server says we sit at a computer (self_status bit); the
## computer is logged in as its owner, whoever sits at it.
extends Control

const Protocol = preload("res://net/protocol.gd")
const ItemArt = preload("res://game/item_art.gd")
const PixelUI = preload("res://ui/pixel_ui.gd")

## ComputerAction to send: action, conversation, argument, text.
signal action(action: int, conv: int, arg: int, text: String)
## Calendar: book `start` for `topic` (0 = cancel).
signal book(start: int, topic: int)
## Lunch app: order `dish`.
signal order(dish: int)
## Company panel (founder): CompanyAction.
signal company_action(action: int, target: int, value: int, text: String)

const DEPARTMENTS := {1: "IT / Produkt", 2: "Biznes", 3: "Zarząd"}
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
var _lock_btn: Button
var _take_btn: Button
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
var tab := "chat"               # chat / calendar
var calendar := {}              # last Calendar packet
var _cal_view := VBoxContainer.new()
var _cal_mine := Label.new()
var _cal_topic := OptionButton.new()
var _cal_list := VBoxContainer.new()
var _tab_chat: Button
var _tab_cal: Button
var _cal_sig := ""
var lunch := {}                 # last LunchMenu packet
var _lunch_view := VBoxContainer.new()
var _lunch_status := Label.new()
var _lunch_list := VBoxContainer.new()
var _lunch_sig := ""
var _tab_lunch: Button
## Company panel: the server sends CompanyOffers / CompanyPeople only to the
## founder at their own computer; the tab shows while they keep coming.
const COMPANY_FRESH_MSEC := 3000
var company_offers := {}
var company_people := {}
var _company_msec := -COMPANY_FRESH_MSEC
var _co_view := VBoxContainer.new()
var _co_sig := ""
var _co_drafts := {}            # text typed into the panel's fields, by key
var _tab_company: Button


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
	var fs := Vector2(minf(1040, size.x - 60), minf(660, size.y - 60))
	_frame.size = fs
	_frame.position = (size - fs) / 2


# ------------------------------------------------------------------- state

func on_computer(p: Dictionary) -> void:
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
		if not state.locked:
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
	var now := Time.get_ticks_msec()
	if now - _last_sync >= SYNC_MSEC:
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
## pc:take, pc:close; <conv> = general / dept / dm:<nick>.
func dev_command(cmd: String) -> void:
	var parts := cmd.split(":")
	if parts.size() > 2 and parts[1] == "dm":  # dm:<nick> is one token
		parts[1] = "dm:" + parts[2]
		parts.remove_at(2)
	if parts.size() > 3:  # the text may contain ':'
		parts[2] = ":".join(parts.slice(2))
		parts.resize(3)
	match parts[0]:
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
	_frame.add_theme_stylebox_override("panel", PixelUI.box("screen"))
	add_child(_frame)
	var screen_bg := PanelContainer.new()
	var ssb := StyleBoxFlat.new()
	ssb.bg_color = PixelUI.PAPER
	screen_bg.add_theme_stylebox_override("panel", ssb)
	_frame.add_child(screen_bg)
	_screen.add_theme_constant_override("separation", 0)
	screen_bg.add_child(_screen)

	# Top bar: app name, whose account, lock / take / close.
	var bar := PanelContainer.new()
	bar.add_theme_stylebox_override("panel", PixelUI.box("title"))
	_screen.add_child(bar)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	bar.add_child(row)
	_style_label(_title, 16, Color.WHITE)
	row.add_child(_title)
	_tab_chat = _button("Komunikator", false)
	_tab_chat.pressed.connect(func(): _set_tab("chat"))
	row.add_child(_tab_chat)
	_tab_cal = _button("Kalendarz", false)
	_tab_cal.pressed.connect(func(): _set_tab("calendar"))
	row.add_child(_tab_cal)
	_tab_lunch = _button("Obiady", false)
	_tab_lunch.pressed.connect(func(): _set_tab("lunch"))
	row.add_child(_tab_lunch)
	_tab_company = _button("Firma", false)
	_tab_company.pressed.connect(func(): _set_tab("company"))
	row.add_child(_tab_company)
	_style_label(_as_owner, 14, Color("#ffcf6e"))
	_as_owner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(_as_owner)
	_lock_btn = _button("Zablokuj", false)
	_lock_btn.pressed.connect(func(): action.emit(Protocol.PC_LOCK, 0, 0, ""))
	row.add_child(_lock_btn)
	_take_btn = _button("Zabierz laptop", false)
	_take_btn.pressed.connect(func(): action.emit(Protocol.PC_TAKE, 0, 0, ""))
	row.add_child(_take_btn)
	var close := _button("Zamknij (Esc)", false)
	close.pressed.connect(func(): action.emit(Protocol.PC_CLOSE, 0, 0, ""))
	row.add_child(close)

	# Messenger.
	_chat_view.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_screen.add_child(_chat_view)
	_body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_body.add_theme_constant_override("separation", 0)
	_chat_view.add_child(_body)
	var side := PanelContainer.new()
	var sdb := StyleBoxFlat.new()
	sdb.bg_color = PixelUI.NAVY
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
	_entry.add_theme_stylebox_override("normal", PixelUI.box("input"))
	_entry.add_theme_stylebox_override("focus", PixelUI.box("input_focus"))
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
	_screen.add_child(cal_pad)
	var ch := Label.new()
	_style_label(ch, 20, Color("#1c2430"))
	ch.text = "Kalendarz zarządu — spotkania na dziś"
	_cal_view.add_child(ch)
	_style_label(_cal_mine, 15, Color("#3d5a86"))
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
	_screen.add_child(lunch_pad)
	var lh := Label.new()
	_style_label(lh, 20, Color("#1c2430"))
	lh.text = "Obiady do biura — dostawa na recepcję (piętro 1)"
	_lunch_view.add_child(lh)
	_style_label(_lunch_status, 15, Color("#3d5a86"))
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
	_screen.add_child(co_pad)

	# Lock screen.
	_lock_view.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_lock_view.alignment = BoxContainer.ALIGNMENT_CENTER
	_lock_view.add_theme_constant_override("separation", 14)
	_screen.add_child(_lock_view)
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
	_title.text = "Komunikator firmowy — konto: %s" % owner_name
	_as_owner.text = "" if mine else "Uwaga: piszesz jako %s!" % owner_name
	_chat_view.visible = not state.locked and tab == "chat"
	_screen.get_node("cal_pad").visible = not state.locked and tab == "calendar"
	_screen.get_node("lunch_pad").visible = not state.locked and tab == "lunch"
	_tab_lunch.visible = not state.locked
	_tab_lunch.modulate = Color(1, 1, 1, 1.0 if tab == "lunch" else 0.6)
	var founder := _company_fresh()
	if tab == "company" and not founder:
		tab = "chat"
	_screen.get_node("company_pad").visible = not state.locked and tab == "company"
	_tab_company.visible = not state.locked and founder
	_tab_company.modulate = Color(1, 1, 1, 1.0 if tab == "company" else 0.6)
	_tab_chat.visible = not state.locked
	_tab_cal.visible = not state.locked
	_tab_chat.modulate = Color(1, 1, 1, 1.0 if tab == "chat" else 0.6)
	_tab_cal.modulate = Color(1, 1, 1, 1.0 if tab == "calendar" else 0.6)
	_lock_view.visible = state.locked
	_lock_btn.visible = not state.locked
	if state.locked:
		_lock_owner.text = "%s — zablokowany" % owner_name
		_lock_hint.text = "Przyłóż palec do czytnika, żeby odblokować." if mine else "Tylko %s może go odblokować. Laptop możesz najwyżej zabrać." % owner_name
		_unlock_btn.visible = mine
		return
	if tab == "calendar":
		_render_calendar()
		return
	if tab == "lunch":
		_render_lunch()
		return
	if tab == "company":
		_render_company()
		return
	_render_sidebar()
	var c := _conv(current)
	_conv_title.text = c.get("title", "")
	_render_messages()
	_render_input()


func _set_tab(t: String) -> void:
	tab = t
	_cal_sig = ""
	_lunch_sig = ""
	_co_sig = ""
	_render()


# ----------------------------------------------------------------- company

func _company_fresh() -> bool:
	return not company_offers.is_empty() and Time.get_ticks_msec() - _company_msec < COMPANY_FRESH_MSEC


func on_company(p: Dictionary) -> void:
	var was := _company_fresh()
	if p.type == Protocol.T_COMPANY_OFFERS:
		company_offers = p
	else:
		company_people = p
	_company_msec = Time.get_ticks_msec()
	if not visible:
		return
	if not was:
		_render()  # show the tab
	elif tab == "company":
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
	e.add_theme_stylebox_override("normal", PixelUI.box("input"))
	e.add_theme_stylebox_override("focus", PixelUI.box("input_focus"))
	e.text_changed.connect(func(t: String): _co_drafts[key] = t)
	return e


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

	_co_view.add_child(_co_label("Ogłoszenia na portalu", 18, Color("#3d5a86")))
	for o in company_offers.offers:
		var id: int = o.id
		var places: int = o.places
		row = _co_row()
		var t := _co_label(o.title, 15)
		t.custom_minimum_size = Vector2(250, 0)
		row.add_child(t)
		var minus := _button("−", false)
		minus.add_theme_color_override("font_color", Color("#1c2430"))
		minus.disabled = places == 0
		minus.pressed.connect(func(): company_action.emit(Protocol.CO_SET_PLACES, id, places - 1, ""))
		row.add_child(minus)
		var n := _co_label("%d miejsc" % places if places != 1 else "1 miejsce", 15, Color("#16a085") if places else Color("#8a93a3"))
		n.custom_minimum_size = Vector2(80, 0)
		n.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		row.add_child(n)
		var plus := _button("+", false)
		plus.add_theme_color_override("font_color", Color("#1c2430"))
		plus.disabled = places >= Protocol.CO_MAX_PLACES
		plus.pressed.connect(func(): company_action.emit(Protocol.CO_SET_PLACES, id, places + 1, ""))
		row.add_child(plus)
		var key := "desc:%d" % id
		var desc := _co_edit(key, o.description, 200, 300)
		desc.placeholder_text = "Opis stanowiska"
		row.add_child(desc)
		var save := _button("Zapisz opis", false)
		save.add_theme_color_override("font_color", Color("#1c2430"))
		save.pressed.connect(func():
			_co_drafts.erase(key)
			company_action.emit(Protocol.CO_SET_DESCRIPTION, id, 0, desc.text.strip_edges()))
		row.add_child(save)

	_co_view.add_child(_co_label("Kandydaci po rozmowie", 18, Color("#3d5a86")))
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
		var rej := PixelUI.button("Odrzuć", false, true)
		rej.pressed.connect(func(): company_action.emit(Protocol.CO_REJECT, pid, 0, ""))
		row.add_child(rej)

	_co_view.add_child(_co_label("Zespół", 18, Color("#3d5a86")))
	var staff: Array = company_people.get("staff", [])
	if staff.is_empty():
		_co_view.add_child(_co_label("Na razie tylko Ty.", 14, Color("#8a93a3")))
	for s in staff:
		var pid: int = s.id
		row = _co_row()
		var l := _co_label("%s — %s, od dnia %d" % [s.nick, DEPARTMENTS.get(s.department, "?"), s.day], 15)
		l.custom_minimum_size = Vector2(430, 0)
		row.add_child(l)
		if pid == my_id:
			row.add_child(_co_label("(Ty)", 14, Color("#8a93a3")))
			continue
		var fire := PixelUI.button("Zwolnij", false, true)
		fire.pressed.connect(func(): company_action.emit(Protocol.CO_FIRE, pid, 0, ""))
		row.add_child(fire)


func on_lunch(p: Dictionary) -> void:
	lunch = p
	if visible and tab == "lunch":
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
	if visible and tab == "calendar":
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
				_style_label(st, 15, Color("#2e6bd9"))
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
		sb.bg_color = Color("#3d5a86") if c.conv == current else Color(0, 0, 0, 0)
		var hover := sb.duplicate()
		hover.bg_color = Color("#46618c") if c.conv == current else Color(1, 1, 1, 0.08)
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
	PixelUI.style_label(l, size, color)


func _button(text: String, primary: bool) -> Button:
	return PixelUI.button(text, primary)
