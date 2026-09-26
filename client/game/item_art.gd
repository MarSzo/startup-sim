## Pixel-art item icons, drawn with rectangles on any CanvasItem.
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


## Draw `kind` into a box of `size` pixels at `o` (top-left), scale `s`.
static func draw(c: CanvasItem, kind: int, o: Vector2, s: float) -> void:
	var r := func(x: float, y: float, w: float, h: float, col: Color) -> void:
		c.draw_rect(Rect2(o + Vector2(x, y) * s, Vector2(w, h) * s), col)
	match kind:
		GUEST_PASS:
			r.call(1, 3, 14, 10, Color("#1c1c24"))
			r.call(2, 4, 12, 8, Color("#f1c40f"))
			r.call(2, 4, 12, 2, Color("#e67e22"))
			r.call(4, 7, 8, 1, Color("#7a5a10"))
			r.call(4, 9, 6, 1, Color("#7a5a10"))
			r.call(7, 1, 2, 3, Color("#9aa4ab"))  # clip
		EMPLOYEE_CARD:
			r.call(1, 3, 14, 10, Color("#1c1c24"))
			r.call(2, 4, 12, 8, Color("#f4f6f8"))
			r.call(2, 4, 12, 2, Color("#2e6bd9"))
			r.call(3, 7, 4, 4, Color("#c9a37a"))  # photo
			r.call(4, 7, 2, 1, Color("#3b2a1e"))
			r.call(8, 7, 5, 1, Color("#4a5566"))
			r.call(8, 9, 4, 1, Color("#4a5566"))
			r.call(7, 1, 2, 3, Color("#9aa4ab"))
		LAPTOP:
			r.call(1, 3, 14, 9, Color("#1c1c24"))
			r.call(2, 4, 12, 7, Color("#5c6570"))
			r.call(3, 5, 10, 5, Color("#7fb2d8"))
			r.call(0, 12, 16, 3, Color("#8a939c"))
			r.call(0, 12, 16, 1, Color("#b4bcc3"))
		COFFEE:
			r.call(3, 5, 9, 9, Color("#1c1c24"))
			r.call(4, 6, 7, 7, Color("#f4f1ea"))
			r.call(4, 6, 7, 2, Color("#6b4a2e"))
			r.call(11, 8, 3, 3, Color("#f4f1ea"))
			r.call(12, 9, 1, 1, Color("#1c1c24"))
			r.call(6, 2, 1, 2, Color(1, 1, 1, 0.6))
			r.call(8, 1, 1, 3, Color(1, 1, 1, 0.5))
		EMPTY_CUP:  # a mug with a brown ring at the bottom
			r.call(3, 5, 9, 9, Color("#1c1c24"))
			r.call(4, 6, 7, 7, Color("#f4f1ea"))
			r.call(4, 6, 7, 1, Color("#d8d2c4"))
			r.call(5, 11, 5, 1, Color("#9c7b5b"))
			r.call(11, 8, 3, 3, Color("#f4f1ea"))
			r.call(12, 9, 1, 1, Color("#1c1c24"))
		FRUIT:
			r.call(3, 5, 10, 9, Color("#1c1c24"))
			r.call(4, 6, 8, 7, Color("#e74c3c"))
			r.call(5, 7, 2, 2, Color("#f5a09a"))
			r.call(7, 2, 1, 4, Color("#5d4037"))
			r.call(8, 3, 3, 2, Color("#27ae60"))
		SANDWICH_CHEESE, SANDWICH_HAM:
			# Triangle sandwich in a wrapper.
			r.call(2, 4, 12, 9, Color("#1c1c24"))
			r.call(3, 5, 10, 7, Color("#e9d7a8"))
			r.call(3, 8, 10, 2, Color("#f1c40f") if kind == SANDWICH_CHEESE else Color("#e8a0a0"))
			r.call(3, 7, 10, 1, Color("#7fbf5a"))
			r.call(3, 5, 10, 1, Color(1, 1, 1, 0.5))
		WRAP:
			r.call(2, 5, 12, 7, Color("#1c1c24"))
			r.call(3, 6, 10, 5, Color("#e8d3a0"))
			r.call(11, 6, 2, 5, Color("#7fbf5a"))
			r.call(3, 8, 8, 1, Color("#c9b27a"))
		BURGER:
			r.call(2, 3, 12, 11, Color("#1c1c24"))
			r.call(3, 4, 10, 3, Color("#d99a4a"))
			r.call(3, 7, 10, 1, Color("#7fbf5a"))
			r.call(3, 8, 10, 2, Color("#6b3a1f"))
			r.call(3, 10, 10, 1, Color("#f1c40f"))
			r.call(3, 11, 10, 2, Color("#d99a4a"))
			r.call(5, 5, 1, 1, Color("#f4e1b8"))
			r.call(9, 4, 1, 1, Color("#f4e1b8"))
		FRIES:
			r.call(3, 7, 10, 7, Color("#1c1c24"))
			r.call(4, 8, 8, 5, Color("#d63031"))
			for i in 4:
				r.call(4 + i * 2, 3 + (i % 2), 1, 5, Color("#f7c948"))
		BUN:
			r.call(2, 5, 12, 8, Color("#1c1c24"))
			r.call(3, 6, 10, 6, Color("#d9a35a"))
			r.call(6, 8, 4, 2, Color("#f5e3a3"))
		BAR:
			r.call(1, 6, 14, 5, Color("#1c1c24"))
			r.call(2, 7, 12, 3, Color("#8e44ad"))
			r.call(5, 7, 5, 3, Color("#f4f1ea"))
		CHIPS:
			r.call(3, 2, 10, 12, Color("#1c1c24"))
			r.call(4, 3, 8, 10, Color("#e67e22"))
			r.call(5, 6, 6, 3, Color("#f7d774"))
			r.call(4, 3, 8, 1, Color("#c0392b"))
		WATER, JUICE:
			r.call(5, 1, 6, 14, Color("#1c1c24"))
			r.call(6, 4, 4, 10, Color("#bfe3f5") if kind == WATER else Color("#f39c12"))
			r.call(7, 2, 2, 2, Color("#2e86de"))
			r.call(6, 7, 4, 2, Color("#2e86de") if kind == WATER else Color("#27ae60"))
		ENERGY_DRINK, BEER:
			r.call(4, 2, 8, 12, Color("#1c1c24"))
			r.call(5, 3, 6, 10, Color("#27ae60") if kind == ENERGY_DRINK else Color("#d4a017"))
			r.call(5, 3, 6, 1, Color("#b4bcc3"))
			r.call(6, 6, 4, 4, Color("#101010") if kind == ENERGY_DRINK else Color("#f4f1ea"))
			r.call(7, 7, 2, 2, Color("#2ecc71") if kind == ENERGY_DRINK else Color("#c0392b"))
		WINE:
			r.call(6, 0, 4, 16, Color("#1c1c24"))
			r.call(7, 1, 2, 4, Color("#2c3e50"))
			r.call(6, 5, 4, 10, Color("#6d1a36"))
			r.call(7, 8, 2, 3, Color("#f4f1ea"))
		DONUT:
			r.call(2, 3, 12, 11, Color("#1c1c24"))
			r.call(3, 4, 10, 9, Color("#d9934a"))
			r.call(4, 4, 8, 4, Color("#f06292"))
			r.call(6, 7, 4, 3, Color("#8a5a2b"))
			r.call(5, 5, 1, 1, Color("#fff176"))
			r.call(9, 5, 1, 1, Color("#81d4fa"))
		COOKIE:
			r.call(3, 4, 10, 10, Color("#1c1c24"))
			r.call(4, 5, 8, 8, Color("#d9a35a"))
			for chip in [[5, 6], [9, 7], [6, 10], [10, 10]]:
				r.call(chip[0], chip[1], 2, 2, Color("#4a2c17"))
		CHEESECAKE:
			r.call(2, 5, 12, 9, Color("#1c1c24"))
			r.call(3, 6, 10, 7, Color("#f4e3b5"))
			r.call(3, 11, 10, 2, Color("#b5824a"))
			r.call(3, 6, 10, 1, Color("#e8c77a"))
		PIZZA:
			# Pizza box.
			r.call(1, 3, 14, 11, Color("#1c1c24"))
			r.call(2, 4, 12, 9, Color("#e8d3a0"))
			r.call(4, 6, 8, 5, Color("#c0392b"))
			r.call(5, 7, 2, 1, Color("#f7f1e3"))
			r.call(9, 8, 2, 1, Color("#f7f1e3"))
		SUSHI:
			r.call(1, 5, 14, 8, Color("#1c1c24"))
			r.call(2, 6, 12, 6, Color("#2c2f36"))
			for sx in [3, 7, 11]:
				r.call(sx, 7, 3, 3, Color("#f4f1ea"))
				r.call(sx + 1, 8, 1, 1, Color("#e67e22"))
		PIEROGI, SCHNITZEL, SALAD, KEBAB:
			# Takeaway box with a coloured sticker.
			var sticker: Color = {PIEROGI: Color("#f1c40f"), SCHNITZEL: Color("#c0392b"), SALAD: Color("#27ae60"), KEBAB: Color("#e67e22")}[kind]
			r.call(2, 4, 12, 10, Color("#1c1c24"))
			r.call(3, 5, 10, 8, Color("#d9d2c4"))
			r.call(3, 5, 10, 2, Color("#b8ad99"))
			r.call(6, 8, 4, 3, sticker)
		UMBRELLA:
			# Folded umbrella: canopy strap, shaft, J handle.
			r.call(6, 1, 4, 10, Color("#1c1c24"))
			r.call(7, 2, 2, 8, Color("#2e6bd9"))
			r.call(7, 5, 2, 1, Color("#8fb7ff"))
			r.call(7, 10, 2, 3, Color("#5c6570"))
			r.call(5, 12, 4, 2, Color("#6b4a2e"))
			r.call(5, 11, 1, 2, Color("#6b4a2e"))
		CIGARETTES:
			r.call(3, 3, 10, 11, Color("#1c1c24"))
			r.call(4, 4, 8, 9, Color("#ecf0f1"))
			r.call(4, 4, 8, 3, Color("#c0392b"))
			r.call(5, 2, 1, 3, Color("#f4f1ea"))
			r.call(7, 2, 1, 3, Color("#f4f1ea"))
			r.call(5, 2, 1, 1, Color("#e67e22"))
