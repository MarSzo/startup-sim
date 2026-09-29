## Weather on screen (between the world and the HUD): rain streaks, a storm
## with lightning, fog. Full effects only outdoors; indoors you only notice
## lightning through the windows.
extends Control

const Protocol = preload("res://net/protocol.gd")

signal lightning

var weather := Protocol.WEATHER_SUNNY
var outdoors := false
var _drops: Array = []     # [x, y, speed, length] in screen space
var _flash := 0.0          # lightning flash alpha
var _next_flash := 4.0
var _t := 0.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	get_viewport().size_changed.connect(_fit)
	_fit()


func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size


var _drawn := []  # weather, outdoors, size of the last picture


func set_state(p_weather: int, p_outdoors: bool) -> void:
	weather = p_weather
	outdoors = p_outdoors


func _rain_count() -> int:
	if not outdoors:
		return 0
	match weather:
		Protocol.WEATHER_RAIN:
			return 160
		Protocol.WEATHER_STORM:
			return 320
	return 0


func _process(delta: float) -> void:
	_t += delta
	var want := _rain_count()
	while _drops.size() < want:
		_drops.append([randf() * size.x, randf() * size.y, randf_range(700, 1000), randf_range(10, 18)])
	while _drops.size() > want:
		_drops.pop_back()
	for d in _drops:
		d[1] += d[2] * delta
		d[0] -= d[2] * 0.18 * delta
		if d[1] > size.y:
			d[1] = -d[3]
			d[0] = randf() * (size.x + 100)
	if weather == Protocol.WEATHER_STORM:
		_next_flash -= delta
		if _next_flash <= 0.0:
			_flash = 0.85
			lightning.emit()
			_next_flash = randf_range(3.0, 9.0)
	var was_flash := _flash > 0.0
	_flash = maxf(0.0, _flash - delta * 2.5)
	# Nothing moving (sun, clouds, indoors): the last picture stays.
	var busy := not _drops.is_empty() or _flash > 0.0 or was_flash or (outdoors and weather == Protocol.WEATHER_FOG)
	if busy or _drawn != [weather, outdoors, size]:
		_drawn = [weather, outdoors, size]
		queue_redraw()


func _draw() -> void:
	var r := Rect2(Vector2.ZERO, size)
	var ink := Color(0.12, 0.09, 0.07)
	if outdoors and weather == Protocol.WEATHER_FOG:
		# Fog banks: big soft clouds with a faint ink rim, drifting.
		draw_rect(r, Color(0.8, 0.78, 0.74, 0.3))
		for pass_i in 2:
			for i in 9:
				var x := fmod(_t * (10.0 + i * 2.5) + i * 190.0, size.x + 500.0) - 250.0
				var y := size.y * (0.08 + (i % 5) * 0.2) + sin(_t * 0.3 + i) * 12.0
				var rad := 90.0 + (i % 3) * 35.0
				if pass_i == 0:
					draw_circle(Vector2(x, y), rad + 3.0, Color(ink, 0.08))
				else:
					draw_circle(Vector2(x, y), rad, Color(0.88, 0.86, 0.82, 0.16))
	if outdoors and weather == Protocol.WEATHER_STORM:
		draw_rect(r, Color(0.05, 0.06, 0.12, 0.25))  # a dark sky
	# Rain: slanted inked strokes with a light core; splashes on the ground.
	for d in _drops:
		var a := Vector2(d[0], d[1])
		var b := a + Vector2(d[3] * 0.25, d[3] * 1.4)
		draw_line(a, b, Color(ink, 0.55), 3.0, true)
		draw_line(a, b, Color(0.78, 0.84, 0.92, 0.75), 1.4, true)
		if int(d[0] * 13.0) % 7 == 0 and d[1] > size.y * 0.3:
			var k := fmod(_t * 3.0 + d[0], 1.0)
			draw_arc(Vector2(d[0] * 0.97, fmod(d[0] * 3.7, size.y)), 3.0 + k * 5.0, PI, TAU, 8, Color(0.8, 0.86, 0.95, 0.5 * (1.0 - k)), 1.2, true)
	if _flash > 0.0:
		draw_rect(r, Color(1, 0.98, 0.9, _flash * (0.8 if outdoors else 0.25)))
