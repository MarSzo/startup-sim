## A stab: red drops fly out and fall for a moment (client-only; the stain
## left on the floor is a puddle from the server).
extends Node2D

const LIFE := 0.6
const RED := Color("#a3161a")

var _drops: Array = []  # [position, velocity, radius]
var _t := 0.0


func _ready() -> void:
	var rng := RandomNumberGenerator.new()
	rng.randomize()
	for i in 14:
		var a := rng.randf_range(-PI, 0.0)  # up and sideways
		_drops.append([Vector2(0, -8), Vector2(cos(a), sin(a)) * rng.randf_range(25.0, 70.0), rng.randf_range(0.6, 1.5)])


func _process(delta: float) -> void:
	_t += delta
	if _t >= LIFE:
		queue_free()
		return
	for d in _drops:
		d[1].y += 160.0 * delta  # falling
		d[0] += d[1] * delta
	queue_redraw()


func _draw() -> void:
	var fade := 1.0 - _t / LIFE
	for d in _drops:
		draw_circle(d[0], d[2], Color(RED, fade))
