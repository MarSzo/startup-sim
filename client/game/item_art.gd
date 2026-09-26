## Hand-drawn item icons (ink outlines, like the rest of the game), drawn
## with vector calls on any CanvasItem in a 16x16 box.
## Kinds match server/src/inventory.rs.
extends RefCounted

const NONE := 0
const GUEST_PASS := 1
const EMPLOYEE_CARD := 2
const LAPTOP := 3
const COFFEE := 4
const FRUIT := 5

# Shop goods (server/src/shop.rs).
const SANDWICH_CHEESE := 10
const SANDWICH_HAM := 11
const WRAP := 12
const BURGER := 13
const FRIES := 14
const BUN := 15
const BAR := 16
const CHIPS := 17
const WATER := 18
const ENERGY_DRINK := 19
const JUICE := 20
const BEER := 21
const WINE := 22
const CIGARETTES := 23
const UMBRELLA := 24
const DONUT := 25
const COOKIE := 26
const CHEESECAKE := 27
const PIEROGI := 28
const PIZZA := 29
const SUSHI := 30
const SCHNITZEL := 31
const SALAD := 32
const KEBAB := 33
const EMPTY_CUP := 34

const NAMES := {GUEST_PASS: "Przepustka gościa", EMPLOYEE_CARD: "Karta pracownika", LAPTOP: "Laptop", COFFEE: "Kawa", FRUIT: "Owoc",
	SANDWICH_CHEESE: "Kanapka z serem", SANDWICH_HAM: "Kanapka z szynką", WRAP: "Wrap wege", BURGER: "Hamburger",
	FRIES: "Frytki", BUN: "Drożdżówka", BAR: "Batonik", CHIPS: "Chipsy", WATER: "Woda", ENERGY_DRINK: "Energetyk",
	JUICE: "Sok pomarańczowy", BEER: "Piwo", WINE: "Wino", CIGARETTES: "Papierosy", UMBRELLA: "Parasol",
	DONUT: "Pączek", COOKIE: "Ciastko", CHEESECAKE: "Kawałek sernika",
	PIEROGI: "Pierogi ruskie", PIZZA: "Pizza margherita", SUSHI: "Zestaw sushi", SCHNITZEL: "Schabowy z ziemniakami",
	SALAD: "Sałatka z kurczakiem", KEBAB: "Kebab", EMPTY_CUP: "Pusty kubek"}
const SMALL := [GUEST_PASS, EMPLOYEE_CARD, FRUIT, SANDWICH_CHEESE, SANDWICH_HAM, WRAP, BUN, BAR, CHIPS, WATER,
	ENERGY_DRINK, JUICE, BEER, CIGARETTES, UMBRELLA, DONUT, COOKIE, CHEESECAKE]


static func item_name(kind: int) -> String:
	return NAMES.get(kind, "")


const INK := Color("#2a2118")


## Draw `kind` into a 16x16 box at `o` (top-left), scale `s`.
static func draw(c: CanvasItem, kind: int, o: Vector2, s: float) -> void:
	var w := maxf(0.9 * s, 0.35)  # ink width
	# Helpers in box units (0..16).
	var P := func(x: float, y: float) -> Vector2: return o + Vector2(x, y) * s
	var poly := func(pts: Array, fill: Color) -> void:
		var pp := PackedVector2Array()
		for q in pts:
			pp.append(o + (q as Vector2) * s)
		c.draw_colored_polygon(pp, fill)
		pp.append(pp[0])
		c.draw_polyline(pp, INK, w, true)
	var rr := func(x: float, y: float, bw: float, bh: float, fill: Color, rad := 1.2) -> void:
		var k := minf(rad, minf(bw, bh) * 0.45)
		poly.call([Vector2(x + k, y), Vector2(x + bw - k, y), Vector2(x + bw, y + k), Vector2(x + bw, y + bh - k),
			Vector2(x + bw - k, y + bh), Vector2(x + k, y + bh), Vector2(x, y + bh - k), Vector2(x, y + k)], fill)
	var circ := func(x: float, y: float, r: float, fill: Color) -> void:
		c.draw_circle(o + Vector2(x, y) * s, (r + 0.45) * s, INK)
		c.draw_circle(o + Vector2(x, y) * s, r * s, fill)
	var dot := func(x: float, y: float, r: float, col: Color) -> void:
		c.draw_circle(o + Vector2(x, y) * s, r * s, col)
	var ln := func(x0: float, y0: float, x1: float, y1: float, col: Color, lw := 0.8) -> void:
		c.draw_line(o + Vector2(x0, y0) * s, o + Vector2(x1, y1) * s, col, maxf(lw * s, 0.3), true)
	var bottle := func(fill: Color, label: Color, neck_col: Color) -> void:
		poly.call([Vector2(6.8, 1.2), Vector2(9.2, 1.2), Vector2(9.2, 4.2), Vector2(11, 6.5), Vector2(11, 14.8),
			Vector2(5, 14.8), Vector2(5, 6.5), Vector2(6.8, 4.2)], fill)
		rr.call(5.4, 8.5, 5.2, 3.6, label, 0.4)
		rr.call(6.6, 0.4, 2.8, 1.6, neck_col, 0.4)
	var plate := func() -> void:
		c.draw_set_transform(o + Vector2(8, 10) * s, 0.0, Vector2(1.0, 0.62))
		c.draw_circle(Vector2.ZERO, 7.4 * s, INK)
		c.draw_circle(Vector2.ZERO, 7.0 * s, Color("#efe9dc"))
		c.draw_arc(Vector2.ZERO, 5.0 * s, 0, TAU, 20, Color("#d4ccb8"), 0.6 * s, true)
		c.draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
	match kind:
		GUEST_PASS, EMPLOYEE_CARD:
			var guest := kind == GUEST_PASS
			rr.call(1.2, 3.4, 13.6, 10, Color("#f1d15a") if guest else Color("#f2eee4"), 1.4)
			c.draw_rect(Rect2(P.call(1.9, 4.1), Vector2(12.2, 2.2) * s), Color("#d98a3e") if guest else Color("#4f7fb0"))
			if not guest:
				rr.call(3, 7.4, 4, 4.4, Color("#c9a37a"), 0.6)
				dot.call(5, 9, 1.0, Color("#6b4a2e"))
			ln.call(8.2 if not guest else 4, 8.4, 12.6, 8.4, Color("#6b5a48"), 0.7)
			ln.call(8.2 if not guest else 4, 10.4, 11.6, 10.4, Color("#6b5a48"), 0.7)
			rr.call(6.8, 1.0, 2.4, 3.4, Color("#aab0b3"), 0.5)
		LAPTOP:
			poly.call([Vector2(2.6, 2.4), Vector2(13.4, 2.4), Vector2(13.4, 10.6), Vector2(2.6, 10.6)], Color("#4d5359"))
			c.draw_rect(Rect2(P.call(3.6, 3.4), Vector2(8.8, 6.2) * s), Color("#8fb9d3"))
			ln.call(4.4, 4.4, 7.4, 4.4, Color(1, 1, 1, 0.6), 0.7)
			poly.call([Vector2(1.2, 11), Vector2(14.8, 11), Vector2(15.6, 13.8), Vector2(0.4, 13.8)], Color("#9aa1a6"))
		COFFEE, EMPTY_CUP:
			poly.call([Vector2(3.4, 5.6), Vector2(11.6, 5.6), Vector2(10.8, 14.4), Vector2(4.2, 14.4)], Color("#f1ece2"))
			c.draw_arc(P.call(12.2, 9.6), 2.2 * s, -PI / 2, PI / 2, 10, INK, 1.6 * s, true)
			c.draw_arc(P.call(12.2, 9.6), 2.2 * s, -PI / 2, PI / 2, 10, Color("#f1ece2"), 0.8 * s, true)
			if kind == COFFEE:
				c.draw_set_transform(o + Vector2(7.5, 6.3) * s, 0.0, Vector2(1.0, 0.35))
				c.draw_circle(Vector2.ZERO, 3.6 * s, Color("#6b4a2e"))
				c.draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
				for k in 2:
					c.draw_arc(P.call(6 + k * 3, 3.2), 1.0 * s, PI * 0.6, PI * 1.6, 6, Color(INK, 0.5), 0.6 * s, true)
			else:
				ln.call(5.2, 12.6, 9.8, 12.6, Color("#9c7b5b"), 0.8)
		FRUIT:
			circ.call(8, 9.5, 5.4, Color("#c9463a"))
			dot.call(6.2, 7.4, 1.4, Color(1, 1, 1, 0.35))
			ln.call(8, 4.2, 8.6, 2.2, Color("#6b4a2e"), 1.0)
			poly.call([Vector2(8.6, 3.6), Vector2(12, 2.4), Vector2(10.6, 4.6)], Color("#6f9a45"))
		SANDWICH_CHEESE, SANDWICH_HAM:
			var fill := Color("#e8c24e") if kind == SANDWICH_CHEESE else Color("#e59a9a")
			poly.call([Vector2(1.4, 13.4), Vector2(14.6, 13.4), Vector2(8, 3.2)], Color("#e7d3a6"))
			poly.call([Vector2(2.6, 12.2), Vector2(13.4, 12.2), Vector2(8, 4.6)], fill)
			poly.call([Vector2(3.6, 11.2), Vector2(12.4, 11.2), Vector2(8, 5.8)], Color("#f3e6c4"))
			ln.call(3.4, 12.2, 12.6, 12.2, Color("#7da04f"), 1.0)
		WRAP:
			rr.call(1.4, 5, 13.2, 7, Color("#e9d7a4"), 3.2)
			circ.call(13, 8.5, 2.6, Color("#7da04f"))
			dot.call(13, 8.5, 1.2, Color("#c9463a"))
			ln.call(4, 6.5, 10, 6.5, Color("#c9b27a"), 0.7)
		BURGER:
			poly.call([Vector2(2, 7), Vector2(3.5, 3.4), Vector2(8, 2.2), Vector2(12.5, 3.4), Vector2(14, 7)], Color("#d99a4e"))
			rr.call(1.4, 7.2, 13.2, 1.6, Color("#7da04f"), 0.6)
			rr.call(2, 8.8, 12, 2.4, Color("#6b3f25"), 0.8)
			rr.call(2.2, 11.2, 11.6, 2.8, Color("#d99a4e"), 1.2)
			for q in [Vector2(6, 4), Vector2(9, 3.6), Vector2(11, 5)]:
				dot.call(q.x, q.y, 0.4, Color("#f4ead0"))
		FRIES:
			for k in 5:
				rr.call(4 + k * 1.8, 1.4 + (k % 2) * 1.2, 1.4, 7, Color("#f0cf5c"), 0.3)
			poly.call([Vector2(3, 6.4), Vector2(13, 6.4), Vector2(11.8, 14.6), Vector2(4.2, 14.6)], Color("#c9463a"))
			ln.call(5.5, 9.5, 10.5, 9.5, Color("#f4ead0"), 0.8)
		BUN:
			circ.call(8, 9, 6, Color("#d6a45e"))
			c.draw_arc(P.call(8, 9), 3.4 * s, 0.2, TAU - 0.4, 16, Color("#f4ead0"), 1.1 * s, true)
			c.draw_arc(P.call(8, 9), 1.4 * s, 0.4, TAU - 0.6, 10, Color("#f4ead0"), 0.9 * s, true)
		BAR:
			poly.call([Vector2(1.4, 6), Vector2(14.6, 5), Vector2(14.6, 11), Vector2(1.4, 12)], Color("#8e4a2e"))
			rr.call(5, 5.6, 6, 6, Color("#e0b84a"), 0.6)
			ln.call(1.8, 5.6, 1.8, 12, INK, 0.6)
		CHIPS:
			poly.call([Vector2(3, 2.4), Vector2(13, 2.4), Vector2(12.4, 14.4), Vector2(3.6, 14.4)], Color("#d9443a"))
			circ.call(8, 8.6, 3, Color("#f0cf5c"))
			ln.call(3.2, 3.6, 12.8, 3.6, Color("#f4ead0"), 0.6)
		WATER:
			bottle.call(Color("#a9d4e8"), Color("#4f86c0"), Color("#3f6fa8"))
		JUICE:
			poly.call([Vector2(4, 4.6), Vector2(8, 1.4), Vector2(12, 4.6), Vector2(12, 14.6), Vector2(4, 14.6)], Color("#f0a13a"))
			circ.call(8, 9.6, 2.4, Color("#f6c65a"))
		ENERGY_DRINK, BEER:
			if kind == BEER:
				bottle.call(Color("#8a5a2a"), Color("#efe0b0"), Color("#d4b870"))
			else:
				rr.call(4.2, 2.2, 7.6, 12.6, Color("#2c3a4a"), 1.4)
				poly.call([Vector2(8.6, 4), Vector2(6, 9), Vector2(8, 9), Vector2(7, 13), Vector2(10.2, 7.6), Vector2(8.2, 7.6)], Color("#8fd04a"))
		WINE:
			bottle.call(Color("#5a1f2a"), Color("#efe6d2"), Color("#8e2a3a"))
		DONUT:
			circ.call(8, 8.5, 6.2, Color("#d9a15a"))
			c.draw_arc(P.call(8, 8.5), 3.8 * s, 0, TAU, 24, Color("#e889a8"), 3.6 * s, true)
			circ.call(8, 8.5, 1.8, Color("#3a2a20"))
			for q in [Vector2(5.4, 6.2), Vector2(10.4, 7), Vector2(7.2, 12), Vector2(11, 11)]:
				dot.call(q.x, q.y, 0.45, [Color("#f4ead0"), Color("#6fb0d8"), Color("#e0c24a")][int(q.x) % 3])
		COOKIE:
			circ.call(8, 8.5, 6, Color("#c9914e"))
			for q in [Vector2(6, 6.5), Vector2(10, 7), Vector2(7.5, 11), Vector2(11, 10.6), Vector2(5, 10)]:
				dot.call(q.x, q.y, 0.9, Color("#4a2e1e"))
		CHEESECAKE:
			poly.call([Vector2(1.6, 12.4), Vector2(14.4, 12.4), Vector2(14.4, 6.6), Vector2(4, 4.4)], Color("#f3e3b4"))
			poly.call([Vector2(1.6, 12.4), Vector2(14.4, 12.4), Vector2(14.4, 14.4), Vector2(1.6, 14.4)], Color("#b88a4e"))
			ln.call(4.2, 4.6, 14.2, 6.8, Color("#c9463a"), 1.2)
		PIEROGI:
			plate.call()
			for q in [Vector2(5, 9.2), Vector2(8.2, 10.8), Vector2(11, 9.2)]:
				c.draw_arc(o + q * s, 2.6 * s, PI, TAU, 10, INK, 1.8 * s, true)
				c.draw_arc(o + q * s, 2.6 * s, PI, TAU, 10, Color("#f0e0b6"), 1.0 * s, true)
				ln.call(q.x - 2.6, q.y, q.x + 2.6, q.y, INK, 0.6)
		PIZZA:
			circ.call(8, 8.5, 6.6, Color("#d9a15a"))
			dot.call(8, 8.5, 5.4, Color("#d9553f"))
			for q in [Vector2(6, 6.6), Vector2(10.4, 7.4), Vector2(7, 11), Vector2(10.6, 11)]:
				circ.call(q.x, q.y, 1.0, Color("#9a2f25"))
			for q in [Vector2(8.4, 5.2), Vector2(4.8, 9.2), Vector2(9, 9.4)]:
				dot.call(q.x, q.y, 0.6, Color("#f4ead0"))
		SUSHI:
			rr.call(1.2, 10.4, 13.6, 3.2, Color("#b88a4e"), 0.6)
			for k in 3:
				circ.call(3.8 + k * 4.2, 8, 1.9, Color("#262a2e"))
				dot.call(3.8 + k * 4.2, 8, 1.3, Color("#f4efe4"))
				dot.call(3.8 + k * 4.2, 8, 0.6, Color("#e07a5a"))
		SCHNITZEL:
			plate.call()
			poly.call([Vector2(3, 8), Vector2(9.4, 7), Vector2(10.4, 11.2), Vector2(4, 12)], Color("#d49a4a"))
			circ.call(12, 9.2, 1.4, Color("#f0dc9a"))
			circ.call(11.4, 11.6, 1.4, Color("#f0dc9a"))
		SALAD:
			poly.call([Vector2(1.6, 8), Vector2(14.4, 8), Vector2(12, 14.2), Vector2(4, 14.2)], Color("#e9e4d6"))
			for q in [Vector2(4.4, 7.2), Vector2(7.6, 6.2), Vector2(10.8, 7), Vector2(6, 8), Vector2(9.4, 8)]:
				circ.call(q.x, q.y, 1.7, Color("#7da04f"))
			dot.call(8.2, 7.2, 0.9, Color("#c9463a"))
		KEBAB:
			poly.call([Vector2(3, 3.4), Vector2(13, 3.4), Vector2(11.2, 14.6), Vector2(4.8, 14.6)], Color("#e9d7a4"))
			rr.call(4, 3.8, 8, 3.6, Color("#9a5a32"), 1.0)
			dot.call(6, 4.4, 0.9, Color("#7da04f"))
			dot.call(9.6, 4.6, 0.9, Color("#c9463a"))
		UMBRELLA:
			var pts := [Vector2(1.4, 8.6)]
			for k in 9:
				var a := PI + k * PI / 8.0
				pts.append(Vector2(8 + cos(a) * 6.6, 8.6 + sin(a) * 6.6))
			pts.append(Vector2(14.6, 8.6))
			poly.call(pts, Color("#3a5f9e"))
			ln.call(8, 8.6, 8, 14, INK, 1.2)
			c.draw_arc(P.call(9.2, 14), 1.2 * s, 0, PI, 6, INK, 1.2 * s, true)
		CIGARETTES:
			rr.call(3.6, 3, 8.8, 11.6, Color("#f1ece2"), 0.8)
			c.draw_rect(Rect2(P.call(4.2, 3.6), Vector2(7.6, 3.4) * s), Color("#c9463a"))
			for k in 3:
				rr.call(4.6 + k * 2.4, 1.2, 1.8, 2.6, Color("#e0b86a"), 0.3)
		_:
			rr.call(3, 3, 10, 10, Color("#cfc6b2"), 1.4)
