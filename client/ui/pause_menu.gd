## The Esc menu during the game (and the StartOS button on the home
## computer): back to the game, settings, leave to the title screen, quit.
## The game keeps running on the server meanwhile.
extends Control

const Ink = preload("res://ui/ink_ui.gd")
const SettingsPanel = preload("res://ui/settings_panel.gd")

signal to_menu
signal quit
## "Wyloguj": forget the remembered login and go back to the login screen.
signal logout
signal settings_changed

var _dim := ColorRect.new()
var _card := PanelContainer.new()
var _menu := VBoxContainer.new()
var _settings := SettingsPanel.new()


func _ready() -> void:
	visible = false
	mouse_filter = Control.MOUSE_FILTER_STOP
	get_viewport().size_changed.connect(_fit)
	_dim.color = Color(0.05, 0.03, 0.02, 0.55)
	add_child(_dim)
	_card.add_theme_stylebox_override("panel", Ink.box("paper"))
	add_child(_card)
	var box := VBoxContainer.new()
	_card.add_child(box)
	_menu.add_theme_constant_override("separation", 12)
	box.add_child(_menu)
	_menu.add_child(Ink.label("Przerwa", 30, Ink.TEXT_INK))
	_menu.add_child(Ink.label("Gra toczy się dalej — inni pracują.", 16, Ink.TEXT_MUTED))
	for entry in [["Wróć do gry", func(): close(), true], ["Ustawienia", func(): _show(_settings), false],
			["Wyjdź do menu", func(): close(); to_menu.emit(), false], ["Wyloguj", func(): close(); logout.emit(), false],
			["Wyjdź z gry", func(): quit.emit(), false]]:
		var b := Ink.button(entry[0], entry[2])
		b.custom_minimum_size = Vector2(300, 44)
		b.add_theme_font_size_override("font_size", 24)
		b.pressed.connect(entry[1])
		_menu.add_child(b)
	_settings.visible = false
	_settings.back.connect(func(): _show(_menu))
	_settings.changed.connect(func(): settings_changed.emit())
	box.add_child(_settings)
	_fit()


func _show(what: Control) -> void:
	_menu.visible = what == _menu
	_settings.visible = what == _settings
	_fit.call_deferred()


func open() -> void:
	_show(_menu)
	visible = true


func close() -> void:
	visible = false


func _fit() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_dim.size = vs
	_card.reset_size()
	_card.position = (vs - _card.size) / 2


func _input(event: InputEvent) -> void:
	if visible and event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_ESCAPE:
		if _settings.visible:
			_show(_menu)
		else:
			close()
		get_viewport().set_input_as_handled()
