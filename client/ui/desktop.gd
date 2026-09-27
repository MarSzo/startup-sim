## The character's home computer (GDD 9a, step 2): desktop with a browser
## (job portal + application form), mail and an online interview window.
## The server decides everything; this UI shows what it last sent and
## re-sends our last action when the server's state shows it got lost.
extends Control

const Ink = preload("res://ui/ink_ui.gd")

const Protocol = preload("res://net/protocol.gd")
const PlayerView = preload("res://game/player_view.gd")

signal apply(offer_id: int, motivation: String)
signal answer(attempt: int, index: int, choice: int)
signal portal_action(action: int, arg: int)
## "Załóż firmę" (CompanyAction FOUND).
signal found_company(name: String)
## StartOS button: the game menu (settings, leave).
signal menu_requested

const DEPT_NAMES := {1: "IT / Produkt", 2: "Biznes", 3: "Zarząd"}
const RESEND_MSEC := 1500

var offers := {}          # id -> offer dict (merged from JobOffers parts)
var mails := {}           # id -> mail dict
var unread := {}          # mail id -> true
var hired := false
var job_title := ""
var department := 0
var nick := ""
## From the server's Clock: our company's name and whether it has a founder
## (if not, the portal offers to found it).
var company := ""
var founded := true
var _founding := false
var _found_name := ""
## Fired: back on the portal until the next job.
var fired := false
var profile := {}
## Dev: apply for this offer, answer at random, go to the office (0 = off).
var auto_offer := 0
var auto_delay := 0.0
## Dev (--found=<name>): found the company once the portal knows it can.
var auto_found := ""

var _root := Control.new()
var _windows := {}        # name -> window PanelContainer
var _body := {}           # name -> VBoxContainer (window content)
var _taskbar := HBoxContainer.new()
var _clock := Label.new()
var game_day := 0          # from the server's Clock (0 = not known yet)
var game_minute := 0
var _toast := Label.new()
var _mail_icon_badge := Label.new()
var _browser_view := "list"   # list / form
var _form_offer := 0
var _motivation := TextEdit.new()
var _pending_apply := {}      # offer -> [msec, motivation]
var _pending_action := {}     # "action:arg" -> msec
var _answered := {}           # "attempt:index" -> choice
var _question_key := ""
var _interview_offer := 0
var _result_attempt := -1
var _selected_mail := -1
var _auto_applied := false


func _ready() -> void:
	get_viewport().size_changed.connect(_fit)
	visibility_changed.connect(_fit)
	_fit()
	_build_desktop()


func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size


func set_profile(p_nick: String, p_profile: Dictionary) -> void:
	nick = p_nick
	profile = p_profile


func reset() -> void:
	offers.clear()
	mails.clear()
	unread.clear()
	hired = false
	_pending_apply.clear()
	_pending_action.clear()
	_answered.clear()
	_question_key = ""
	_result_attempt = -1
	_auto_applied = false
	_browser_view = "list"
	visible = true
	for w in _windows.keys():
		_close_window(w)
	_refresh_mail_badge()


# ----------------------------------------------------------------- desktop

func _build_desktop() -> void:
	var bg := TextureRect.new()
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.texture = wallpaper()
	bg.stretch_mode = TextureRect.STRETCH_SCALE
	bg.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	add_child(bg)
	_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_root)
	var logo := _label("StartOS", 64, Color(1, 1, 1, 0.08))
	logo.set_anchors_preset(Control.PRESET_CENTER)
	logo.position = Vector2(-130, -60)
	_root.add_child(logo)

	var icons := VBoxContainer.new()
	icons.position = Vector2(24, 24)
	icons.add_theme_constant_override("separation", 18)
	_root.add_child(icons)
	icons.add_child(_icon("Przeglądarka", "browser", func(): _open_window("browser")))
	var mail_icon := _icon("Poczta", "mail", func(): _open_window("mail"))
	Ink.style_label(_mail_icon_badge, 16, Color.WHITE)
	var badge_bg := StyleBoxFlat.new()
	badge_bg.bg_color = Ink.RED
	badge_bg.border_color = Ink.INK
	badge_bg.set_border_width_all(Ink.LINE)
	badge_bg.content_margin_left = 6
	badge_bg.content_margin_right = 6
	_mail_icon_badge.add_theme_stylebox_override("normal", badge_bg)
	_mail_icon_badge.position = Vector2(56, -4)
	_mail_icon_badge.visible = false
	mail_icon.add_child(_mail_icon_badge)
	icons.add_child(mail_icon)
	icons.add_child(_icon("Kosz", "trash", func(): _toast_msg("Kosz jest pusty. Na razie.")))

	# Taskbar.
	var bar := PanelContainer.new()
	bar.add_theme_stylebox_override("panel", Ink.box("hud"))
	bar.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	bar.offset_top = -52
	_root.add_child(bar)
	var row := HBoxContainer.new()
	bar.add_child(row)
	var start_btn := Ink.button("◆ StartOS")
	start_btn.tooltip_text = "Menu gry: ustawienia, wyjście"
	start_btn.pressed.connect(func(): menu_requested.emit())
	row.add_child(start_btn)
	_taskbar.add_theme_constant_override("separation", 6)
	_taskbar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(_taskbar)
	_clock.add_theme_font_size_override("font_size", 15)
	_clock.add_theme_color_override("font_color", Color(1, 1, 1, 0.85))
	row.add_child(_clock)

	_toast.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	_toast.position = Vector2(-380, 16)
	_toast.size = Vector2(360, 0)
	_toast.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	Ink.style_label(_toast, 16, Ink.TEXT)
	_toast.add_theme_stylebox_override("normal", Ink.box("hud"))
	_toast.visible = false
	_root.add_child(_toast)


## The desktop wallpaper: a soft dusk sky, a few stars and a dark city
## skyline with lit windows (drawn small, smoothly scaled up).
static func wallpaper() -> Texture2D:
	var w := 320
	var h := 180
	var img := Image.create(w, h, false, Image.FORMAT_RGBA8)
	var top := Color("#1f1a2e")
	var mid := Color("#4a3552")
	var low := Color("#b0706a")
	for y in h:
		var t := float(y) / h
		var c := top.lerp(mid, t / 0.6) if t < 0.6 else mid.lerp(low, (t - 0.6) / 0.4)
		for x in w:
			img.set_pixel(x, y, c)
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for k in 60:
		img.set_pixel(rng.randi_range(0, w - 1), rng.randi_range(0, h / 2), Color(1, 0.95, 0.85, rng.randf_range(0.3, 0.8)))
	var x := 0
	while x < w:
		var bw := rng.randi_range(14, 34)
		var bh := rng.randi_range(28, 80)
		var shade := Color("#17121c").lightened(rng.randf_range(0.0, 0.06))
		for yy in range(h - bh, h):
			for xx in range(x, mini(x + bw, w)):
				img.set_pixel(xx, yy, shade)
		for wy in range(h - bh + 5, h - 3, 7):
			for wx in range(x + 3, mini(x + bw - 3, w), 5):
				if rng.randf() < 0.3:
					for dy in 3:
						for dx in 2:
							img.set_pixel(mini(wx + dx, w - 1), mini(wy + dy, h - 1), Color("#e8b85a"))
		x += bw + rng.randi_range(0, 4)
	return ImageTexture.create_from_image(img)

func _icon(caption: String, kind: String, on_open: Callable) -> Control:
	var b := Button.new()
	b.flat = true
	b.custom_minimum_size = Vector2(84, 80)
	b.pressed.connect(on_open)
	var pic := Control.new()
	pic.size = Vector2(84, 52)
	pic.mouse_filter = Control.MOUSE_FILTER_IGNORE
	pic.draw.connect(func(): draw_icon(pic, kind))
	b.add_child(pic)
	var l := _label(caption, 16, Color.WHITE, false)
	l.position = Vector2(0, 54)
	l.size = Vector2(84, 22)
	l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	l.add_theme_constant_override("outline_size", 4)
	l.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.6))
	l.mouse_filter = Control.MOUSE_FILTER_IGNORE
	b.add_child(l)
	return b


## Desktop icons, hand-drawn like the rest of the game (ink outlines):
## browser, mail, trash, chat, calendar, company, tasks.
static func draw_icon(c: Control, kind: String) -> void:
	var ink := Ink.INK
	var o := Vector2(c.size.x / 2, 26)
	match kind:
		"browser":  # a globe
			c.draw_circle(o, 19, ink)
			c.draw_circle(o, 16.5, Color("#5b8fc2"))
			var land := PackedVector2Array([o + Vector2(-9, -8), o + Vector2(-2, -11), o + Vector2(3, -5), o + Vector2(-1, 1), o + Vector2(-8, 0)])
			c.draw_colored_polygon(land, Color("#7a9a4a"))
			var land2 := PackedVector2Array([o + Vector2(4, 3), o + Vector2(12, 1), o + Vector2(11, 9), o + Vector2(5, 11)])
			c.draw_colored_polygon(land2, Color("#7a9a4a"))
			c.draw_arc(o, 16.5, 0, TAU, 32, ink, 2.0, true)
			c.draw_line(o + Vector2(-16, 0), o + Vector2(16, 0), Color(ink, 0.5), 1.2, true)
			c.draw_arc(o + Vector2(0, 0), 16.5, PI * 1.5, PI * 2.5, 16, Color(ink, 0.35), 1.2, true)
			c.draw_arc(o + Vector2(-5, -5), 5, PI * 1.1, PI * 1.5, 6, Color(1, 1, 1, 0.5), 2.0, true)
		"mail":  # an envelope
			var r := Rect2(o + Vector2(-20, -13), Vector2(40, 27))
			c.draw_rect(r.grow(1.5), ink)
			c.draw_rect(r, Color("#efe4c8"))
			c.draw_polyline(PackedVector2Array([r.position, o + Vector2(0, 2), Vector2(r.end.x, r.position.y)]), ink, 2.0, true)
			c.draw_circle(o + Vector2(0, 2), 4, Color("#a8402f"))  # wax seal
			c.draw_arc(o + Vector2(0, 2), 4, 0, TAU, 12, ink, 1.2, true)
		"trash":  # a bin
			var body := PackedVector2Array([o + Vector2(-13, -8), o + Vector2(13, -8), o + Vector2(10, 18), o + Vector2(-10, 18)])
			c.draw_colored_polygon(body, Color("#9aa3a0"))
			var closed := body.duplicate()
			closed.append(body[0])
			c.draw_polyline(closed, ink, 2.0, true)
			for k in [-5.0, 0.0, 5.0]:
				c.draw_line(o + Vector2(k, -4), o + Vector2(k * 0.8, 15), Color(ink, 0.6), 1.5, true)
			var lid := Rect2(o + Vector2(-16, -14), Vector2(32, 5))
			c.draw_rect(lid.grow(1.2), ink)
			c.draw_rect(lid, Color("#b7bfbc"))
			c.draw_rect(Rect2(o + Vector2(-4, -18), Vector2(8, 4)), ink)
		"chat":  # two speech bubbles
			for b in [[Vector2(-7, -5), Color("#f2e7cb")], [Vector2(7, 5), Color("#9fd0c0")]]:
				var r := Rect2(o + b[0] - Vector2(13, 9), Vector2(26, 18))
				c.draw_rect(r.grow(1.5), ink)
				c.draw_rect(r, b[1])
				var tail := PackedVector2Array([r.position + Vector2(5, 18), r.position + Vector2(3, 24), r.position + Vector2(11, 18)])
				c.draw_colored_polygon(tail, ink)
				for k in 3:
					c.draw_circle(r.get_center() + Vector2(-6 + k * 6, 0), 1.6, ink)
		"calendar":  # a desk calendar with a red header
			var r := Rect2(o + Vector2(-17, -16), Vector2(34, 32))
			c.draw_rect(r.grow(1.5), ink)
			c.draw_rect(r, Color("#f4ead0"))
			c.draw_rect(Rect2(r.position, Vector2(34, 9)), Color("#a8402f"))
			for gy in 3:
				for gx in 4:
					c.draw_rect(Rect2(r.position + Vector2(3 + gx * 8, 12 + gy * 6), Vector2(5, 3)), Color(ink, 0.55))
			c.draw_rect(Rect2(r.position + Vector2(19, 18), Vector2(5, 3)), Color("#a8402f"))
		"company":  # an office building
			var r := Rect2(o + Vector2(-13, -19), Vector2(26, 38))
			c.draw_rect(r.grow(1.5), ink)
			c.draw_rect(r, Color("#c9b48a"))
			for wy in 5:
				for wx in 3:
					c.draw_rect(Rect2(r.position + Vector2(3 + wx * 8, 3 + wy * 7), Vector2(4, 4)), Color("#e8b85a") if (wx + wy) % 3 else Color(ink, 0.7))
			c.draw_rect(Rect2(o + Vector2(-3, 11), Vector2(6, 8)), ink)
		"tasks":  # a board with three columns of cards
			var r := Rect2(o + Vector2(-19, -15), Vector2(38, 30))
			c.draw_rect(r.grow(1.5), ink)
			c.draw_rect(r, Color("#f4ead0"))
			var cols := [Color("#e0a82e"), Color("#5b8fc2"), Color("#6f8f3e")]
			for k in 3:
				for n in 3 - k:
					c.draw_rect(Rect2(r.position + Vector2(3 + k * 12, 4 + n * 8), Vector2(9, 6)), cols[k])

func _process(_d: float) -> void:
	if not visible:
		return
	if game_day > 0:
		_clock.text = "%s   Dzień %d · %02d:%02d" % [nick, game_day, game_minute / 60, game_minute % 60]
	else:
		_clock.text = nick


# ----------------------------------------------------------------- windows

const TITLES := {"browser": "Przeglądarka — praca.example", "mail": "Poczta — %s", "interview": "Rozmowa online"}


func _open_window(name: String) -> void:
	if not _windows.has(name):
		var win := Ink.panel("paper")
		var col := VBoxContainer.new()
		col.add_theme_constant_override("separation", 0)
		win.add_child(col)
		var title_bar := PanelContainer.new()
		title_bar.add_theme_stylebox_override("panel", Ink.box("title"))
		var trow := HBoxContainer.new()
		title_bar.add_child(trow)
		var tl := _label(TITLES[name] % [profile.get("email", "")] if name == "mail" else TITLES[name], 16, Color.WHITE)
		tl.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		trow.add_child(tl)
		var close := Ink.button("X", false, true)
		close.pressed.connect(func(): _close_window(name))
		trow.add_child(close)
		title_bar.gui_input.connect(func(ev): _drag(win, ev))
		col.add_child(title_bar)
		var scroll := ScrollContainer.new()
		scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
		scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		col.add_child(scroll)
		var margin := MarginContainer.new()
		margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		for side in ["left", "right", "top", "bottom"]:
			margin.add_theme_constant_override("margin_" + side, 18)
		scroll.add_child(margin)
		var body := VBoxContainer.new()
		body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		body.add_theme_constant_override("separation", 12)
		margin.add_child(body)
		var vs := get_viewport_rect().size
		var wsize := Vector2(minf(820, vs.x - 180), vs.y - 120)
		win.size = wsize
		var offset: Vector2 = {"browser": Vector2(0, 0), "mail": Vector2(40, 24), "interview": Vector2(80, 12)}[name]
		win.position = Vector2(140, 20) + offset
		_root.add_child(win)
		_windows[name] = win
		_body[name] = body
		var tbtn := Ink.button({"browser": "Przeglądarka", "mail": "Poczta", "interview": "Rozmowa"}[name])
		tbtn.pressed.connect(func(): _focus(name))
		tbtn.name = "task_" + name
		_taskbar.add_child(tbtn)
	_focus(name)
	_render(name)


func _close_window(name: String) -> void:
	if _windows.has(name):
		_windows[name].queue_free()
		_windows.erase(name)
		_body.erase(name)
		var t := _taskbar.get_node_or_null("task_" + name)
		if t:
			_taskbar.remove_child(t)  # now: a reopened window reuses the name
			t.queue_free()


func _focus(name: String) -> void:
	if _windows.has(name):
		_root.move_child(_windows[name], -1)
		_root.move_child(_toast, -1)
		if name == "mail":
			pass


func _drag(win: Control, ev: InputEvent) -> void:
	if ev is InputEventMouseMotion and ev.button_mask & MOUSE_BUTTON_MASK_LEFT:
		win.position += ev.relative
	elif ev is InputEventMouseButton and ev.pressed:
		_root.move_child(win, -1)


func _render(name: String) -> void:
	if not _body.has(name):
		return
	var body: VBoxContainer = _body[name]
	for c in body.get_children():
		c.queue_free()
	match name:
		"browser": _render_browser(body)
		"mail": _render_mail(body)
		"interview": pass  # filled by the question / result handlers


# ----------------------------------------------------------------- browser

func _render_browser(body: VBoxContainer) -> void:
	var addr := _label("🔒  https://praca.example/oferty" if _browser_view == "list" else "🔒  https://praca.example/aplikuj", 13, Color("#4a5566"))
	body.add_child(addr)
	if _browser_view == "form" and offers.has(_form_offer):
		_render_form(body, offers[_form_offer])
		return
	body.add_child(_label("Praca od zaraz — najnowsze ogłoszenia", 26))
	body.add_child(_label("Znajdź pracę marzeń (albo chociaż taką z owocowymi czwartkami).", 15, Color("#4a5566")))
	if not founded and not hired:
		_render_found_card(body)
	if offers.is_empty():
		body.add_child(_label("Ładowanie ofert…", 18))
		return
	var ids := offers.keys()
	ids.sort()
	for id in ids:
		var o: Dictionary = offers[id]
		# Our startup: hidden while the position is filled (unless you applied).
		var ours: bool = o.department != 0  # this building's company
		if ours and o.get("vacancies", 0) == 0 and not o.applied and not _pending_apply.has(id):
			continue
		var box := _card(body)
		var head := HBoxContainer.new()
		var t := _label(o.title, 21)
		t.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		head.add_child(t)
		box.add_child(head)
		var company: String = o.company + ("  ·  dział " + DEPT_NAMES[o.department] if DEPT_NAMES.has(o.department) else "")
		box.add_child(_label(company, 14, Color("#2e6bd9")))
		box.add_child(_label(o.description, 15, Color("#4a5566")))
		if ours:
			var free: int = o.get("vacancies", 0)
			var places := "Stanowisko obsadzone" if free == 0 else ("Wolne miejsca: %d" % free)
			box.add_child(_label(places, 14, Color("#8f5a1a") if free > 0 else Color("#c0392b")))
		if o.applied or _pending_apply.has(id):
			box.add_child(_label("✓ Zgłoszenie wysłane — odpowiedź przyjdzie e-mailem.", 14, Color("#1e8449")))
		else:
			var b := _button("Aplikuj")
			var oid: int = id
			b.pressed.connect(func(): _browser_view = "form"; _form_offer = oid; _render("browser"))
			box.add_child(b)


func _render_form(body: VBoxContainer, o: Dictionary) -> void:
	body.add_child(_label("Formularz zgłoszeniowy", 26))
	body.add_child(_label("%s — %s" % [o.title, o.company], 16, Color("#2e6bd9")))
	var box := _card(body)
	var g := ["Kobieta", "Mężczyzna", "Inna"]
	for row in [["Imię", nick], ["E-mail", profile.get("email", "")], ["Miejscowość", profile.get("city", "")],
			["Wiek", str(profile.get("age", ""))], ["Płeć", g[clampi(profile.get("gender", 0), 0, 2)]]]:
		var h := HBoxContainer.new()
		var k := _label(row[0] + ":", 15, Color("#4a5566"))
		k.custom_minimum_size = Vector2(130, 0)
		h.add_child(k)
		var v := _label(str(row[1]), 15)
		v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		h.add_child(v)
		box.add_child(h)
	box.add_child(_label("Dlaczego chcesz u nas pracować? (opcjonalnie)", 15, Color("#4a5566")))
	if _motivation.get_parent():
		_motivation.get_parent().remove_child(_motivation)
	_motivation.custom_minimum_size = Vector2(0, 90)
	_motivation.placeholder_text = "Np. bo lubię wyzwania i kawę z ekspresu."
	_motivation.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	box.add_child(_motivation)
	var consent := CheckBox.new()
	consent.text = "Wyrażam zgodę na przetwarzanie moich danych i mojej osoby w procesie rekrutacji."
	consent.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	consent.custom_minimum_size = Vector2(300, 0)
	for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color", "font_hover_pressed_color"]:
		consent.add_theme_color_override(k, Color("#1c2430"))
	box.add_child(consent)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	var back := _button("Wróć", false)
	back.pressed.connect(func(): _browser_view = "list"; _render("browser"))
	row.add_child(back)
	var send := _button("Wyślij zgłoszenie")
	var oid: int = o.id
	send.pressed.connect(func():
		if not consent.button_pressed:
			_toast_msg("Zaznacz zgodę na przetwarzanie danych — bez tego HR nie przeczyta nawet imienia.")
			return
		_send_apply(oid, _motivation.text.strip_edges())
		_motivation.text = ""
		_browser_view = "list"
		_render("browser")
		_toast_msg("Zgłoszenie wysłane! Odpowiedź przyjdzie na %s." % profile.get("email", "")))
	row.add_child(send)
	box.add_child(row)


func _send_apply(offer: int, motivation: String) -> void:
	_pending_apply[offer] = [Time.get_ticks_msec(), motivation]
	apply.emit(offer, motivation)


# -------------------------------------------------------------------- mail

func _render_mail(body: VBoxContainer) -> void:
	var ids := mails.keys()
	ids.sort()
	ids.reverse()
	if ids.is_empty():
		body.add_child(_label("Skrzynka odbiorcza jest pusta. Wyślij kilka zgłoszeń!", 17, Color("#4a5566")))
		return
	if _selected_mail < 0 or not mails.has(_selected_mail):
		_selected_mail = ids[0]
	var h := HBoxContainer.new()
	h.add_theme_constant_override("separation", 14)
	body.add_child(h)
	var list := VBoxContainer.new()
	list.custom_minimum_size = Vector2(250, 0)
	h.add_child(list)
	for id in ids:
		var m: Dictionary = mails[id]
		var b := Button.new()
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.text = ("● " if unread.has(id) else "   ") + m.from + "\n   " + m.subject
		b.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		b.custom_minimum_size = Vector2(250, 0)
		b.toggle_mode = true
		b.button_pressed = id == _selected_mail
		var mid: int = id
		b.pressed.connect(func(): _selected_mail = mid; _render("mail"))
		list.add_child(b)
	var m: Dictionary = mails[_selected_mail]
	unread.erase(_selected_mail)
	_refresh_mail_badge()
	var view := _card_in(h)
	view.get_parent().size_flags_horizontal = Control.SIZE_EXPAND_FILL
	view.add_child(_label(m.subject, 20))
	view.add_child(_label("Od: " + m.from, 14, Color("#4a5566")))
	view.add_child(HSeparator.new())
	view.add_child(_label(m.body, 16))
	match m.action:
		Protocol.PORTAL_JOIN_INTERVIEW:
			var b := _button("Dołącz do rozmowy")
			var arg: int = m.arg
			b.pressed.connect(func(): _join(arg))
			view.add_child(b)
		Protocol.PORTAL_GO_TO_OFFICE:
			var b := _button("Idę do biura")
			b.pressed.connect(_go_to_office)
			view.add_child(b)


func _refresh_mail_badge() -> void:
	_mail_icon_badge.text = str(unread.size())
	_mail_icon_badge.visible = unread.size() > 0


func _join(offer: int) -> void:
	_interview_offer = offer
	_pending_action["%d:%d" % [Protocol.PORTAL_JOIN_INTERVIEW, offer]] = Time.get_ticks_msec()
	portal_action.emit(Protocol.PORTAL_JOIN_INTERVIEW, offer)
	_open_window("interview")
	var body: VBoxContainer = _body["interview"]
	body.add_child(_label("Łączenie z rozmową…", 20))


func _go_to_office() -> void:
	_pending_action["%d:0" % Protocol.PORTAL_GO_TO_OFFICE] = Time.get_ticks_msec()
	portal_action.emit(Protocol.PORTAL_GO_TO_OFFICE, 0)
	_toast_msg("Wychodzisz z domu… Do zobaczenia w biurze!")


# --------------------------------------------------------------- interview

func _video_tile(parent: Container, who: String, look: int, appearance: Dictionary, seed_id: int) -> void:
	var tile := PanelContainer.new()
	var s := StyleBoxFlat.new()
	s.bg_color = Color("#20242e")
	s.border_color = Ink.INK
	s.set_border_width_all(Ink.LINE)
	tile.add_theme_stylebox_override("panel", s)
	tile.custom_minimum_size = Vector2(230, 170)
	var stage := Control.new()
	stage.clip_contents = true
	tile.add_child(stage)
	var holder := Node2D.new()
	holder.position = Vector2(115, 250)
	holder.scale = Vector2(9, 9)
	stage.add_child(holder)
	var v := PlayerView.new()
	v.look = look
	holder.add_child(v)
	v.setup(seed_id, "", 9)
	if not appearance.is_empty():
		v.set_appearance(appearance)
	var name_l := _label(who, 16, Color.WHITE, false)
	var nb := StyleBoxFlat.new()
	nb.bg_color = Color(0, 0, 0, 0.6)
	nb.content_margin_left = 6
	nb.content_margin_right = 6
	name_l.add_theme_stylebox_override("normal", nb)
	name_l.position = Vector2(8, 144)
	stage.add_child(name_l)
	parent.add_child(tile)


func _show_question(p: Dictionary, key: String) -> void:
	_open_window("interview")
	_question_key = key
	var body: VBoxContainer = _body["interview"]
	for c in body.get_children():
		c.queue_free()
	body.add_child(_label("Rozmowa online: %s" % offers.get(_interview_offer, {}).get("title", "rekrutacja"), 20))
	var tiles := HBoxContainer.new()
	tiles.add_theme_constant_override("separation", 12)
	_video_tile(tiles, "Kasia, HR — %s" % (company if company != "" else "Startup Sim"), PlayerView.LOOK_OFFICE, {}, 7)
	_video_tile(tiles, nick + " (Ty)", PlayerView.LOOK_PLAYER, profile.get("appearance", {}), 1)
	body.add_child(tiles)
	body.add_child(_label("Pytanie %d z %d" % [p.index + 1, p.total], 15, Color("#2e6bd9")))
	body.add_child(_label(p.text, 22))
	for i: int in p.options.size():
		var b := _button(p.options[i], false)
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		var choice: int = i
		b.pressed.connect(func(): _send_answer(p.attempt, p.index, choice))
		body.add_child(b)
	if auto_offer > 0:
		var n: int = p.options.size()
		_auto(func(): _send_answer(p.attempt, p.index, randi() % n))


func _send_answer(attempt: int, index: int, choice: int) -> void:
	var key := "%d:%d" % [attempt, index]
	if _answered.has(key):
		return
	_answered[key] = choice
	answer.emit(attempt, index, choice)
	if _body.has("interview"):
		var body: VBoxContainer = _body["interview"]
		for c in body.get_children():
			c.queue_free()
		body.add_child(_label("Rekruterka notuje coś w zeszycie…", 20))


func _show_result(p: Dictionary) -> void:
	_open_window("interview")
	var body: VBoxContainer = _body["interview"]
	for c in body.get_children():
		c.queue_free()
	if p.passed:
		hired = true
		job_title = offers.get(_interview_offer, {}).get("title", "")
		department = p.department
		body.add_child(_label("Świetna rozmowa! (%d/%d)" % [p.score, p.total], 26, Color("#1e8449")))
		body.add_child(_label("Kasia z HR uśmiecha się szeroko. Szczegóły przyjdą e-mailem.", 17))
	else:
		body.add_child(_label("Dziękujemy za rozmowę (%d/%d)" % [p.score, p.total], 26, Color("#c0392b")))
		body.add_child(_label("Kasia z HR: „Odezwiemy się.” — czyli raczej nie tym razem. Możesz aplikować ponownie.", 17))
	var close := _button("Zamknij")
	close.pressed.connect(func(): _close_window("interview"); _open_window("mail"))
	body.add_child(close)
	if auto_offer > 0:
		_auto(func(): _close_window("interview"))


# ------------------------------------------------------------------ network

## Feed every packet from the net client; desktop ones are handled here.
func on_packet(p: Dictionary) -> void:
	var now := Time.get_ticks_msec()
	match p.type:
		Protocol.T_JOB_OFFERS:
			var changed := false
			for o in p.offers:
				if not offers.has(o.id) or offers[o.id].applied != o.applied:
					changed = true
				offers[o.id] = o
				if o.applied:
					_pending_apply.erase(o.id)
				elif _pending_apply.has(o.id) and now - _pending_apply[o.id][0] > RESEND_MSEC:
					_send_apply(o.id, _pending_apply[o.id][1])  # our Apply got lost
			_resend_actions(now)
			if changed and _browser_view == "list":
				_render("browser")
			if auto_offer > 0 and not _auto_applied and offers.has(auto_offer) and not offers[auto_offer].applied:
				_auto_applied = true
				_auto(func(): _send_apply(auto_offer, "Tryb automatyczny."))
		Protocol.T_MAIL:
			if not mails.has(p.id):
				mails[p.id] = p
				unread[p.id] = true
				_refresh_mail_badge()
				_toast_msg("Nowa wiadomość od: %s\n%s" % [p.from, p.subject])
				var audio = preload("res://audio/audio.gd").inst
				if audio:
					audio.play("notify", -4.0)
				if _windows.has("mail"):
					_render("mail")
				if auto_offer > 0:
					_open_window("mail")
					if p.action == Protocol.PORTAL_JOIN_INTERVIEW:
						_auto(func(): _join(p.arg))
					elif p.action == Protocol.PORTAL_GO_TO_OFFICE:
						_auto(_go_to_office)
					elif p.subject.begins_with("Dziękujemy za rozmowę"):
						_auto_applied = false  # try again
		Protocol.T_QUESTION:
			_pending_action.erase("%d:%d" % [Protocol.PORTAL_JOIN_INTERVIEW, _interview_offer])
			var key := "%d:%d" % [p.attempt, p.index]
			if _answered.has(key):
				answer.emit(p.attempt, p.index, _answered[key])  # our Answer got lost
			elif key != _question_key:
				_show_question(p, key)
		Protocol.T_RECRUIT_RESULT:
			if p.attempt != _result_attempt:
				_result_attempt = p.attempt
				_show_result(p)


func _resend_actions(now: int) -> void:
	for key in _pending_action.keys():
		if now - _pending_action[key] > RESEND_MSEC:
			var parts: PackedStringArray = key.split(":")
			_pending_action[key] = now
			portal_action.emit(int(parts[0]), int(parts[1]))


## Nobody has founded the company on this server yet: found it yourself.
func _render_found_card(body: VBoxContainer) -> void:
	var box := _card(body)
	box.add_child(_label("Załóż własną firmę", 21, Color("#8e44ad")))
	box.add_child(_label("Biuro w tym budynku czeka na założyciela. Nadaj firmie nazwę — od razu trafisz do zarządu, dostaniesz kartę, laptop i panel do zatrudniania ludzi.", 15, Color("#4a5566")))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	box.add_child(row)
	var edit := LineEdit.new()
	edit.placeholder_text = "Nazwa firmy, np. Pixel Pierogi sp. z o.o."
	edit.text = _found_name
	edit.max_length = 40
	edit.custom_minimum_size = Vector2(380, 36)
	edit.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	edit.text_changed.connect(func(t: String): _found_name = t)
	row.add_child(edit)
	var b := _button("Załóż firmę")
	b.pressed.connect(func(): found(edit.text))
	row.add_child(b)


func found(name: String) -> void:
	name = name.strip_edges()
	if name.length() < 3:
		_toast_msg("Nazwa firmy musi mieć co najmniej 3 znaki.")
		return
	_founding = true
	found_company.emit(name)


## Clock: company name / founder; being fired brings the portal back.
func on_clock(p: Dictionary) -> void:
	var redraw: bool = p.company != company or p.founded != founded
	company = p.company
	founded = p.founded
	if hired and p.place == Protocol.PLACE_PORTAL:
		hired = false
		fired = true
		job_title = ""
		department = 0
		# Start the job hunt afresh: the old windows (interview) go and a new
		# inbox and offers follow this Clock.
		reset()
		redraw = true
	elif fired and not hired and p.place != Protocol.PLACE_PORTAL:
		on_entered_world()  # hired again: off to the office
	if auto_found != "" and not founded and not hired:
		var n := auto_found
		auto_found = ""
		_auto(func(): found(n))
	if redraw and visible:
		_render("browser")


## We are in the world: the desktop goes away.
func on_entered_world() -> void:
	if _founding:
		_founding = false
		job_title = "Założyciel/ka"
		department = 3
	hired = true
	_pending_action.clear()
	visible = false


func _auto(action: Callable) -> void:
	get_tree().create_timer(maxf(auto_delay, 0.05)).timeout.connect(action)


# ------------------------------------------------------------------ widgets

func _toast_msg(text: String) -> void:
	_toast.text = text
	_toast.visible = true
	_root.move_child(_toast, -1)
	var my := text
	get_tree().create_timer(4.0).timeout.connect(func(): if _toast.text == my: _toast.visible = false)


func _label(text: String, size: int, color := Ink.TEXT_INK, wrap := true) -> Label:
	return Ink.label(text, size, color, wrap)


func _card(parent: Container) -> VBoxContainer:
	return _card_in(parent)


func _card_in(parent: Container) -> VBoxContainer:
	var panel := Ink.panel("card")
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)
	parent.add_child(panel)
	return box


func _button(text: String, primary := true) -> Button:
	var b := Ink.button(text, primary)
	b.custom_minimum_size = Vector2(0, 36)
	return b
