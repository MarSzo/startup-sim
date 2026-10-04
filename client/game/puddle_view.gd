## A puddle left by a toilet accident (entity kind PUDDLE): neon yellow -
## or, thrown up (held = 1), a lumpy greenish-beige one; or a brown pile
## with flies (held = 2); blood after a stab (held = 3) - on the floor until
## the cleaner mops it up or the office closes. The blob's shape comes from
## the entity id, so every puddle looks a little different.
extends Node2D

const GLOW := Color(1.0, 0.97, 0.2, 0.3)
const FILL := Color("#fff700")
const SHINE := Color(1.0, 1.0, 0.85, 0.85)
const VOMIT_GLOW := Color(0.6, 0.7, 0.2, 0.25)
const VOMIT_FILL := Color("#b9b44e")
const VOMIT_LUMP := Color("#8a7a3a")
const BLOOD_GLOW := Color(0.45, 0.0, 0.0, 0.25)
const BLOOD_FILL := Color("#8e1216")
const POOP := Color("#6b4423")
const POOP_DARK := Color("#4a2e17")
const INK := Color("#1d1712")

var last_seen_tick := 0
var vomit := false
var poop := false
var blood := false
var _blobs: Array = []  # [offset, radius]
var _lumps: Array = []  # vomit: [offset, radius]


func setup(id: int, kind := 0) -> void:
	vomit = kind == 1
	poop = kind == 2
	blood = kind == 3
	set_process(poop)  # the flies
	var rng := RandomNumberGenerator.new()
	rng.seed = id
	_blobs = [[Vector2.ZERO, 6.0]]
	for i in 3:
		var a := TAU * i / 3.0 + rng.randf_range(-0.6, 0.6)
		_blobs.append([Vector2(cos(a), sin(a)) * rng.randf_range(3.0, 5.0), rng.randf_range(3.0, 4.5)])
	_lumps = []
	for i in 7:
		_lumps.append([Vector2(rng.randf_range(-6, 6), rng.randf_range(-5, 5)), rng.randf_range(0.6, 1.3)])
	queue_redraw()


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	if poop:
		_draw_pile()
		return
	# Flattened: a puddle lies on the floor.
	draw_set_transform(Vector2.ZERO, 0.0, Vector2(1.0, 0.55))
	var glow := BLOOD_GLOW if blood else (VOMIT_GLOW if vomit else GLOW)
	var fill := BLOOD_FILL if blood else (VOMIT_FILL if vomit else FILL)
	var scale := 0.6 if blood else 1.0  # a stain, not a lake
	for b in _blobs:
		draw_circle(b[0] * scale, b[1] * scale + 2.5, glow)
	for b in _blobs:
		draw_circle(b[0] * scale, b[1] * scale, fill)
	if vomit:
		for l in _lumps:
			draw_circle(l[0], l[1], VOMIT_LUMP)
	draw_circle(Vector2(-2.5, -2.0) * scale, 1.6 * scale, Color(SHINE, 0.3 if blood else (0.5 if vomit else 1.0)))
	draw_set_transform(Vector2.ZERO)


## A swirl of three blobs, inked, a shine, and two flies circling.
func _draw_pile() -> void:
	draw_set_transform(Vector2(0, 1), 0.0, Vector2(1.0, 0.45))
	draw_circle(Vector2.ZERO, 7.5, Color(0, 0, 0, 0.25))
	draw_set_transform(Vector2.ZERO)
	for b in [[Vector2(0, 0), 5.2], [Vector2(0, -3.2), 3.8], [Vector2(0.4, -5.8), 2.4]]:
		draw_circle(b[0], b[1] + 0.7, INK)
	for b in [[Vector2(0, 0), 5.2], [Vector2(0, -3.2), 3.8], [Vector2(0.4, -5.8), 2.4]]:
		draw_circle(b[0], b[1], POOP)
		draw_arc(b[0] + Vector2(0, 0.6), b[1] - 0.6, 0.2, PI - 0.2, 8, POOP_DARK, 0.6, true)
	draw_circle(Vector2(-1.6, -4.0), 0.8, Color(1, 1, 1, 0.5))
	var t := Time.get_ticks_msec() / 1000.0
	for i in 2:
		var a := t * (3.0 + i) + i * PI
		var f := Vector2(cos(a) * 6.0, -8.0 + sin(a * 1.7) * 2.5)
		draw_circle(f, 0.7, INK)
		draw_line(f + Vector2(-0.9, -0.6), f + Vector2(0.9, -0.6), Color(1, 1, 1, 0.6), 0.5)
