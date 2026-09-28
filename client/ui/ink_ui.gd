## The game's UI kit in a hand-drawn "paper and ink" style (inspired by
## Don't Starve): a handwritten font (Patrick Hand, OFL - fonts/), parchment
## and dark-wood panels with wobbly ink outlines and torn edges, drawn in code
## as 9-slice textures, ink buttons, inputs, round badges and bars.
## Everything UI goes through here, so the HUD, the computer, the job portal
## and the dialogs share one look.
extends RefCounted

## Ink line width (screen pixels).
const LINE := 2

# Palette: parchment, ink, dark wood, burgundy, muted gold / green.
const INK := Color("#2a2118")
const DARK := Color("#3a2e24")          # HUD panels (dark wood / leather)
const DARK_HI := Color("#5a4636")
const DARK_LO := Color("#231b14")
const PAPER := Color("#e9dcbc")         # windows
const PAPER_HI := Color("#f4ead0")
const PAPER_LO := Color("#cdb98f")
const CARD := Color("#f2e7cb")
const CARD_LO := Color("#d9c7a0")
const ACCENT := Color("#8c3a2b")        # primary buttons, title bands
const ACCENT_HI := Color("#b0523e")
const ACCENT_LO := Color("#5e241a")
const GOLD := Color("#d4a94a")
const RED := Color("#a8402f")
const GREEN := Color("#6f8f3e")
const TEXT := Color("#f1e6c8")          # text on dark
const TEXT_DIM := Color("#c2af8a")
const TEXT_INK := Color("#2a2118")      # text on paper
const TEXT_MUTED := Color("#7d6a52")

static var _font: FontFile
static var _theme: Theme
static var _boxes := {}


static func font() -> FontFile:
	if _font == null:
		# The imported font (what an exported game has); without an import
		# yet (a fresh clone, *.import is not in git) the plain file.
		var path := "res://fonts/PatrickHand-Regular.ttf"
		if ResourceLoader.exists(path):
			_font = (load(path) as FontFile).duplicate()
		else:
			_font = FontFile.new()
			_font.load_dynamic_font(path)
		_font.antialiasing = TextServer.FONT_ANTIALIASING_GRAY
		_font.hinting = TextServer.HINTING_LIGHT
		_font.oversampling = 1.0
	return _font


## Patrick Hand is small for its size: everything a bit bigger, never tiny.
static func fs(n: int) -> int:
	return maxi(18, int(round(n * 1.25)))


## A Theme for the whole window: the handwritten font everywhere, ink
## buttons, inputs, scroll bars and panels by default.
static func theme() -> Theme:
	if _theme:
		return _theme
	var t := Theme.new()
	t.default_font = font()
	t.default_font_size = 20
	for st in ["normal", "hover", "pressed", "disabled", "focus"]:
		t.set_stylebox(st, "Button", button_box(st, false))
		t.set_stylebox(st, "OptionButton", button_box(st, false))
	for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color", "font_hover_pressed_color"]:
		for c in ["Button", "OptionButton", "CheckBox", "CheckButton"]:
			t.set_color(k, c, TEXT_INK)
	t.set_color("font_disabled_color", "Button", TEXT_MUTED)
	# Check boxes: just the tick box and the text, no button frame.
	for c in ["CheckBox", "CheckButton"]:
		for st in ["normal", "hover", "pressed", "focus", "hover_pressed", "disabled"]:
			t.set_stylebox(st, c, StyleBoxEmpty.new())
	for st in ["normal", "read_only"]:
		t.set_stylebox(st, "LineEdit", box("input"))
		t.set_stylebox(st, "TextEdit", box("input"))
	t.set_stylebox("focus", "LineEdit", box("input_focus"))
	t.set_stylebox("focus", "TextEdit", box("input_focus"))
	for c in ["LineEdit", "TextEdit", "SpinBox"]:
		t.set_color("font_color", c, TEXT_INK)
		t.set_color("font_placeholder_color", c, TEXT_MUTED)
		t.set_color("font_uneditable_color", c, Color(TEXT_INK, 0.7))
		t.set_color("caret_color", c, INK)
		t.set_color("selection_color", c, Color(GOLD, 0.45))
	t.set_stylebox("panel", "PanelContainer", box("paper"))
	t.set_stylebox("panel", "PopupMenu", box("paper"))
	t.set_color("font_color", "PopupMenu", TEXT_INK)
	t.set_color("font_hover_color", "PopupMenu", TEXT_INK)
	t.set_stylebox("hover", "PopupMenu", box("card_hover"))
	for bar in ["VScrollBar", "HScrollBar"]:
		t.set_stylebox("scroll", bar, box("scroll"))
		t.set_stylebox("grabber", bar, box("grabber"))
		t.set_stylebox("grabber_highlight", bar, box("grabber"))
		t.set_stylebox("grabber_pressed", bar, box("grabber"))
	t.set_color("font_color", "Label", TEXT)
	t.set_font_size("font_size", "Label", 20)
	t.set_font_size("font_size", "Button", 20)
	t.set_font_size("font_size", "LineEdit", 20)
	t.set_font_size("font_size", "TextEdit", 20)
	_theme = t
	return t


# ------------------------------------------------------------------ frames

## Named 9-slice frames: hud (dark wood), paper (window), card, card_hover,
## title (burgundy band), input, input_focus, screen (monitor), scroll,
## grabber, slot, slot_active, bubble, mine (own chat message).
static func box(kind: String) -> StyleBoxTexture:
	if _boxes.has(kind):
		return _boxes[kind]
	var spec := _spec(kind)
	var sb := _make_box(spec, hash(kind))
	_boxes[kind] = sb
	return sb


static func button_box(state: String, primary: bool, danger := false) -> StyleBoxTexture:
	var key := "btn_%s_%s_%s" % [state, primary, danger]
	if _boxes.has(key):
		return _boxes[key]
	var base: Color = RED if danger else (ACCENT if primary else CARD)
	match state:
		"hover", "focus":
			base = base.lightened(0.1)
		"pressed":
			base = base.darkened(0.12)
		"disabled":
			base = base.lerp(Color("#9d9282"), 0.55)
	var spec := {"fill": base, "grain": 0.05, "outline": INK, "width": 2.5, "torn": false,
		"shadow": state != "pressed", "pad": 14, "pad_v": 4, "corner": 7}
	var sb := _make_box(spec, hash(key))
	if state == "pressed":
		sb.content_margin_top += 2
		sb.content_margin_bottom -= 2
	_boxes[key] = sb
	return sb


static func _spec(kind: String) -> Dictionary:
	match kind:
		"hud":
			return {"fill": Color(DARK, 0.94), "grain": 0.06, "outline": INK, "width": 3.0, "torn": true, "shadow": true, "pad": 16, "pad_v": 12, "corner": 10}
		"paper":
			return {"fill": PAPER, "grain": 0.07, "outline": INK, "width": 3.0, "torn": true, "shadow": true, "pad": 18, "pad_v": 14, "corner": 8}
		"card":
			return {"fill": CARD, "grain": 0.05, "outline": Color(INK, 0.55), "width": 2.0, "torn": false, "shadow": false, "pad": 14, "pad_v": 12, "corner": 6}
		"card_hover":
			return {"fill": CARD.lightened(0.15), "grain": 0.05, "outline": Color(INK, 0.55), "width": 2.0, "torn": false, "shadow": false, "pad": 14, "pad_v": 12, "corner": 6}
		"title":
			return {"fill": ACCENT, "grain": 0.08, "outline": INK, "width": 3.0, "torn": false, "shadow": false, "pad": 14, "pad_v": 6, "corner": 6}
		"input":
			return {"fill": PAPER_HI, "grain": 0.03, "outline": Color(INK, 0.8), "width": 2.0, "torn": false, "shadow": false, "pad": 10, "pad_v": 4, "corner": 4}
		"input_focus":
			return {"fill": PAPER_HI, "grain": 0.03, "outline": ACCENT, "width": 3.0, "torn": false, "shadow": false, "pad": 10, "pad_v": 4, "corner": 4}
		"screen":
			return {"fill": Color("#2b2119"), "grain": 0.08, "outline": INK, "width": 4.0, "torn": false, "shadow": true, "pad": 16, "pad_v": 16, "corner": 14}
		"scroll":
			return {"fill": Color(INK, 0.12), "grain": 0.0, "outline": Color(0, 0, 0, 0), "width": 0.0, "torn": false, "shadow": false, "pad": 4, "pad_v": 4, "corner": 4}
		"grabber":
			return {"fill": DARK_HI, "grain": 0.05, "outline": INK, "width": 2.0, "torn": false, "shadow": false, "pad": 4, "pad_v": 4, "corner": 4}
		"slot":
			return {"fill": Color("#2b221a"), "grain": 0.08, "outline": INK, "width": 3.0, "torn": false, "shadow": false, "pad": 4, "pad_v": 4, "corner": 10}
		"slot_active":
			return {"fill": Color("#3d3024"), "grain": 0.08, "outline": GOLD, "width": 3.0, "torn": false, "shadow": false, "pad": 4, "pad_v": 4, "corner": 10}
		"bubble":
			return {"fill": PAPER_HI, "grain": 0.04, "outline": INK, "width": 2.5, "torn": false, "shadow": true, "pad": 10, "pad_v": 6, "corner": 10}
		"mine":
			return {"fill": Color("#efe0b0"), "grain": 0.04, "outline": Color(INK, 0.6), "width": 2.0, "torn": false, "shadow": false, "pad": 10, "pad_v": 6, "corner": 8}
	return _spec("paper")


## Draw a 64x64 frame: grainy fill, a wobbly ink outline (a bit thicker in
## places, like a brush), rounded corners, optionally torn edges and a soft
## drop shadow; then a 9-slice StyleBox with 22 px borders tiled along the
## edges.
static func _make_box(s: Dictionary, seed_value: int) -> StyleBoxTexture:
	var n := 64
	var m := 22
	var img := Image.create(n, n, false, Image.FORMAT_RGBA8)
	img.fill(Color(0, 0, 0, 0))
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	var inset := 4.0 if s.shadow else 2.0
	var corner: float = s.corner
	var width: float = s.width
	# Edge wobble per position along each side (same on opposite sides so
	# that tiling the border keeps continuity).
	var wob := []
	for i in n:
		wob.append(sin(i * 0.9 + rng.randf() * 0.5) * (0.9 if s.torn else 0.35) + (rng.randf() - 0.5) * (1.2 if s.torn else 0.3))
	for y in n:
		for x in n:
			# Distance inside the rounded rect (negative = outside).
			var ex: float = inset + wob[y] * 0.6
			var ey: float = inset + wob[x] * 0.6
			var dx := minf(x + 0.5 - ex, n - ex - (x + 0.5))
			var dy := minf(y + 0.5 - ey, n - ey - (y + 0.5))
			var d := minf(dx, dy)
			if dx < corner and dy < corner:
				d = corner - Vector2(corner - dx, corner - dy).length()
			if d < 0:
				# Shadow below / right.
				if s.shadow:
					var sd := minf(minf(x + 0.5 - (ex - 2), n - (ex - 3) - (x + 0.5)), minf(y + 0.5 - (ey - 2), n - (ey - 4) - (y + 0.5)))
					if sd >= 0 and (x > n / 2 or y > n / 2):
						img.set_pixel(x, y, Color(0, 0, 0, 0.22))
				continue
			var c: Color = s.fill
			var g: float = s.grain
			if g > 0:
				c = c.darkened(rng.randf() * g) if rng.randf() < 0.5 else c.lightened(rng.randf() * g * 0.6)
			var w := width + (0.8 if (x + y) % 11 < 3 else 0.0) * (1.0 if width > 0 else 0.0)
			if d < w:
				var a := clampf(w - d, 0.0, 1.0)
				c = c.lerp(s.outline, a * s.outline.a) if s.outline.a > 0 else c
				c.a = maxf(c.a, s.outline.a * a)
			elif d < 1.0:
				c.a *= d
			img.set_pixel(x, y, c)
	var sb := StyleBoxTexture.new()
	sb.texture = ImageTexture.create_from_image(img)
	for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
		sb.set_texture_margin(side, m)
	sb.axis_stretch_horizontal = StyleBoxTexture.AXIS_STRETCH_MODE_TILE_FIT
	sb.axis_stretch_vertical = StyleBoxTexture.AXIS_STRETCH_MODE_TILE_FIT
	sb.content_margin_left = s.pad
	sb.content_margin_right = s.pad
	sb.content_margin_top = s.pad_v
	sb.content_margin_bottom = s.pad_v
	# The frame's inner part is drawn with the texture's edge; keep content
	# clear of the ink.
	sb.expand_margin_left = 0
	return sb


# ----------------------------------------------------------------- widgets

static func button(text: String, primary := false, danger := false) -> Button:
	var b := Button.new()
	b.text = text
	b.add_theme_font_override("font", font())
	b.add_theme_font_size_override("font_size", 20)
	for st in ["normal", "hover", "pressed", "disabled", "focus"]:
		b.add_theme_stylebox_override(st, button_box(st, primary, danger))
	var fc := TEXT if (primary or danger) else TEXT_INK
	for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		b.add_theme_color_override(k, fc)
	b.add_theme_color_override("font_disabled_color", Color(fc, 0.55))
	b.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	return b


static func label(text: String, size := 16, color := TEXT_INK, wrap := false) -> Label:
	var l := Label.new()
	l.text = text
	style_label(l, size, color)
	if wrap:
		l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return l


static func style_label(l: Label, size: int, color: Color) -> void:
	l.add_theme_font_override("font", font())
	l.add_theme_font_size_override("font_size", fs(size))
	l.add_theme_color_override("font_color", color)


## A panel with one of the frames.
static func panel(kind := "paper") -> PanelContainer:
	var p := PanelContainer.new()
	p.add_theme_stylebox_override("panel", box(kind))
	return p


## A hand-inked bar into `c` at `r`: `value` 0..1.
static func draw_bar(c: CanvasItem, r: Rect2, value: float, color: Color) -> void:
	c.draw_rect(r, Color("#1e1712"))
	var fill := Rect2(r.position, Vector2(r.size.x * clampf(value, 0.0, 1.0), r.size.y))
	c.draw_rect(fill, color)
	c.draw_rect(Rect2(fill.position, Vector2(fill.size.x, 2)), color.lightened(0.25))
	c.draw_rect(r, INK, false, LINE)


## A filled circle (sun, moon...).
static func draw_disc(c: CanvasItem, center: Vector2, radius: float, color: Color) -> void:
	c.draw_circle(center, radius, color)
	c.draw_arc(center, radius, 0, TAU, 48, Color(INK, 0.6), 2.0, true)
