## Pixel-art item icons, drawn with rectangles on any CanvasItem.
## Kinds match server/src/inventory.rs.
extends RefCounted

const NONE := 0
const GUEST_PASS := 1
const EMPLOYEE_CARD := 2
const LAPTOP := 3
const COFFEE := 4

const NAMES := {GUEST_PASS: "Przepustka gościa", EMPLOYEE_CARD: "Karta pracownika", LAPTOP: "Laptop", COFFEE: "Kawa"}
const SMALL := [GUEST_PASS, EMPLOYEE_CARD]


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
