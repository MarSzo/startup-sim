## Character needs (top right) as round badges, Don't Starve style: each is
## a jar of coloured liquid that drains as the need gets worse, with an ink
## ring and an icon; the number shows on hover (and when it's critical the
## badge pulses and shows it anyway). Wallet as a coin; warnings (dirty
## hands, upset stomach) on a paper note below. From the server's Stats.
extends Control

const Ink = preload("res://ui/ink_ui.gd")

## [name, true if high = bad, liquid colour, icon]
const ROWS := [
	["Głód", true, Color("#c9803a"), "food"],
	["Energia", false, Color("#d8b83e"), "bolt"],
	["Stres", true, Color("#8a5aa8"), "brain"],
	["Toaleta", true, Color("#4f8fc0"), "drop"],
	["Higiena", false, Color("#5aa89a"), "soap"],
]
const CRITICAL := 80
const SIZE := 64.0
const GAP := 10.0

var values := [0, 100, 0, 0, 100]
var dirty_hands := false
var money := 0
var have := false
var _badges: Array[Control] = []
var _hover := -1
var _coin := Control.new()
var _note := Label.new()
var _row := HBoxContainer.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_row.add_theme_constant_override("separation", int(GAP))
	_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_row)
	_coin.custom_minimum_size = Vector2(120, SIZE)
	_coin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_coin.draw.connect(_draw_coin)
	_row.add_child(_coin)
	for i in ROWS.size():
		var b := Control.new()
		b.custom_minimum_size = Vector2(SIZE, SIZE + 22)
		b.mouse_filter = Control.MOUSE_FILTER_PASS
		b.tooltip_text = ROWS[i][0]
		var idx := i
		b.draw.connect(func(): _draw_badge(b, idx))
		b.mouse_entered.connect(func(): _hover = idx)
		b.mouse_exited.connect(func(): if _hover == idx: _hover = -1)
		_row.add_child(b)
		_badges.append(b)
	_note.add_theme_stylebox_override("normal", Ink.box("bubble"))
	Ink.style_label(_note, 16, Ink.TEXT_INK)
	_note.visible = false
	add_child(_note)
	visible = false
	get_viewport().size_changed.connect(_place)
	_row.resized.connect(_place)
	_place.call_deferred()


func _place() -> void:
	var vs := get_viewport_rect().size
	position = Vector2.ZERO
	size = vs
	_row.reset_size()
	_row.position = Vector2(vs.x - _row.size.x - 18, 14)
	_note.reset_size()
	_note.position = Vector2(vs.x - _note.size.x - 18, _row.position.y + _row.size.y + 4)


func update_stats(p: Dictionary) -> void:
	values = [p.hunger, p.energy, p.stress, p.bladder, p.hygiene]
	money = p.money
	dirty_hands = (p.stats_flags & 1) != 0
	var upset: bool = (p.stats_flags & 2) != 0
	var warn := ""
	if upset:
		warn = "Rozstrój żołądka — szybko do toalety!"
	elif dirty_hands:
		warn = "Brudne ręce — umyj je (umywalka / płyn)"
	_note.text = warn
	_note.add_theme_color_override("font_color", Ink.RED if upset else Ink.TEXT_INK)
	_note.visible = warn != ""
	have = true
	visible = true
	_coin.queue_redraw()
	_place()


## 0 = fine .. 100 = terrible.
func badness(i: int) -> int:
	return values[i] if ROWS[i][1] else 100 - values[i]


func _process(_d: float) -> void:
	if visible:
		for b in _badges:
			b.queue_redraw()  # sloshing liquid, pulsing when critical


func _draw_coin() -> void:
	var c := _coin
	var r := 20.0
	var center := Vector2(c.size.x - r - 4, SIZE / 2)
	c.draw_circle(center + Vector2(2, 3), r, Color(0, 0, 0, 0.3))
	c.draw_circle(center, r, Ink.GOLD)
	c.draw_circle(center, r - 5, Ink.GOLD.lightened(0.18))
	c.draw_arc(center, r, 0, TAU, 40, Ink.INK, 3.0, true)
	c.draw_arc(center, r - 5, 0, TAU, 40, Color(Ink.INK, 0.5), 1.5, true)
	var f := Ink.font()
	c.draw_string(f, center + Vector2(-6, 8), "zł", HORIZONTAL_ALIGNMENT_LEFT, -1, 20, Ink.INK)
	var txt := "%d,%02d" % [money / 100, money % 100]
	var w := f.get_string_size(txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 24).x
	var pos := Vector2(center.x - r - 8 - w, center.y + 8)
	c.draw_string_outline(f, pos, txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 24, 6, Ink.INK)
	c.draw_string(f, pos, txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 24, Ink.TEXT)


func _draw_badge(b: Control, i: int) -> void:
	var t := Time.get_ticks_msec() / 1000.0
	var bad := badness(i)
	var level := clampf(1.0 - bad / 100.0, 0.0, 1.0)
	var critical := bad >= CRITICAL
	var pulse := 1.0 + (0.06 * sin(t * 8.0) if critical else 0.0)
	var r := SIZE / 2 - 3
	var c := Vector2(SIZE / 2, SIZE / 2)
	# Shadow, dark glass.
	b.draw_circle(c + Vector2(2, 3), r * pulse, Color(0, 0, 0, 0.35))
	b.draw_circle(c, r * pulse, Color("#1e1712"))
	# Liquid up to `level`, with a little wave on top.
	var col: Color = ROWS[i][2]
	if critical:
		col = col.lerp(Ink.RED, 0.5 + 0.3 * sin(t * 8.0))
	var inner := (r - 3) * pulse
	var surface := c.y + inner - level * inner * 2
	for y in range(int(c.y - inner), int(c.y + inner) + 1):
		var fy := float(y) + 0.5
		var dy := fy - c.y
		var half := sqrt(maxf(inner * inner - dy * dy, 0.0))
		if half <= 0:
			continue
		# Wave: the surface wobbles along x; draw in two halves.
		var x0 := c.x - half
		var x1 := c.x + half
		var wave_l := surface + sin(t * 2.2 + x0 * 0.15) * 1.5
		var wave_r := surface + sin(t * 2.2 + x1 * 0.15) * 1.5
		if fy >= maxf(wave_l, wave_r):
			b.draw_line(Vector2(x0, fy), Vector2(x1, fy), col if fy > surface + 3 else col.lightened(0.25), 1.0)
		elif fy >= minf(wave_l, wave_r):
			var mid := c.x
			if wave_l < wave_r:
				b.draw_line(Vector2(x0, fy), Vector2(mid, fy), col.lightened(0.25), 1.0)
			else:
				b.draw_line(Vector2(mid, fy), Vector2(x1, fy), col.lightened(0.25), 1.0)
	# Glass shine and the ink ring.
	b.draw_arc(c, inner - 4, PI * 1.1, PI * 1.45, 12, Color(1, 1, 1, 0.25), 3.0, true)
	b.draw_arc(c, r * pulse, 0, TAU, 48, Ink.INK, 4.0, true)
	b.draw_arc(c, r * pulse - 3, 0, TAU, 48, Color(Ink.DARK_HI, 0.8), 1.5, true)
	_draw_icon(b, ROWS[i][3], c)
	# The number: on hover, or when critical.
	if _hover == i or critical:
		var f := Ink.font()
		var txt := str(values[i])
		var w := f.get_string_size(txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 20).x
		var p := Vector2(c.x - w / 2, SIZE + 18)
		b.draw_string_outline(f, p, txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 20, 5, Ink.INK)
		b.draw_string(f, p, txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 20, Ink.TEXT)
	elif _hover == -1:
		pass


## Simple inked icons in the middle of a badge.
func _draw_icon(b: Control, kind: String, c: Vector2) -> void:
	var ink := Ink.INK
	var fill := Ink.PAPER_HI
	match kind:
		"food":  # a drumstick
			b.draw_line(c + Vector2(4, 4), c + Vector2(12, 12), ink, 6.0, true)
			b.draw_line(c + Vector2(4, 4), c + Vector2(12, 12), fill, 3.0, true)
			b.draw_circle(c + Vector2(13, 11), 3.5, ink)
			b.draw_circle(c + Vector2(11, 14), 3.5, ink)
			b.draw_circle(c + Vector2(13, 11), 2.0, fill)
			b.draw_circle(c + Vector2(11, 14), 2.0, fill)
			b.draw_circle(c + Vector2(-3, -3), 11, ink)
			b.draw_circle(c + Vector2(-3, -3), 8.5, Color("#b8733a"))
			b.draw_circle(c + Vector2(-6, -6), 3, Color("#d99a5a"))
		"bolt":
			var pts := PackedVector2Array([c + Vector2(3, -16), c + Vector2(-9, 2), c + Vector2(-1, 2), c + Vector2(-4, 16), c + Vector2(9, -3), c + Vector2(1, -3), c + Vector2(3, -16)])
			b.draw_colored_polygon(pts, fill)
			b.draw_polyline(pts, ink, 2.5, true)
		"brain":
			for p in [Vector2(-6, -3), Vector2(6, -3), Vector2(-4, 6), Vector2(4, 6)]:
				b.draw_circle(c + p, 8.5, ink)
			for p in [Vector2(-6, -3), Vector2(6, -3), Vector2(-4, 6), Vector2(4, 6)]:
				b.draw_circle(c + p, 6.5, Color("#e7b8c8"))
			b.draw_line(c + Vector2(0, -10), c + Vector2(0, 12), ink, 2.0, true)
			b.draw_arc(c + Vector2(-6, 0), 4, 0.3, 2.6, 8, ink, 1.5, true)
			b.draw_arc(c + Vector2(6, 0), 4, 0.5, 2.8, 8, ink, 1.5, true)
		"drop":
			var pts := PackedVector2Array()
			pts.append(c + Vector2(0, -16))
			for k in 13:
				var a := PI * (-0.15 + k / 12.0 * 1.3)
				pts.append(c + Vector2(cos(a) * 10, 5 + sin(a) * 10))
			pts.append(c + Vector2(0, -16))
			b.draw_colored_polygon(pts, Color("#cfe6f5"))
			b.draw_polyline(pts, ink, 2.5, true)
			b.draw_arc(c + Vector2(-3, 6), 4, PI * 0.6, PI * 1.1, 6, Color.WHITE, 2.0, true)
		"soap":
			var r := Rect2(c + Vector2(-12, -2), Vector2(22, 12))
			b.draw_rect(r, Color("#f1c9d8"))
			b.draw_rect(r, ink, false, 2.5)
			for p in [Vector2(-6, -10), Vector2(3, -13), Vector2(9, -7)]:
				b.draw_circle(c + p, 4, Color(1, 1, 1, 0.85))
				b.draw_arc(c + p, 4, 0, TAU, 12, ink, 1.5, true)
