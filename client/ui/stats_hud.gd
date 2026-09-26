## Character needs (top right): hunger, energy, stress, bladder, 0..100 each
## from the server's Stats packet. Bars turn from green to red as a need gets
## worse and blink when it is critical.
extends Control

const PixelUI = preload("res://ui/pixel_ui.gd")

## [label, true if high = bad]
const ROWS := [["Głód", true], ["Energia", false], ["Stres", true], ["Toaleta", true], ["Higiena", false]]
const CRITICAL := 80

var values := [0, 100, 0, 0, 100]
var dirty_hands := false
var money := 0
var _dirty := Label.new()
var _money := Label.new()
var have := false
var _panel := PanelContainer.new()
var _bars: Array[Control] = []
var _nums: Array[Label] = []


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_panel.add_theme_stylebox_override("panel", PixelUI.box("hud"))
	_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_panel)
	var grid := GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 6)
	_panel.add_child(grid)
	for i in ROWS.size():
		var l := Label.new()
		l.text = ROWS[i][0]
		PixelUI.style_label(l, 16, PixelUI.TEXT)
		grid.add_child(l)
		var bar := Control.new()
		bar.custom_minimum_size = Vector2(128, 14)
		bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		var idx := i
		bar.draw.connect(func(): _draw_bar(bar, idx))
		grid.add_child(bar)
		_bars.append(bar)
		var n := Label.new()
		n.custom_minimum_size = Vector2(30, 0)
		n.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		PixelUI.style_label(n, 16, PixelUI.TEXT_DIM)
		grid.add_child(n)
		_nums.append(n)
	_dirty.text = "Brudne ręce — umyj je (umywalka / płyn)"
	PixelUI.style_label(_dirty, 16, Color("#ffb347"))
	_dirty.visible = false
	grid.get_parent().remove_child(grid)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 6)
	_panel.add_child(col)
	PixelUI.style_label(_money, 16, PixelUI.GOLD)
	col.add_child(_money)
	col.add_child(grid)
	col.add_child(_dirty)
	visible = false
	_panel.resized.connect(_place)
	get_viewport().size_changed.connect(_place)
	_place.call_deferred()


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_panel.position = Vector2(vs.x - _panel.size.x - 16, 16)


func update_stats(p: Dictionary) -> void:
	values = [p.hunger, p.energy, p.stress, p.bladder, p.hygiene]
	money = p.money
	_money.text = "Portfel: %d,%02d zł" % [money / 100, money % 100]
	dirty_hands = (p.stats_flags & 1) != 0
	var upset: bool = (p.stats_flags & 2) != 0
	var warn := ""
	if upset:
		warn = "Rozstrój żołądka — szybko do toalety!"
	elif dirty_hands:
		warn = "Brudne ręce — umyj je (umywalka / płyn)"
	_dirty.text = warn
	_dirty.add_theme_color_override("font_color", Color("#ff6b6b") if upset else Color("#ffb347"))
	if _dirty.visible != (warn != ""):
		_dirty.visible = warn != ""
		_panel.reset_size()  # shrink back when the line hides
	have = true
	visible = true
	for i in values.size():
		_nums[i].text = str(values[i])
		_bars[i].queue_redraw()


## 0 = fine .. 100 = terrible.
func badness(i: int) -> int:
	return values[i] if ROWS[i][1] else 100 - values[i]


func _process(_d: float) -> void:
	if not visible:
		return
	for i in values.size():
		if badness(i) >= CRITICAL:
			_bars[i].queue_redraw()  # blink


func _draw_bar(bar: Control, i: int) -> void:
	var r := Rect2(Vector2.ZERO, bar.size)
	var bad := badness(i)
	var c := Color("#4caf50").lerp(Color("#f1c40f"), clampf(bad / 50.0, 0, 1))
	if bad > 50:
		c = Color("#f1c40f").lerp(Color("#e74c3c"), clampf((bad - 50) / 40.0, 0, 1))
	if bad >= CRITICAL and (Time.get_ticks_msec() / 350) % 2 == 0:
		c = c.lightened(0.35)
	PixelUI.draw_bar(bar, r, values[i] / 100.0, c)
