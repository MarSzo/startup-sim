## The HR app ("Kadry"): the contract, its annexes, leave days and leave
## requests. The server sends HrInfo; requests go back as HrAction.
extends VBoxContainer

const Ink = preload("res://ui/ink_ui.gd")
const Protocol = preload("res://net/protocol.gd")
const Departments = preload("res://net/departments.gd")

signal action(action: int, arg: int)

const STATUS := {1: "zaakceptowany", 2: "odrzucony", 3: "anulowany", 4: "wykorzystany"}
const TEXT := Color("#1c2430")
const DIM := Color("#4a5566")

var info := {}
var tab := "contract"
var _tabs := HBoxContainer.new()
var _body := VBoxContainer.new()
var _sig := ""


func _init() -> void:
	add_theme_constant_override("separation", 10)
	_tabs.add_theme_constant_override("separation", 6)
	add_child(_tabs)
	for t in [["contract", "Umowa"], ["annexes", "Aneksy"], ["leave", "Urlop"]]:
		var b := Ink.button(t[1])
		b.add_theme_font_size_override("font_size", 15)
		var id: String = t[0]
		b.pressed.connect(func(): show_tab(id))
		_tabs.add_child(b)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(scroll)
	_body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_body.add_theme_constant_override("separation", 8)
	scroll.add_child(_body)
	_render()


func on_hr(p: Dictionary) -> void:
	info = p
	_render()


func show_tab(t: String) -> void:
	tab = t
	_sig = ""
	_render()


func _render() -> void:
	var sig := "%s|%s" % [tab, str(info)]
	if sig == _sig:
		return
	_sig = sig
	for c in _body.get_children():
		c.queue_free()
	if info.is_empty():
		_body.add_child(_label("Ładowanie teczki pracownika…", 17, DIM))
		return
	match tab:
		"contract": _render_contract()
		"annexes": _render_annexes()
		"leave": _render_leave()


func _render_contract() -> void:
	_body.add_child(_label("Umowa", 24, TEXT))
	var form: String = Protocol.EMPLOYMENT_NAMES.get(info.form, "Umowa o pracę")
	var rows := [["Stanowisko", info.title], ["Dział", Departments.name_of(info.department, "—")], ["Forma", form],
		["Wynagrodzenie", "%s zł brutto / mies." % _thousands(info.salary)], ["Stawka", "%s zł / h" % _zl(info.pay_rate)],
		["Od dnia", str(info.start_day)], ["Nagany", "%d / 3" % info.reprimands]]
	for r in rows:
		_body.add_child(_row(r[0], str(r[1])))
	_body.add_child(_label("Podpisano w HR. Zmiany wynagrodzenia — aneksem (spotkanie z prezesem w kalendarzu).", 14, DIM))


func _render_annexes() -> void:
	_body.add_child(_label("Aneksy do umowy", 24, TEXT))
	var annexes: Array = info.annexes
	if annexes.size() <= 1:
		_body.add_child(_label("Brak aneksów. Podwyżki załatwia się na spotkaniu z prezesem.", 15, DIM))
	for a in annexes:
		_body.add_child(_row("Dzień %d" % a.day, a.text))


func _render_leave() -> void:
	_body.add_child(_label("Urlop", 24, TEXT))
	var left := "Dni urlopu do wykorzystania: %d" % info.leave_days
	_body.add_child(_label(left, 18, TEXT))
	_body.add_child(_label("Kolejny dzień urlopu za %d przepracowane dni. Na umowie o pracę urlop jest płatny (8 h), na B2B i zleceniu — bezpłatny." \
		% maxi(5 - info.worked, 1), 14, DIM))
	_body.add_child(_label("Złóż wniosek (najbliższe 7 dni):", 16, TEXT))
	var days := HFlowContainer.new()
	days.add_theme_constant_override("h_separation", 6)
	days.add_theme_constant_override("v_separation", 6)
	var taken := {}
	for r in info.requests:
		if r.status == 1:
			taken[r.day] = true
	for d in range(info.today + 1, info.today + 8):
		var b := Ink.button("Dzień %d" % d, false)
		b.add_theme_font_size_override("font_size", 14)
		b.disabled = taken.has(d) or info.leave_days == 0
		var day: int = d
		b.pressed.connect(func(): action.emit(Protocol.HR_REQUEST, day))
		days.add_child(b)
	_body.add_child(days)
	_body.add_child(_label("Wnioski", 18, TEXT))
	if info.requests.is_empty():
		_body.add_child(_label("Brak wniosków.", 15, DIM))
	var reqs: Array = info.requests.duplicate()
	reqs.reverse()
	for r in reqs:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 10)
		var l := _label("Dzień %d — %s" % [r.day, STATUS.get(r.status, "?")], 15, TEXT)
		l.custom_minimum_size = Vector2(260, 0)
		row.add_child(l)
		if r.status == 1 and r.day > info.today:
			var c := Ink.button("Anuluj", false, true)
			c.add_theme_font_size_override("font_size", 14)
			var id: int = r.id
			c.pressed.connect(func(): action.emit(Protocol.HR_CANCEL, id))
			row.add_child(c)
		_body.add_child(row)


func _row(k: String, v: String) -> HBoxContainer:
	var h := HBoxContainer.new()
	var kl := _label(k + ":", 15, DIM)
	kl.custom_minimum_size = Vector2(150, 0)
	h.add_child(kl)
	var vl := _label(v, 15, TEXT)
	vl.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	vl.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	h.add_child(vl)
	return h


func _label(text: String, size: int, color: Color) -> Label:
	var l := Label.new()
	Ink.style_label(l, size, color)
	l.text = text
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return l


static func _zl(grosze: int) -> String:
	return "%d,%02d" % [grosze / 100, grosze % 100]


static func _thousands(v: int) -> String:
	var s := str(v)
	var out := ""
	for i in s.length():
		if i > 0 and (s.length() - i) % 3 == 0:
			out += " "
		out += s[i]
	return out
