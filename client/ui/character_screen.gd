## Character creation (GDD 9a, step 1): name, gender, age, city, e-mail and
## appearance with a live preview. The last character is remembered locally
## (user://character.cfg) so it doesn't have to be typed in every time.
extends Control

const PlayerView = preload("res://game/player_view.gd")

## Emitted with a validated character; `address` = server "host:port".
signal connect_pressed(nick: String, profile: Dictionary, address: String)

const SAVE_PATH := "user://character.cfg"
const GENDERS := ["Kobieta", "Mężczyzna", "Inna"]
const PREVIEW_SCALE := 7.0

var nick_edit := LineEdit.new()
var gender_opt := OptionButton.new()
var age_spin := SpinBox.new()
var city_edit := LineEdit.new()
var email_edit := LineEdit.new()
var addr_edit := LineEdit.new()
var button := Button.new()
var status := Label.new()

var appearance := {"skin": 0, "hair_style": 0, "hair_color": 0, "shirt": 1, "pants": 0}
var _preview := PlayerView.new()
var _swatches := {}      # key -> Array[Button]
var _hair_label := Label.new()


func _ready() -> void:
	get_viewport().size_changed.connect(_fit)
	visibility_changed.connect(_fit)
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
	center.add_child(outer)

	var title := _label("Startup Sim", 38, Color.WHITE)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	outer.add_child(title)
	var sub := _label("Stwórz swoją postać — za chwilę zaczniesz szukać pracy.", 16, Color(1, 1, 1, 0.6))
	sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	outer.add_child(sub)

	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 18)
	outer.add_child(row)
	row.add_child(_panel(_build_form()))
	row.add_child(_panel(_build_looks()))

	button.text = "Rozpocznij"
	button.custom_minimum_size = Vector2(0, 46)
	button.add_theme_font_size_override("font_size", 19)
	var bs := StyleBoxFlat.new()
	bs.bg_color = Color("#2e6bd9")
	bs.set_corner_radius_all(8)
	var bh := bs.duplicate()
	bh.bg_color = Color("#3b7bef")
	var bd := bs.duplicate()
	bd.bg_color = Color("#3a4257")
	button.add_theme_stylebox_override("normal", bs)
	button.add_theme_stylebox_override("hover", bh)
	button.add_theme_stylebox_override("pressed", bh)
	button.add_theme_stylebox_override("disabled", bd)
	button.add_theme_color_override("font_color", Color.WHITE)
	outer.add_child(button)
	status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status.custom_minimum_size = Vector2(0, 24)
	outer.add_child(status)

	button.pressed.connect(_submit)
	for e in [nick_edit, city_edit, email_edit, addr_edit]:
		e.text_submitted.connect(func(_t): _submit())
	_load()
	_refresh_looks()
	nick_edit.grab_focus()


# ------------------------------------------------------------------ layout

## Lives in a CanvasLayer: size explicitly to the viewport.
func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size


func _label(text: String, size: int, color := Color(1, 1, 1, 0.8)) -> Label:
	var l := Label.new()
	l.text = text
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", color)
	return l


func _panel(content: Control) -> PanelContainer:
	var p := PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color("#2a3042")
	sb.set_corner_radius_all(10)
	sb.set_content_margin_all(18)
	p.add_theme_stylebox_override("panel", sb)
	p.add_child(content)
	return p


func _field(box: VBoxContainer, caption: String, control: Control) -> void:
	box.add_child(_label(caption, 14, Color(1, 1, 1, 0.6)))
	control.custom_minimum_size.x = 300
	box.add_child(control)


func _build_form() -> VBoxContainer:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	box.add_child(_label("Dane postaci", 20, Color.WHITE))
	nick_edit.max_length = 16
	nick_edit.placeholder_text = "np. Ola"
	_field(box, "Imię", nick_edit)
	for g in GENDERS:
		gender_opt.add_item(g)
	_field(box, "Płeć", gender_opt)
	age_spin.min_value = 18
	age_spin.max_value = 70
	age_spin.value = 25
	_field(box, "Wiek", age_spin)
	city_edit.max_length = 40
	city_edit.placeholder_text = "np. Kraków"
	_field(box, "Miejscowość", city_edit)
	email_edit.max_length = 64
	email_edit.placeholder_text = "np. ola.nowak@poczta.pl"
	_field(box, "E-mail (postaci — do CV)", email_edit)
	box.add_child(HSeparator.new())
	addr_edit.placeholder_text = "127.0.0.1:7777"
	_field(box, "Adres serwera", addr_edit)
	return box


func _build_looks() -> HBoxContainer:
	var h := HBoxContainer.new()
	h.add_theme_constant_override("separation", 16)
	# Preview.
	var pv := VBoxContainer.new()
	var stage := Control.new()
	stage.custom_minimum_size = Vector2(180, 230)
	stage.clip_contents = true
	var holder := Node2D.new()
	holder.position = Vector2(90, 200)
	holder.scale = Vector2(PREVIEW_SCALE, PREVIEW_SCALE)
	stage.add_child(holder)
	holder.add_child(_preview)
	_preview.setup(0, "", PREVIEW_SCALE)
	pv.add_child(stage)
	var rot := Button.new()
	rot.text = "Obróć"
	rot.pressed.connect(func(): _preview.set_facing([2, 0, 3, 1][_preview.facing]))
	pv.add_child(rot)
	var rnd := Button.new()
	rnd.text = "Losuj wygląd"
	rnd.pressed.connect(_randomize)
	pv.add_child(rnd)
	h.add_child(pv)
	# Pickers.
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	box.add_child(_label("Wygląd", 20, Color.WHITE))
	_swatch_row(box, "Kolor skóry", "skin", PlayerView.SKINS)
	box.add_child(_label("Fryzura", 14, Color(1, 1, 1, 0.6)))
	var hr := HBoxContainer.new()
	var prev := Button.new()
	prev.text = "<"
	prev.pressed.connect(func(): _cycle_hair(-1))
	var next := Button.new()
	next.text = ">"
	next.pressed.connect(func(): _cycle_hair(1))
	_hair_label.custom_minimum_size = Vector2(120, 0)
	_hair_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	hr.add_child(prev)
	hr.add_child(_hair_label)
	hr.add_child(next)
	box.add_child(hr)
	_swatch_row(box, "Kolor włosów", "hair_color", PlayerView.HAIRS)
	_swatch_row(box, "Koszula", "shirt", PlayerView.SHIRTS)
	_swatch_row(box, "Spodnie", "pants", PlayerView.PANTS)
	h.add_child(box)
	return h


func _swatch_row(box: VBoxContainer, caption: String, key: String, colors: Array) -> void:
	box.add_child(_label(caption, 14, Color(1, 1, 1, 0.6)))
	var grid := GridContainer.new()
	grid.columns = 5
	grid.add_theme_constant_override("h_separation", 4)
	grid.add_theme_constant_override("v_separation", 4)
	var buttons: Array[Button] = []
	for i in colors.size():
		var b := Button.new()
		b.custom_minimum_size = Vector2(30, 24)
		b.tooltip_text = "%s %d" % [caption, i + 1]
		var idx := i
		b.pressed.connect(func(): appearance[key] = idx; _refresh_looks())
		grid.add_child(b)
		buttons.append(b)
	_swatches[key] = [buttons, colors]
	box.add_child(grid)


func _refresh_looks() -> void:
	for key in _swatches:
		var buttons: Array = _swatches[key][0]
		var colors: Array = _swatches[key][1]
		for i in buttons.size():
			var sb := StyleBoxFlat.new()
			sb.bg_color = colors[i]
			sb.set_corner_radius_all(4)
			var selected: bool = appearance[key] == i
			sb.set_border_width_all(3 if selected else 1)
			sb.border_color = Color.WHITE if selected else Color(0, 0, 0, 0.5)
			for state in ["normal", "hover", "pressed", "focus"]:
				buttons[i].add_theme_stylebox_override(state, sb)
	_hair_label.text = PlayerView.HAIR_STYLE_NAMES[appearance.hair_style]
	_preview.set_appearance(appearance)


func _cycle_hair(d: int) -> void:
	var n: int = PlayerView.HAIR_STYLE_NAMES.size()
	appearance.hair_style = (appearance.hair_style + d + n) % n
	_refresh_looks()


func _randomize() -> void:
	appearance = {
		"skin": randi() % PlayerView.SKINS.size(),
		"hair_style": randi() % PlayerView.HAIR_STYLE_NAMES.size(),
		"hair_color": randi() % PlayerView.HAIRS.size(),
		"shirt": randi() % PlayerView.SHIRTS.size(),
		"pants": randi() % PlayerView.PANTS.size(),
	}
	_refresh_looks()


# -------------------------------------------------------------- behaviour

func profile() -> Dictionary:
	return {
		"gender": gender_opt.selected,
		"age": int(age_spin.value),
		"city": city_edit.text.strip_edges(),
		"email": email_edit.text.strip_edges().to_lower(),
		"appearance": appearance.duplicate(),
	}


## Same rules as the server (validate_profile); returns an error or "".
func validation_error() -> String:
	if nick_edit.text.strip_edges().is_empty():
		return "Podaj imię."
	if city_edit.text.strip_edges().is_empty():
		return "Podaj miejscowość."
	var e := email_edit.text.strip_edges()
	var parts := e.split("@")
	if parts.size() != 2 or parts[0].is_empty() or not parts[1].contains(".") or parts[1].begins_with(".") \
			or parts[1].ends_with(".") or e.contains(" "):
		return "Podaj poprawny adres e-mail (np. ola.nowak@poczta.pl)."
	return ""


## Dev / autoconnect: fill in a valid character around a given name.
func set_defaults(nick: String, address: String) -> void:
	if nick != "":
		nick_edit.text = nick
	if city_edit.text == "":
		city_edit.text = "Warszawa"
	if email_edit.text == "" or nick != "":
		email_edit.text = "%s@poczta.pl" % nick_edit.text.to_lower().replace(" ", ".").validate_filename()
	addr_edit.text = address if address != "" else (addr_edit.text if addr_edit.text != "" else "127.0.0.1:7777")


func set_status(text: String, is_error := false) -> void:
	status.text = text
	status.modulate = Color(1, 0.45, 0.4) if is_error else Color(1, 1, 1, 0.8)


func set_busy(busy: bool) -> void:
	button.disabled = busy
	for c in [nick_edit, city_edit, email_edit, addr_edit]:
		c.editable = not busy


func _submit() -> void:
	if button.disabled:
		return
	var err := validation_error()
	if err != "":
		set_status(err, true)
		return
	_save()
	var addr := addr_edit.text.strip_edges()
	connect_pressed.emit(nick_edit.text.strip_edges(), profile(), addr if addr != "" else "127.0.0.1:7777")


func _save() -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("character", "nick", nick_edit.text.strip_edges())
	cfg.set_value("character", "gender", gender_opt.selected)
	cfg.set_value("character", "age", int(age_spin.value))
	cfg.set_value("character", "city", city_edit.text.strip_edges())
	cfg.set_value("character", "email", email_edit.text.strip_edges())
	cfg.set_value("character", "appearance", appearance)
	cfg.set_value("connection", "address", addr_edit.text.strip_edges())
	cfg.save(SAVE_PATH)


func _load() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(SAVE_PATH) != OK:
		_randomize()
		return
	nick_edit.text = cfg.get_value("character", "nick", "")
	gender_opt.select(clampi(cfg.get_value("character", "gender", 0), 0, GENDERS.size() - 1))
	age_spin.value = cfg.get_value("character", "age", 25)
	city_edit.text = cfg.get_value("character", "city", "")
	email_edit.text = cfg.get_value("character", "email", "")
	var a = cfg.get_value("character", "appearance", {})
	if typeof(a) == TYPE_DICTIONARY and a.size() == 5:
		appearance = a
	addr_edit.text = cfg.get_value("connection", "address", "")
