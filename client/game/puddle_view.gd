## A puddle left by a toilet accident (entity kind PUDDLE): neon yellow -
## or, thrown up after drinking (held = 1), a lumpy greenish-beige one -
## flat on the floor until the cleaner mops it up or the office closes. The blob's shape comes from
## the entity id, so every puddle looks a little different.
extends Node2D

const GLOW := Color(1.0, 0.97, 0.2, 0.3)
const FILL := Color("#fff700")
const SHINE := Color(1.0, 1.0, 0.85, 0.85)
const VOMIT_GLOW := Color(0.6, 0.7, 0.2, 0.25)
const VOMIT_FILL := Color("#b9b44e")
const VOMIT_LUMP := Color("#8a7a3a")

var last_seen_tick := 0
var vomit := false
var _blobs: Array = []  # [offset, radius]
var _lumps: Array = []  # vomit: [offset, radius]


func setup(id: int, p_vomit := false) -> void:
	vomit = p_vomit
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


func _draw() -> void:
	# Flattened: a puddle lies on the floor.
	draw_set_transform(Vector2.ZERO, 0.0, Vector2(1.0, 0.55))
	for b in _blobs:
		draw_circle(b[0], b[1] + 2.5, VOMIT_GLOW if vomit else GLOW)
	for b in _blobs:
		draw_circle(b[0], b[1], VOMIT_FILL if vomit else FILL)
	if vomit:
		for l in _lumps:
			draw_circle(l[0], l[1], VOMIT_LUMP)
	draw_circle(Vector2(-2.5, -2.0), 1.6, Color(SHINE, 0.5) if vomit else SHINE)
	draw_set_transform(Vector2.ZERO)
