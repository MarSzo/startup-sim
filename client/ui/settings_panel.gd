## The settings (title screen and the Esc menu): full screen, the ink
## effect over the world, the camera zoom. Changes apply and save at once.
extends VBoxContainer

const Ink = preload("res://ui/ink_ui.gd")
const Settings = preload("res://ui/settings.gd")

signal changed
signal back

var _full := CheckButton.new()
var _mood := CheckButton.new()
var _zoom := HSlider.new()
var _zoom_label := Label.new()


func _ready() -> void:
	Settings.load_once()
	add_theme_constant_override("separation", 10)
	add_child(Ink.label("Ustawienia", 28, Ink.TEXT_INK))
	for pair in [[_full, "Pełny ekran"], [_mood, "Efekt „tuszu i papieru” na świecie"]]:
		var cb: CheckButton = pair[0]
		cb.text = pair[1]
		cb.add_theme_font_override("font", Ink.font())
		cb.add_theme_font_size_override("font_size", 20)
		for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color", "font_hover_pressed_color"]:
			cb.add_theme_color_override(k, Ink.TEXT_INK)
		add_child(cb)
	_full.button_pressed = Settings.fullscreen
	_mood.button_pressed = Settings.mood
	_full.toggled.connect(func(on: bool):
		Settings.fullscreen = on
		Settings.apply_window()
		_save())
	_mood.toggled.connect(func(on: bool):
		Settings.mood = on
		_save())
	var zr := HBoxContainer.new()
	zr.add_theme_constant_override("separation", 12)
	Ink.style_label(_zoom_label, 18, Ink.TEXT_INK)
	_zoom_label.custom_minimum_size = Vector2(210, 0)
	zr.add_child(_zoom_label)
	_zoom.min_value = 0.6
	_zoom.max_value = 2.0
	_zoom.step = 0.1
	_zoom.value = Settings.zoom
	_zoom.custom_minimum_size = Vector2(220, 24)
	_zoom.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_zoom.value_changed.connect(func(v: float):
		Settings.zoom = v
		_update_zoom_label()
		_save())
	zr.add_child(_zoom)
	add_child(zr)
	_update_zoom_label()
	add_child(Ink.label("W grze: kółko myszy albo + / - zmienia przybliżenie na chwilę.", 16, Ink.TEXT_MUTED))
	var b := Ink.button("Wróć")
	b.pressed.connect(func(): back.emit())
	add_child(b)


func _update_zoom_label() -> void:
	_zoom_label.text = "Przybliżenie kamery: %.1f×" % Settings.zoom


func _save() -> void:
	Settings.save()
	changed.emit()
