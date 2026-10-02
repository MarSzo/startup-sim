## A puddle left by a toilet accident (entity kind PUDDLE): neon yellow,
## flat on the floor until the cleaner mops it up or the office closes. The blob's shape comes from
## the entity id, so every puddle looks a little different.
extends Node2D

const GLOW := Color(1.0, 0.97, 0.2, 0.3)
const FILL := Color("#fff700")
const SHINE := Color(1.0, 1.0, 0.85, 0.85)

var last_seen_tick := 0
var _blobs: Array = []  # [offset, radius]


func setup(id: int) -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = id
	_blobs = [[Vector2.ZERO, 6.0]]
	for i in 3:
		var a := TAU * i / 3.0 + rng.randf_range(-0.6, 0.6)
		_blobs.append([Vector2(cos(a), sin(a)) * rng.randf_range(3.0, 5.0), rng.randf_range(3.0, 4.5)])
	queue_redraw()


func _draw() -> void:
	# Flattened: a puddle lies on the floor.
	draw_set_transform(Vector2.ZERO, 0.0, Vector2(1.0, 0.55))
	for b in _blobs:
		draw_circle(b[0], b[1] + 2.5, GLOW)
	for b in _blobs:
		draw_circle(b[0], b[1], FILL)
	draw_circle(Vector2(-2.5, -2.0), 1.6, SHINE)
	draw_set_transform(Vector2.ZERO)
