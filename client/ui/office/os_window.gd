## A window on the office computer's desktop: a title bar (drag it, X to
## close) and a body that holds one app.
extends PanelContainer

const Ink = preload("res://ui/ink_ui.gd")

signal closed
signal focused

var content := MarginContainer.new()
var _title := Label.new()


func setup(title: String, body: Control, pad := 12) -> void:
	add_theme_stylebox_override("panel", Ink.box("paper"))
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 0)
	add_child(col)
	var bar := PanelContainer.new()
	bar.add_theme_stylebox_override("panel", Ink.box("title"))
	col.add_child(bar)
	var row := HBoxContainer.new()
	bar.add_child(row)
	Ink.style_label(_title, 16, Color.WHITE)
	_title.text = title
	_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(_title)
	var x := Ink.button("X", false, true)
	x.focus_mode = Control.FOCUS_NONE
	x.pressed.connect(func(): closed.emit())
	row.add_child(x)
	bar.gui_input.connect(_drag)
	gui_input.connect(func(ev):
			if ev is InputEventMouseButton and ev.pressed:
				focused.emit())
	content.size_flags_vertical = Control.SIZE_EXPAND_FILL
	for side in ["left", "right", "top", "bottom"]:
		content.add_theme_constant_override("margin_" + side, pad)
	col.add_child(content)
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_child(body)


func set_title(t: String) -> void:
	_title.text = t


func _drag(ev: InputEvent) -> void:
	if ev is InputEventMouseMotion and ev.button_mask & MOUSE_BUTTON_MASK_LEFT:
		var p := get_parent() as Control
		position += ev.relative
		if p:  # keep the title bar on the screen
			position.x = clampf(position.x, -size.x + 80, p.size.x - 80)
			position.y = clampf(position.y, 0, p.size.y - 40)
	elif ev is InputEventMouseButton and ev.pressed:
		focused.emit()
