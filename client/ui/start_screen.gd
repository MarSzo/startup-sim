## Start screen: nick + server address + "Połącz".
extends Control

signal connect_pressed(nick: String, address: String)

var nick_edit := LineEdit.new()
var addr_edit := LineEdit.new()
var button := Button.new()
var status := Label.new()


func _ready() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	var bg := ColorRect.new()
	bg.color = Color("#23232e")
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(360, 0)
	box.add_theme_constant_override("separation", 10)
	center.add_child(box)

	var title := Label.new()
	title.text = "Startup Sim"
	title.add_theme_font_size_override("font_size", 40)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(title)
	var sub := Label.new()
	sub.text = "symulator pracy w startupie IT"
	sub.modulate = Color(1, 1, 1, 0.6)
	sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(sub)
	box.add_child(HSeparator.new())

	box.add_child(_caption("Nick"))
	nick_edit.max_length = 16
	nick_edit.placeholder_text = "Twój nick"
	box.add_child(nick_edit)
	box.add_child(_caption("Adres serwera"))
	addr_edit.placeholder_text = "127.0.0.1:7777"
	box.add_child(addr_edit)
	button.text = "Połącz"
	button.custom_minimum_size = Vector2(0, 40)
	box.add_child(button)
	status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(status)
	var hint := Label.new()
	hint.text = "Sterowanie: WASD / strzałki   ·   F3: debug"
	hint.modulate = Color(1, 1, 1, 0.45)
	hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(hint)

	button.pressed.connect(_submit)
	nick_edit.text_submitted.connect(func(_t): _submit())
	addr_edit.text_submitted.connect(func(_t): _submit())
	nick_edit.grab_focus()


func _caption(text: String) -> Label:
	var l := Label.new()
	l.text = text
	l.modulate = Color(1, 1, 1, 0.75)
	return l


func set_defaults(nick: String, address: String) -> void:
	nick_edit.text = nick
	addr_edit.text = address


func set_status(text: String, is_error := false) -> void:
	status.text = text
	status.modulate = Color(1, 0.45, 0.4) if is_error else Color(1, 1, 1, 0.8)


func set_busy(busy: bool) -> void:
	button.disabled = busy
	nick_edit.editable = not busy
	addr_edit.editable = not busy


func _submit() -> void:
	if button.disabled:
		return
	var nick := nick_edit.text.strip_edges()
	if nick.is_empty():
		set_status("Podaj nick", true)
		return
	var addr := addr_edit.text.strip_edges()
	if addr.is_empty():
		addr = "127.0.0.1:7777"
	connect_pressed.emit(nick, addr)
