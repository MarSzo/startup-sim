## A TV on the wall (3 tiles wide): what's on is drawn here, in the game's
## ink style, from the channel and how long it's been on - the same moment
## for everybody (the server says when it started). Channels: 1 cartoons,
## 2 news, 3 weather, 4 a football match, 5 nature (an aquarium).
extends Node2D

const INK := Color("#1d1712")
const W := 44.0
const H := 26.0
const NEWS := ["PILNE: kałuża w holu", "Prezes: owocowe czwartki codziennie", "Tramwaj spóźniony (znowu)",
	"Pani Wiesia pyta o ślub", "Kasa: jaka parówka jest?", "Dzik w parku miejskim"]

var channel := 0
var started_at := 0.0   # seconds of game time since the channel came on (at the last update)
var weather := ""       # for the weather channel
var _t := 0.0


## `elapsed`: seconds since the channel came on.
func show_channel(ch: int, elapsed: float) -> void:
	if ch != channel or absf(elapsed - _t) > 1.5:
		_t = elapsed
	channel = ch
	set_process(channel != 0)
	queue_redraw()


func _process(delta: float) -> void:
	_t += delta
	queue_redraw()


func _draw() -> void:
	# The set: a black frame on the wall, the screen in it (bottom edge on the wall line).
	var r := Rect2(Vector2(2, -H + 6), Vector2(W, H))
	draw_rect(r.grow(2.0), INK)
	draw_rect(r.grow(1.2), Color("#2b2b33"))
	if channel == 0:
		draw_rect(r, Color("#15161c"))
		draw_line(r.position + Vector2(6, 4), r.position + Vector2(16, 14), Color(1, 1, 1, 0.08), 3.0)
		return
	match channel:
		1: _cartoon(r)
		2: _news(r)
		3: _weather(r)
		4: _match(r)
		5: _aquarium(r)
	# Glass and a little red "on" light.
	draw_line(r.position + Vector2(4, 3), r.position + Vector2(12, 11), Color(1, 1, 1, 0.07), 2.0)
	draw_circle(Vector2(r.end.x - 2, r.end.y + 1.2), 0.6, Color("#e74c3c"))


func _cartoon(r: Rect2) -> void:
	draw_rect(r, Color("#8fd0f0"))
	draw_circle(r.position + Vector2(36, 6), 3.5, Color("#ffd23f"))
	var hill := PackedVector2Array([Vector2(r.position.x, r.end.y), Vector2(r.position.x, r.end.y - 6)])
	for k in 12:
		hill.append(Vector2(r.position.x + k * W / 11.0, r.end.y - 6 - sin(k * 0.8) * 2.0))
	hill.append(Vector2(r.end.x, r.end.y))
	draw_colored_polygon(hill, Color("#6fbf4a"))
	# A blob that hops across, and back.
	var x := fposmod(_t * 9.0, (W - 8) * 2.0)
	x = x if x < W - 8 else (W - 8) * 2.0 - x
	var hop := absf(sin(_t * 5.0)) * 7.0
	var c := r.position + Vector2(4 + x, H - 10 - hop)
	draw_circle(c, 3.6, INK)
	draw_circle(c, 3.0, Color("#e84393"))
	draw_circle(c + Vector2(-1, -0.8), 0.8, Color.WHITE)
	draw_circle(c + Vector2(1.2, -0.8), 0.8, Color.WHITE)
	draw_circle(c + Vector2(-0.8, -0.8), 0.35, INK)
	draw_circle(c + Vector2(1.4, -0.8), 0.35, INK)


func _news(r: Rect2) -> void:
	draw_rect(r, Color("#1f3a68"))
	# The anchor at the desk.
	var a := r.position + Vector2(14, 14)
	draw_circle(a, 4.0, Color("#f2cfae"))
	draw_rect(Rect2(a + Vector2(-5, 4), Vector2(10, 6)), Color("#2c3e50"))
	draw_rect(Rect2(r.position + Vector2(4, 18), Vector2(26, 3)), Color("#8a6a45"))
	draw_rect(Rect2(r.position + Vector2(28, 4), Vector2(13, 9)), Color("#c0392b"))
	_text(r.position + Vector2(29, 11), "NA ŻYWO", 3, Color.WHITE)
	# The ticker at the bottom.
	draw_rect(Rect2(r.position + Vector2(0, H - 5), Vector2(W, 5)), Color("#f1c40f"))
	# A window over the headlines, moving a letter at a time (stays on the screen).
	var text := "   ·   ".join(NEWS) + "   ·   "
	var at := int(_t * 4.0) % text.length()
	var shown := (text + text).substr(at, 18)
	_text(Vector2(r.position.x + 1, r.end.y - 1.2), shown, 4, INK)


func _weather(r: Rect2) -> void:
	draw_rect(r, Color("#5b8fc2"))
	var land := PackedVector2Array([r.position + Vector2(8, 6), r.position + Vector2(24, 4), r.position + Vector2(32, 10),
		r.position + Vector2(28, 21), r.position + Vector2(12, 22), r.position + Vector2(6, 14)])
	draw_colored_polygon(land, Color("#7aa65a"))
	var c := r.position + Vector2(20, 12)
	match weather:
		"deszcz", "burza":
			draw_circle(c, 4, Color("#d0d5dc"))
			draw_circle(c + Vector2(4, 1), 3, Color("#d0d5dc"))
			for k in 3:
				var y := fposmod(_t * 12.0 + k * 3.0, 6.0)
				draw_line(c + Vector2(-2 + k * 2.5, 4 + y), c + Vector2(-3 + k * 2.5, 6 + y), Color("#2e86de"), 0.8)
			if weather == "burza" and fposmod(_t, 3.0) < 0.15:
				draw_rect(r, Color(1, 1, 1, 0.5))
		"pochmurno", "mgła":
			draw_circle(c, 4, Color("#e6e6e6"))
			draw_circle(c + Vector2(4, 1), 3, Color("#e6e6e6"))
		_:
			draw_circle(c, 4, Color("#ffd23f"))
			for k in 8:
				var ang := k * TAU / 8 + _t * 0.5
				draw_line(c + Vector2(cos(ang), sin(ang)) * 5, c + Vector2(cos(ang), sin(ang)) * 7, Color("#ffd23f"), 0.8)
	_text(r.position + Vector2(33, 8), "21°", 5, Color.WHITE)


func _match(r: Rect2) -> void:
	draw_rect(r, Color("#3e8e41"))
	for k in 6:
		if k % 2 == 0:
			draw_rect(Rect2(r.position + Vector2(k * W / 6.0, 0), Vector2(W / 6.0, H)), Color(1, 1, 1, 0.05))
	draw_line(r.position + Vector2(W / 2, 0), r.position + Vector2(W / 2, H), Color(1, 1, 1, 0.6), 0.5)
	draw_arc(r.position + Vector2(W / 2, H / 2), 4, 0, TAU, 16, Color(1, 1, 1, 0.6), 0.5)
	for side in [0.0, W - 2]:
		draw_rect(Rect2(r.position + Vector2(side, H / 2 - 4), Vector2(2, 8)), Color(1, 1, 1, 0.7))
	var ball := r.position + Vector2(W / 2 + sin(_t * 0.9) * 18.0, H / 2 + sin(_t * 1.7) * 8.0)
	for i in 4:
		var red := i < 2
		var p := ball + Vector2((-6 if red else 6) + sin(_t * (1.3 + i)) * 3.0, (i % 2 * 2 - 1) * 4.0)
		draw_circle(p, 1.3, Color("#e74c3c") if red else Color("#f4f6f8"))
	draw_circle(ball, 0.9, Color.WHITE)
	# The score: a goal every 40 s or so.
	var goals := int(_t / 40.0)
	draw_rect(Rect2(r.position + Vector2(1, 1), Vector2(15, 5)), Color(0, 0, 0, 0.6))
	_text(r.position + Vector2(2, 5.2), "%d:%d" % [(goals + 1) / 2, goals / 2], 4, Color.WHITE)


func _aquarium(r: Rect2) -> void:
	draw_rect(r, Color("#1b6f8f"))
	draw_rect(Rect2(r.position + Vector2(0, H - 4), Vector2(W, 4)), Color("#d9c38a"))
	for k in 3:
		var x := r.position.x + 6 + k * 14 + sin(_t + k) * 1.5
		var weed := PackedVector2Array()
		for j in 5:
			weed.append(Vector2(x + sin(_t * 1.5 + j + k) * 1.2, r.end.y - 3 - j * 2.4))
		draw_polyline(weed, Color("#3fa34d"), 1.0)
	for k in 3:
		var dir := 1.0 if k % 2 == 0 else -1.0
		var fx := fposmod(_t * (5.0 + k * 2.0) * dir + k * 15.0, W + 8) - 4
		var f := r.position + Vector2(fx, 6 + k * 6 + sin(_t * 2 + k) * 1.5)
		var col: Color = [Color("#f39c12"), Color("#e84393"), Color("#f1c40f")][k]
		draw_circle(f, 1.8, col)
		draw_colored_polygon(PackedVector2Array([f - Vector2(dir * 1.5, 0), f - Vector2(dir * 3.5, 1.5), f - Vector2(dir * 3.5, -1.5)]), col)
	for k in 4:
		var by := fposmod(-_t * 6.0 + k * 7.0, H - 4)
		draw_arc(r.position + Vector2(10 + k * 9, by), 0.7, 0, TAU, 8, Color(1, 1, 1, 0.5), 0.3)


## Small text on the screen, sharp: drawn 4x larger and scaled down (a 4 px
## font would be rasterized blurry).
func _text(pos: Vector2, text: String, size: int, color: Color) -> void:
	draw_set_transform(pos, 0.0, Vector2(0.25, 0.25))
	draw_string(ThemeDB.fallback_font, Vector2.ZERO, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size * 4, color)
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
