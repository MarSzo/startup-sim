## Logging in before the game: the server (picked from a list, by name —
## the address stays hidden), nick (= the account and the
## character's name), password; "Załóż konto", "Zmień hasło", "Zapamiętaj
## mnie". With a remembered login: "Graj jako X" / "Wyloguj".
extends Control

const Ink = preload("res://ui/ink_ui.gd")
const AuthClient = preload("res://net/auth_client.gd")

## Logged in: `grant` = {nick, ticket, refresh, character}.
signal logged_in(address: String, grant: Dictionary, remember: bool)
signal back

var auth                         # AuthClient node (from main)
var server_opt := OptionButton.new()
var _servers: Array = []     # [{name, address}] in the list
var nick_edit := LineEdit.new()
var pass_edit := LineEdit.new()
var new_pass_edit := LineEdit.new()
var remember_box := CheckBox.new()
var status := Label.new()
var _form := VBoxContainer.new()
var _quick := VBoxContainer.new()
var _quick_label := Label.new()
var _new_pass_row: Control
var _login_btn: Button
var _register_btn: Button
var _change_btn: Button
var _trust_btn: Button
var _changing := false
var _busy := false


func _ready() -> void:
	get_viewport().size_changed.connect(_fit)
	visibility_changed.connect(_on_shown)
	_fit()
	var bg := ColorRect.new()
	bg.color = Color("#1e2230")
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var outer := VBoxContainer.new()
	outer.add_theme_constant_override("separation", 14)
	outer.custom_minimum_size = Vector2(420, 0)
	center.add_child(outer)
	var title := _label("Startup Sim", 44, Color.WHITE)
	title.add_theme_constant_override("outline_size", 8)
	title.add_theme_color_override("font_outline_color", Ink.ACCENT_LO)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	outer.add_child(title)
	var sub := _label("Zaloguj się albo załóż konto — Twój postęp zapisuje się na serwerze.", 16, Color(1, 1, 1, 0.6))
	sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	sub.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	outer.add_child(sub)
	var panel := PanelContainer.new()
	panel.add_theme_stylebox_override("panel", Ink.box("hud"))
	outer.add_child(panel)
	var inner := VBoxContainer.new()
	inner.add_theme_constant_override("separation", 8)
	panel.add_child(inner)

	# Remembered login.
	_quick.add_theme_constant_override("separation", 8)
	_quick_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	Ink.style_label(_quick_label, 18, Color.WHITE)
	_quick.add_child(_quick_label)
	var play := _big_button("Graj", true)
	play.pressed.connect(_quick_play)
	_quick.add_child(play)
	var other := Ink.button("Zaloguj na inne konto")
	other.pressed.connect(func(): _show_form())
	_quick.add_child(other)
	var out := Ink.button("Wyloguj")
	out.pressed.connect(_logout)
	_quick.add_child(out)
	inner.add_child(_quick)

	# The form.
	_form.add_theme_constant_override("separation", 6)
	inner.add_child(_form)
	_fill_servers()
	_field("Serwer", server_opt)
	nick_edit.max_length = 16
	nick_edit.placeholder_text = "np. Ola"
	_field("Nick (imię postaci)", nick_edit)
	pass_edit.secret = true
	pass_edit.max_length = 128
	_field("Hasło", pass_edit)
	new_pass_edit.secret = true
	new_pass_edit.max_length = 128
	new_pass_edit.placeholder_text = "co najmniej 8 znaków"
	_new_pass_row = _field("Nowe hasło", new_pass_edit)
	remember_box.text = "Zapamiętaj mnie na tym komputerze"
	remember_box.button_pressed = true
	for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color", "font_hover_pressed_color"]:
		remember_box.add_theme_color_override(k, Color(1, 1, 1, 0.85))
	_form.add_child(remember_box)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	_login_btn = _big_button("Zaloguj", true)
	_login_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_login_btn.pressed.connect(func(): _go("login"))
	row.add_child(_login_btn)
	_register_btn = _big_button("Załóż konto", false)
	_register_btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_register_btn.pressed.connect(func(): _go("register"))
	row.add_child(_register_btn)
	_form.add_child(row)
	_change_btn = Ink.button("Zmień hasło")
	_change_btn.pressed.connect(_toggle_change)
	_form.add_child(_change_btn)
	for e in [nick_edit, pass_edit, new_pass_edit]:
		e.text_submitted.connect(func(_t): _go("password" if _changing else "login"))

	_trust_btn = Ink.button("Zaufaj nowemu certyfikatowi serwera", false, true)
	_trust_btn.visible = false
	_trust_btn.pressed.connect(func():
		AuthClient.forget_pin(address())
		_trust_btn.visible = false
		set_status("Zapomniany. Spróbuj jeszcze raz."))
	outer.add_child(_trust_btn)
	var b := Ink.button("Wróć do menu")
	b.pressed.connect(func(): back.emit())
	outer.add_child(b)
	status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status.custom_minimum_size = Vector2(0, 24)
	outer.add_child(status)
	_new_pass_row.visible = false
	_load_defaults()


func _fill_servers() -> void:
	server_opt.clear()
	_servers = AuthClient.servers()
	for s in _servers:
		server_opt.add_item(s.name)



## The chosen server's address.
func address() -> String:
	var i := server_opt.selected
	return _servers[i].address if i >= 0 and i < _servers.size() else AuthClient.default_server()


## Choose a server by address (one not on the list — a dev --server — is added).
func set_address(a: String) -> void:
	for i in _servers.size():
		if _servers[i].address == a:
			server_opt.select(i)
			return
	if a == "" or not OS.has_feature("editor"):
		server_opt.select(0)
		return
	_servers.append({"name": AuthClient.server_name(a), "address": a})
	server_opt.add_item(AuthClient.server_name(a))
	server_opt.select(_servers.size() - 1)


func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size


func _label(text: String, size: int, color := Color(1, 1, 1, 0.8)) -> Label:
	var l := Label.new()
	l.text = text
	Ink.style_label(l, size, color)
	return l


func _field(caption: String, control: Control) -> Control:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 2)
	box.add_child(_label(caption, 14, Color(1, 1, 1, 0.6)))
	control.custom_minimum_size.x = 380
	box.add_child(control)
	_form.add_child(box)
	return box


func _big_button(text: String, primary: bool) -> Button:
	var b := Ink.button(text, primary)
	b.custom_minimum_size = Vector2(0, 44)
	b.add_theme_font_size_override("font_size", 22)
	return b


func _load_defaults() -> void:
	var r := AuthClient.remembered()
	set_address(r.get("address", AuthClient.default_server()))
	nick_edit.text = r.get("nick", "")


func _on_shown() -> void:
	if not visible:
		return
	_fit()
	var r := AuthClient.remembered()
	if r.is_empty():
		_show_form()
	else:
		_quick_label.text = "Zalogowano jako %s\n(%s)" % [r.nick, AuthClient.server_name(r.address)]
		_quick.visible = true
		_form.visible = false


func _show_form() -> void:
	_quick.visible = false
	_form.visible = true
	if nick_edit.text == "":
		nick_edit.grab_focus.call_deferred()
	else:
		pass_edit.grab_focus.call_deferred()


func set_status(text: String, is_error := false) -> void:
	status.text = text
	status.add_theme_color_override("font_color", Color("#ff8a80") if is_error else Color(1, 1, 1, 0.8))


func _set_busy(on: bool) -> void:
	_busy = on
	for b in [_login_btn, _register_btn, _change_btn]:
		b.disabled = on


func _toggle_change() -> void:
	_changing = not _changing
	_new_pass_row.visible = _changing
	_change_btn.text = "Anuluj zmianę hasła" if _changing else "Zmień hasło"
	_login_btn.text = "Zmień i zaloguj" if _changing else "Zaloguj"
	_register_btn.visible = not _changing


func _go(what: String) -> void:
	if _busy:
		return
	if _changing and what == "login":
		what = "password"
	var addr := address()
	var nick := nick_edit.text.strip_edges()
	var pw := pass_edit.text
	if addr == "" or nick == "" or pw == "":
		set_status("Wybierz serwer, wpisz nick i hasło.", true)
		return
	_set_busy(true)
	set_status("Łączenie z serwerem logowania…")
	var r: Dictionary
	match what:
		"register":
			r = await auth.register(addr, nick, pw)
		"password":
			r = await auth.change_password(addr, nick, pw, new_pass_edit.text)
		_:
			r = await auth.login(addr, nick, pw)
	_set_busy(false)
	if not r.get("ok", false):
		set_status(str(r.get("error", "Nie udało się.")), true)
		_trust_btn.visible = r.get("cert_changed", false)
		return
	pass_edit.text = ""
	new_pass_edit.text = ""
	if _changing:
		_toggle_change()
	set_status("")
	logged_in.emit(addr, r, remember_box.button_pressed)


## "Graj" with the remembered login: a fresh ticket.
func _quick_play() -> void:
	if _busy:
		return
	var r0 := AuthClient.remembered()
	_set_busy(true)
	set_status("Logowanie…")
	var r: Dictionary = await auth.refresh(r0.address, r0.refresh)
	_set_busy(false)
	if not r.get("ok", false):
		AuthClient.forget()
		set_address(r0.address)
		nick_edit.text = r0.nick
		_show_form()
		set_status(str(r.get("error", "Zaloguj się ponownie.")), true)
		_trust_btn.visible = r.get("cert_changed", false)
		return
	set_status("")
	logged_in.emit(r0.address, r, true)


func _logout() -> void:
	var r := AuthClient.remembered()
	if not r.is_empty():
		auth.logout(r.address, r.refresh)
	AuthClient.forget()
	_show_form()
	set_status("Wylogowano.")
