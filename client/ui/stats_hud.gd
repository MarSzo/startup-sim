## Character needs (top right): hunger, energy, stress, bladder, 0..100 each
## from the server's Stats packet. Bars turn from green to red as a need gets
## worse and blink when it is critical.
extends Control

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
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.07, 0.08, 0.12, 0.82)
	sb.set_corner_radius_all(8)
	sb.set_content_margin_all(10)
	_panel.add_theme_stylebox_override("panel", sb)
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
		l.add_theme_font_size_override("font_size", 14)
		l.add_theme_color_override("font_color", Color.WHITE)
		grid.add_child(l)
		var bar := Control.new()
		bar.custom_minimum_size = Vector2(120, 12)
		bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		var idx := i
		bar.draw.connect(func(): _draw_bar(bar, idx))
		grid.add_child(bar)
		_bars.append(bar)
		var n := Label.new()
		n.custom_minimum_size = Vector2(30, 0)
		n.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		n.add_theme_font_size_override("font_size", 13)
		n.add_theme_color_override("font_color", Color(1, 1, 1, 0.7))
		grid.add_child(n)
		_nums.append(n)
	_dirty.text = "Brudne ręce — umyj je (umywalka / płyn)"
	_dirty.add_theme_font_size_override("font_size", 12)
	_dirty.add_theme_color_override("font_color", Color("#ffb347"))
	_dirty.visible = false
	grid.get_parent().remove_child(grid)
	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 6)
	_panel.add_child(col)
	_money.add_theme_font_size_override("font_size", 14)
	_money.add_theme_color_override("font_color", Color("#f7d774"))
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
	bar.draw_rect(r, Color(1, 1, 1, 0.1))
	var bad := badness(i)
	var c := Color("#4caf50").lerp(Color("#f1c40f"), clampf(bad / 50.0, 0, 1))
	if bad > 50:
		c = Color("#f1c40f").lerp(Color("#e74c3c"), clampf((bad - 50) / 40.0, 0, 1))
	if bad >= CRITICAL and (Time.get_ticks_msec() / 350) % 2 == 0:
		c = c.lightened(0.35)
	bar.draw_rect(Rect2(0, 0, bar.size.x * values[i] / 100.0, bar.size.y), c)
	bar.draw_rect(r, Color(1, 1, 1, 0.25), false, 1.0)
