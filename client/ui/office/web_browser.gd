## The office browser's Internet: a real web page in a native WebView
## (godot_wry, built by tools/build_webview.sh) with an address bar, back
## and reload. The WebView is a native view drawn over the game, so the
## screen hides it whenever something should be on top (`set_shown`).
## Without the extension (CI, tests, other systems) the page can only be
## opened in the player's own browser.
extends VBoxContainer

const Ink = preload("res://ui/ink_ui.gd")

const START := "https://www.onet.pl/"
const SEARCH := "https://duckduckgo.com/?q="
const TEXT := Color("#1c2430")

var url := START
var _addr := LineEdit.new()
var _view: Control = null  # the WebView node (null without the extension)
var _shown := true


## The extension is there and there's a window to draw it in.
static func available() -> bool:
	return ClassDB.class_exists("WebView") and DisplayServer.get_name() != "headless"


## What was typed into the address bar -> a URL (words: a web search).
static func to_url(typed: String) -> String:
	var t := typed.strip_edges()
	if t == "":
		return START
	if t.begins_with("http://") or t.begins_with("https://"):
		return t
	if " " not in t and "." in t:
		return "https://" + t
	return SEARCH + t.uri_encode()


func _init() -> void:
	add_theme_constant_override("separation", 6)
	size_flags_vertical = Control.SIZE_EXPAND_FILL
	var bar := HBoxContainer.new()
	bar.add_theme_constant_override("separation", 6)
	add_child(bar)
	var back := Ink.button("←")
	back.tooltip_text = "Wstecz"
	back.pressed.connect(func(): _call("eval", "history.back()"))
	bar.add_child(back)
	var reload := Ink.button("⟳")
	reload.tooltip_text = "Odśwież"
	reload.pressed.connect(func(): _call("reload"))
	bar.add_child(reload)
	_addr.text = url
	_addr.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_addr.select_all_on_focus = true
	_addr.add_theme_font_size_override("font_size", 15)
	_addr.add_theme_stylebox_override("normal", Ink.box("input"))
	_addr.add_theme_stylebox_override("focus", Ink.box("input_focus"))
	_addr.add_theme_color_override("font_color", TEXT)
	_addr.text_submitted.connect(go)
	bar.add_child(_addr)
	var out := Ink.button("↗")
	out.tooltip_text = "Otwórz w swojej przeglądarce"
	out.pressed.connect(func(): open_outside(url))
	bar.add_child(out)
	if available():
		_view = ClassDB.instantiate("WebView")
		_view.set("url", START)
		_view.set("full_window_size", false)
		_view.set("focused_when_created", false)
		_view.set("forward_input_events", true)
		_view.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		_view.size_flags_vertical = Control.SIZE_EXPAND_FILL
		_view.connect("page_load_started", _on_load)
		add_child(_view)
	else:
		var box := VBoxContainer.new()
		box.alignment = BoxContainer.ALIGNMENT_CENTER
		box.size_flags_vertical = Control.SIZE_EXPAND_FILL
		add_child(box)
		var l := Label.new()
		Ink.style_label(l, 18, TEXT)
		l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		l.text = "Przeglądarka nie działa na tym komputerze.\nStronę możesz otworzyć w swojej przeglądarce."
		box.add_child(l)
		var open := Ink.button("Otwórz %s ↗" % START, true)
		open.pressed.connect(func(): open_outside(url))
		box.add_child(open)


## Load what was typed (or a link).
func go(typed: String) -> void:
	url = to_url(typed)
	_addr.text = url
	_addr.release_focus()
	_call("load_url", url)


## The screen: hidden while another window, a dialog or the game is on top
## (the native view would cover them).
func set_shown(on: bool) -> void:
	_shown = on
	if _view != null:
		_view.visible = on


func _on_load(loaded: String) -> void:
	url = loaded
	if not _addr.has_focus():
		_addr.text = loaded


func _call(method: String, arg: Variant = null) -> void:
	if _view == null:
		return
	if arg == null:
		_view.call(method)
	else:
		_view.call(method, arg)


## In the player's own browser (never in tests / headless).
static func open_outside(page: String) -> void:
	if DisplayServer.get_name() != "headless":
		OS.shell_open(page)
