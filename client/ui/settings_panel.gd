## The settings (title screen and the Esc menu): full screen, battery
## saving, crash reports, the camera zoom, sound volumes, the microphone.
## Changes apply and save at once. The options scroll when the window is
## short; "Wróć" stays visible under them.
extends VBoxContainer

const Ink = preload("res://ui/ink_ui.gd")
const Settings = preload("res://ui/settings.gd")

signal changed
signal back

var _full := CheckButton.new()
var _battery := CheckButton.new()
var _crashes := CheckButton.new()
var _zoom := HSlider.new()
var _zoom_label := Label.new()
var _scroll := ScrollContainer.new()
var _content := VBoxContainer.new()


func _ready() -> void:
	Settings.load_once()
	add_theme_constant_override("separation", 10)
	add_child(Ink.label("Ustawienia", 28, Ink.TEXT_INK))
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(_scroll)
	_content.add_theme_constant_override("separation", 10)
	_content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.add_child(_content)
	for pair in [[_full, "Pełny ekran"], [_battery, "Oszczędzanie baterii (30 klatek/s)"],
			[_crashes, "Wysyłaj raporty awarii bez pytania"]]:
		var cb: CheckButton = pair[0]
		cb.text = pair[1]
		cb.add_theme_font_override("font", Ink.font())
		cb.add_theme_font_size_override("font_size", 20)
		for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color", "font_hover_pressed_color"]:
			cb.add_theme_color_override(k, Ink.TEXT_INK)
		_content.add_child(cb)
	_full.button_pressed = Settings.fullscreen
	_battery.button_pressed = Settings.battery
	_crashes.button_pressed = Settings.crash_reports_always
	_crashes.toggled.connect(func(on: bool):
		Settings.crash_reports_always = on
		_save())
	_battery.toggled.connect(func(on: bool):
		Settings.battery = on
		Settings.apply_fps()
		_save())
	_full.toggled.connect(func(on: bool):
		Settings.fullscreen = on
		Settings.apply_window()
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
	_content.add_child(zr)
	_update_zoom_label()
	_content.add_child(Ink.label("W grze: kółko myszy albo + / - zmienia przybliżenie na chwilę.", 16, Ink.TEXT_MUTED))
	_volume("Efekty", Settings.vol_sfx, func(v: float): Settings.vol_sfx = v)
	_volume("Otoczenie", Settings.vol_ambient, func(v: float): Settings.vol_ambient = v)
	_volume("Muzyka", Settings.vol_music, func(v: float): Settings.vol_music = v)
	_volume("Głosy graczy", Settings.vol_voice, func(v: float): Settings.vol_voice = v)
	var mr := HBoxContainer.new()
	mr.add_theme_constant_override("separation", 12)
	var ml := Ink.label("Mikrofon:", 18, Ink.TEXT_INK)
	ml.custom_minimum_size = Vector2(210, 0)
	mr.add_child(ml)
	var mic := OptionButton.new()
	mic.custom_minimum_size = Vector2(220, 0)
	mic.clip_text = true
	for d in AudioServer.get_input_device_list():
		mic.add_item("Domyślny systemu" if d == "Default" else d)
		mic.set_item_metadata(mic.item_count - 1, d)
		if d == Settings.mic_device:
			mic.select(mic.item_count - 1)
	mic.item_selected.connect(func(i: int):
		Settings.mic_device = mic.get_item_metadata(i)
		Settings.apply_audio()
		_save())
	mr.add_child(mic)
	_content.add_child(mr)
	_content.add_child(Ink.label("Czat głosowy: trzymaj V — mówisz do pomieszczenia, B — szept do osoby obok.", 16, Ink.TEXT_MUTED))
	var b := Ink.button("Wróć")
	b.pressed.connect(func(): back.emit())
	add_child(b)
	get_viewport().size_changed.connect(_fit_height)
	visibility_changed.connect(_fit_height)
	_fit_height.call_deferred()


## The options as tall as they are - or as fits the window (then they
## scroll); the title above and "Wróć" below always show.
func _fit_height() -> void:
	var want := _content.get_combined_minimum_size()
	var room := get_viewport_rect().size.y - FIXED_HEIGHT
	_scroll.custom_minimum_size = Vector2(want.x, clampf(want.y, 120.0, maxf(room, 120.0)))
	reset_size()


## Height around the options: the title screen's logo, the card's margins,
## "Ustawienia" and "Wróć".
const FIXED_HEIGHT := 400.0


## A volume slider row (0..100 %).
func _volume(title: String, value: float, set_value: Callable) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)
	var l := Ink.label("", 18, Ink.TEXT_INK)
	l.custom_minimum_size = Vector2(210, 0)
	row.add_child(l)
	var s := HSlider.new()
	s.min_value = 0.0
	s.max_value = 1.0
	s.step = 0.05
	s.value = value
	s.custom_minimum_size = Vector2(220, 24)
	s.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var show := func(v: float): l.text = "%s: %d%%" % [title, roundi(v * 100)]
	show.call(value)
	s.value_changed.connect(func(v: float):
		set_value.call(v)
		show.call(v)
		Settings.apply_audio()
		_save())
	row.add_child(s)
	_content.add_child(row)


func _update_zoom_label() -> void:
	_zoom_label.text = "Przybliżenie kamery: %.1f×" % Settings.zoom


func _save() -> void:
	Settings.save()
	changed.emit()
