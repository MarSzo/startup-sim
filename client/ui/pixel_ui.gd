## The game's pixel-art UI kit: the pixel font (Pixelify Sans, OFL - see
## fonts/OFL.txt), chunky 9-slice frames drawn in code (one art pixel =
## `PX` screen pixels), buttons, inputs, bars and a Theme for the whole
## window. Everything UI goes through here, so the HUD, the computer, the
## job portal and the dialogs share one look.
extends RefCounted

## Screen pixels per art pixel.
const PX := 2

# Palette (office at night, warm paper, card gold).
const INK := Color("#161826")          # outlines, text on paper
const NAVY := Color("#262a3d")         # HUD panels
const NAVY_HI := Color("#3d4360")
const NAVY_LO := Color("#1a1c2a")
const PAPER := Color("#f3ead7")        # windows
const PAPER_HI := Color("#fffaf0")
const PAPER_LO := Color("#d8cbb0")
const CARD := Color("#fffaf0")
const CARD_LO := Color("#e4d8bf")
const BLUE := Color("#3f63c8")         # primary buttons, title bars
const BLUE_HI := Color("#6d8ff0")
const BLUE_LO := Color("#2a428a")
const GOLD := Color("#e0b040")
const RED := Color("#d24b4b")
const GREEN := Color("#5bb04b")
const TEXT := Color("#f4f1e8")          # text on dark
const TEXT_DIM := Color("#a9adc4")
const TEXT_INK := Color("#2a2433")      # text on paper
const TEXT_MUTED := Color("#7a6f63")

static var _font: FontFile
static var _theme: Theme
static var _boxes := {}


static func font() -> FontFile:
	if _font == null:
		_font = FontFile.new()
		# Read as a plain file (no import step; *.import is not in git): an
		# export has to include fonts/*.ttf in its non-resource files filter.
		_font.load_dynamic_font("res://fonts/PixelifySans.ttf")
		_font.antialiasing = TextServer.FONT_ANTIALIASING_NONE
		_font.hinting = TextServer.HINTING_NONE
		_font.subpixel_positioning = TextServer.SUBPIXEL_POSITIONING_DISABLED
		_font.force_autohinter = false
		# Its "fi" / "fl" ligatures draw as odd glyphs: letters one by one.
		_font.opentype_feature_overrides = {
			TextServerManager.get_primary_interface().name_to_tag("liga"): 0,
			TextServerManager.get_primary_interface().name_to_tag("clig"): 0,
			TextServerManager.get_primary_interface().name_to_tag("dlig"): 0,
		}
	return _font


## Snap a font size to the steps the pixel font looks right at.
static func fs(n: int) -> int:
	if n <= 17:
		return 16
	if n <= 21:
		return 20
	if n <= 28:
		return 24
	return 32


## A Theme for the whole window: pixel font everywhere, pixel buttons,
## inputs, scroll bars and panels by default.
static func theme() -> Theme:
	if _theme:
		return _theme
	var t := Theme.new()
	t.default_font = font()
	t.default_font_size = 16
	for st in ["normal", "hover", "pressed", "disabled", "focus"]:
		t.set_stylebox(st, "Button", button_box(st, false))
		t.set_stylebox(st, "OptionButton", button_box(st, false))
	for k in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		t.set_color(k, "Button", TEXT_INK)
		t.set_color(k, "OptionButton", TEXT_INK)
	t.set_color("font_disabled_color", "Button", TEXT_MUTED)
	for st in ["normal", "read_only"]:
		t.set_stylebox(st, "LineEdit", box("input"))
		t.set_stylebox(st, "TextEdit", box("input"))
	t.set_stylebox("focus", "LineEdit", box("input_focus"))
	t.set_stylebox("focus", "TextEdit", box("input_focus"))
	for c in ["LineEdit", "TextEdit"]:
		t.set_color("font_color", c, TEXT_INK)
		t.set_color("font_placeholder_color", c, TEXT_MUTED)
		t.set_color("caret_color", c, INK)
		t.set_color("selection_color", c, Color(BLUE_HI, 0.45))
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
	t.set_constant("outline_size", "Label", 0)
	_theme = t
	return t


# ------------------------------------------------------------------ frames

## Named 9-slice frames:
## hud (dark translucent), paper (window), card, card_hover, title (blue bar),
## input, input_focus, screen (monitor bezel), scroll, grabber, slot,
## slot_active, bubble, tooltip.
static func box(kind: String) -> StyleBoxTexture:
	if _boxes.has(kind):
		return _boxes[kind]
	var sb := StyleBoxTexture.new()
	var spec := _spec(kind)
	sb.texture = _frame_texture(spec)
	var m: int = spec.edge * PX
	sb.texture_margin_left = m
	sb.texture_margin_right = m
	sb.texture_margin_top = m
	sb.texture_margin_bottom = m
	sb.content_margin_left = spec.pad
	sb.content_margin_right = spec.pad
	sb.content_margin_top = spec.pad_v
	sb.content_margin_bottom = spec.pad_v
	_boxes[kind] = sb
	return sb


static func button_box(state: String, primary: bool, danger := false) -> StyleBoxTexture:
	var key := "btn_%s_%s_%s" % [state, primary, danger]
	if _boxes.has(key):
		return _boxes[key]
	var base: Color = RED if danger else (BLUE if primary else Color("#e6dcc6"))
	match state:
		"hover", "focus":
			base = base.lightened(0.12)
		"pressed":
			base = base.darkened(0.12)
		"disabled":
			base = base.lerp(Color("#9a9488"), 0.6)
	var spec := {
		"fill": base, "outline": INK,
		"hi": base.lightened(0.28), "lo": base.darkened(0.3),
		"edge": 3, "pad": 12, "pad_v": 6, "notch": true, "pressed": state == "pressed",
	}
	var sb := StyleBoxTexture.new()
	sb.texture = _frame_texture(spec)
	for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
		sb.set_texture_margin(side, 3 * PX)
	sb.content_margin_left = 12
	sb.content_margin_right = 12
	sb.content_margin_top = 5 + (2 if state == "pressed" else 0)
	sb.content_margin_bottom = 7 - (2 if state == "pressed" else 0)
	_boxes[key] = sb
	return sb


static func _spec(kind: String) -> Dictionary:
	match kind:
		"hud":
			return {"fill": Color(NAVY, 0.94), "outline": INK, "hi": NAVY_HI, "lo": NAVY_LO, "edge": 3, "pad": 12, "pad_v": 10, "notch": true}
		"paper":
			return {"fill": PAPER, "outline": INK, "hi": PAPER_HI, "lo": PAPER_LO, "edge": 3, "pad": 14, "pad_v": 12, "notch": true}
		"card":
			return {"fill": CARD, "outline": Color("#b9a98a"), "hi": Color.WHITE, "lo": CARD_LO, "edge": 2, "pad": 12, "pad_v": 10, "notch": false}
		"card_hover":
			return {"fill": Color("#fff4d6"), "outline": Color("#b9a98a"), "hi": Color.WHITE, "lo": CARD_LO, "edge": 2, "pad": 12, "pad_v": 10, "notch": false}
		"title":
			return {"fill": BLUE, "outline": INK, "hi": BLUE_HI, "lo": BLUE_LO, "edge": 3, "pad": 12, "pad_v": 6, "notch": false}
		"input":
			return {"fill": Color("#fffdf7"), "outline": INK, "hi": Color("#c9bda3"), "lo": Color("#fffdf7"), "edge": 3, "pad": 8, "pad_v": 6, "notch": false, "inset": true}
		"input_focus":
			return {"fill": Color("#fffdf7"), "outline": BLUE, "hi": Color("#c9bda3"), "lo": Color("#fffdf7"), "edge": 3, "pad": 8, "pad_v": 6, "notch": false, "inset": true}
		"screen":
			return {"fill": Color("#1c1f26"), "outline": Color("#0b0c10"), "hi": Color("#3a3f4c"), "lo": Color("#12141a"), "edge": 4, "pad": 14, "pad_v": 14, "notch": true}
		"scroll":
			return {"fill": Color(0, 0, 0, 0.12), "outline": Color(0, 0, 0, 0), "hi": Color(0, 0, 0, 0.12), "lo": Color(0, 0, 0, 0.12), "edge": 2, "pad": 4, "pad_v": 4, "notch": false}
		"grabber":
			return {"fill": Color("#8c8474"), "outline": INK, "hi": Color("#b3ab99"), "lo": Color("#6a6356"), "edge": 2, "pad": 4, "pad_v": 4, "notch": true}
		"slot":
			return {"fill": Color("#1f2233"), "outline": INK, "hi": NAVY_LO, "lo": NAVY_HI, "edge": 2, "pad": 2, "pad_v": 2, "notch": false, "inset": true}
		"slot_active":
			return {"fill": Color("#2c3150"), "outline": GOLD, "hi": NAVY_LO, "lo": NAVY_HI, "edge": 2, "pad": 2, "pad_v": 2, "notch": false, "inset": true}
		"bubble":
			return {"fill": Color("#fffdf7"), "outline": INK, "hi": Color.WHITE, "lo": Color("#e2d9c6"), "edge": 2, "pad": 8, "pad_v": 6, "notch": true}
		"mine":
			return {"fill": Color("#dbe6ff"), "outline": Color("#6d8ff0"), "hi": Color("#eef3ff"), "lo": Color("#c3d3fb"), "edge": 2, "pad": 10, "pad_v": 6, "notch": true}
	return _spec("paper")


## Draw a (3·edge+1)-art-pixel square frame and scale it up by PX: outline,
## bevel (light top-left, dark bottom-right; swapped when inset / pressed),
## notched corners.
static func _frame_texture(s: Dictionary) -> ImageTexture:
	var e: int = s.edge
	var n := e * 2 + 2
	var img := Image.create(n, n, false, Image.FORMAT_RGBA8)
	img.fill(s.fill)
	var inset: bool = s.get("inset", false) or s.get("pressed", false)
	var hi: Color = s.lo if inset else s.hi
	var lo: Color = s.hi if inset else s.lo
	for i in n:
		img.set_pixel(i, 0, s.outline)
		img.set_pixel(i, n - 1, s.outline)
		img.set_pixel(0, i, s.outline)
		img.set_pixel(n - 1, i, s.outline)
	if e >= 2:
		for i in range(1, n - 1):
			img.set_pixel(i, 1, hi)
			img.set_pixel(1, i, hi)
			img.set_pixel(i, n - 2, lo)
			img.set_pixel(n - 2, i, lo)
	if e >= 3 and not s.get("inset", false):
		for i in range(2, n - 2):
			img.set_pixel(i, n - 3, Color(lo, lo.a * 0.5))
	if s.get("notch", false):
		for c in [Vector2i(0, 0), Vector2i(n - 1, 0), Vector2i(0, n - 1), Vector2i(n - 1, n - 1)]:
			img.set_pixel(c.x, c.y, Color(0, 0, 0, 0))
		for c in [Vector2i(1, 1), Vector2i(n - 2, 1), Vector2i(1, n - 2), Vector2i(n - 2, n - 2)]:
			img.set_pixel(c.x, c.y, s.outline)
	img.resize(n * PX, n * PX, Image.INTERPOLATE_NEAREST)
	return ImageTexture.create_from_image(img)


# ----------------------------------------------------------------- widgets

static func button(text: String, primary := false, danger := false) -> Button:
	var b := Button.new()
	b.text = text
	b.add_theme_font_override("font", font())
	b.add_theme_font_size_override("font_size", 16)
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


## Draw a segmented pixel bar (stats) into `c` at `r`: `value` 0..1.
static func draw_bar(c: CanvasItem, r: Rect2, value: float, color: Color) -> void:
	c.draw_rect(r, INK)
	var inner := r.grow(-PX)
	c.draw_rect(inner, Color("#12141f"))
	var segs := int(inner.size.x / (4 * PX))
	var w: float = inner.size.x / maxi(segs, 1)
	var lit := int(round(clampf(value, 0.0, 1.0) * segs))
	for i in lit:
		var sr := Rect2(inner.position.x + i * w, inner.position.y, w - PX, inner.size.y)
		c.draw_rect(sr, color)
		c.draw_rect(Rect2(sr.position, Vector2(sr.size.x, PX)), color.lightened(0.3))


## A filled circle made of `cell`-sized squares (sun, moon...).
static func draw_pixel_circle(c: CanvasItem, center: Vector2, radius: float, color: Color, cell := 4.0) -> void:
	var n := int(ceil(radius / cell))
	for y in range(-n, n + 1):
		for x in range(-n, n + 1):
			var p := Vector2(x + 0.5, y + 0.5) * cell
			if p.length() <= radius:
				c.draw_rect(Rect2((center / cell).floor() * cell + Vector2(x, y) * cell, Vector2(cell, cell)), color)
