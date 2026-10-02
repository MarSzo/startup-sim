## The terminal app (Ghostty-like: dark, monospaced, a block cursor): the
## make-believe shell (shell.gd). ↑/↓ walk the history.
extends PanelContainer

const Shell = preload("res://ui/office/shell.gd")

signal exit_requested

const BG := Color("#1e1f29")
const FG := Color("#e6e6e6")
const PROMPT := Color("#8be9fd")
const MAX_LINES := 400

var shell := Shell.new()
var _out := RichTextLabel.new()
var _input := LineEdit.new()
var _prompt := Label.new()
var _hist_at := -1


func _init() -> void:
	var sb := StyleBoxFlat.new()
	sb.bg_color = BG
	sb.set_content_margin_all(10)
	sb.set_corner_radius_all(6)
	add_theme_stylebox_override("panel", sb)
	var mono := SystemFont.new()
	mono.font_names = PackedStringArray(["JetBrains Mono", "Menlo", "SF Mono", "Monaco", "Consolas", "DejaVu Sans Mono", "monospace"])
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 4)
	add_child(col)
	_out.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_out.scroll_following = true
	_out.selection_enabled = true
	_out.bbcode_enabled = false
	_out.add_theme_font_override("normal_font", mono)
	_out.add_theme_font_size_override("normal_font_size", 15)
	_out.add_theme_color_override("default_color", FG)
	col.add_child(_out)
	var row := HBoxContainer.new()
	col.add_child(row)
	_prompt.add_theme_font_override("font", mono)
	_prompt.add_theme_font_size_override("font_size", 15)
	_prompt.add_theme_color_override("font_color", PROMPT)
	row.add_child(_prompt)
	_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_input.flat = true
	_input.caret_blink = true
	_input.caret_force_displayed = true
	_input.add_theme_font_override("font", mono)
	_input.add_theme_font_size_override("font_size", 15)
	_input.add_theme_color_override("font_color", FG)
	_input.add_theme_color_override("caret_color", Color("#f8f8f2"))
	_input.add_theme_constant_override("caret_width", 8)  # a block cursor
	for st in ["normal", "focus", "read_only"]:
		_input.add_theme_stylebox_override(st, StyleBoxEmpty.new())  # no field: text on the dark screen
	_input.text_submitted.connect(_submit)
	_input.gui_input.connect(_history_keys)
	row.add_child(_input)
	custom_minimum_size = Vector2(520, 320)


func setup(nick: String, company: String, department: String) -> void:
	shell.setup(nick, company, department)
	_out.text = "StartOS 4.2 — terminal (wpisz help)\n%s\n\n" % shell.run("cat /etc/motd")
	shell.history.clear()
	_prompt.text = shell.prompt()


## The clock and the weather, for `date` and `curl wttr.in`.
func set_context(day: int, minute: int, weather: String) -> void:
	shell.day = day
	shell.minute = minute
	shell.weather = weather


func focus() -> void:
	_input.grab_focus.call_deferred()


## Run a line as if typed (the dev command / tests).
func run_command(line: String) -> String:
	var out := shell.run(line)
	if out == Shell.CLEAR:
		_out.text = ""
	elif out == Shell.EXIT:
		exit_requested.emit()
	else:
		_out.text += shell.prompt() + line + "\n" + (out + "\n" if out != "" else "")
		var lines := _out.text.split("\n")
		if lines.size() > MAX_LINES:
			_out.text = "\n".join(lines.slice(lines.size() - MAX_LINES))
	_prompt.text = shell.prompt()
	_hist_at = -1
	return out


func _submit(line: String) -> void:
	_input.text = ""
	run_command(line)
	focus()


func _history_keys(e: InputEvent) -> void:
	if not (e is InputEventKey and e.pressed):
		return
	var h := shell.history
	if h.is_empty():
		return
	if e.keycode == KEY_UP:
		_hist_at = h.size() - 1 if _hist_at < 0 else maxi(_hist_at - 1, 0)
	elif e.keycode == KEY_DOWN and _hist_at >= 0:
		_hist_at += 1
		if _hist_at >= h.size():
			_hist_at = -1
			_input.text = ""
			accept_event()
			return
	else:
		return
	_input.text = h[_hist_at]
	_input.caret_column = _input.text.length()
	accept_event()
