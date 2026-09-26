## Weather on screen (between the world and the HUD): rain streaks, a storm
## with lightning, fog. Full effects only outdoors; indoors you only notice
## lightning through the windows.
extends Control

const Protocol = preload("res://net/protocol.gd")

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
			_next_flash = randf_range(3.0, 9.0)
	_flash = maxf(0.0, _flash - delta * 2.5)
	queue_redraw()


func _draw() -> void:
	var r := Rect2(Vector2.ZERO, size)
	if outdoors and weather == Protocol.WEATHER_FOG:
		# Drifting fog banks.
		draw_rect(r, Color(0.86, 0.88, 0.9, 0.35))
		for i in 6:
			var x := fmod(_t * (12.0 + i * 3.0) + i * 260.0, size.x + 400.0) - 200.0
			draw_circle(Vector2(x, size.y * (0.15 + i * 0.14)), 180.0, Color(0.92, 0.93, 0.95, 0.12))
	var rain := Color(0.75, 0.82, 0.95, 0.45)
	for d in _drops:
		draw_line(Vector2(d[0], d[1]), Vector2(d[0] + d[3] * 0.18, d[1] + d[3]), rain, 1.2)
	if _flash > 0.0:
		draw_rect(r, Color(1, 1, 1, _flash * (0.8 if outdoors else 0.25)))
