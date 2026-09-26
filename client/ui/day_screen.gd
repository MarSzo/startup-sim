## Full-screen day cards (game time from the server's Clock packet):
## - at home for the night: "Koniec dnia" with the hours worked and the pay,
## - on the way to work: "Dzień N" with the arrival time,
## - a short "Dzień N" card whenever the personal day number goes up.
extends Control

const Protocol = preload("res://net/protocol.gd")

const CARD_SEC := 3.5

var clock := {}           # last Clock packet
var _day := 0             # personal day already announced
var _card_until := 0.0    # transient card visible until (seconds, engine time)
var _bg := ColorRect.new()
var _title := Label.new()
var _sub := Label.new()
var _info := Label.new()
var _sky := Control.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP
	visible = false
	get_viewport().size_changed.connect(_fit)
	add_child(_bg)
	_sky.draw.connect(_draw_sky)
	add_child(_sky)
	var col := VBoxContainer.new()
	col.alignment = BoxContainer.ALIGNMENT_CENTER
	col.set_anchors_preset(Control.PRESET_FULL_RECT)
	col.add_theme_constant_override("separation", 14)
	add_child(col)
	for l in [_title, _sub, _info]:
		l.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		l.add_theme_color_override("font_color", Color.WHITE)
		col.add_child(l)
	_title.add_theme_font_size_override("font_size", 64)
	_sub.add_theme_font_size_override("font_size", 26)
	_info.add_theme_font_size_override("font_size", 18)
	_info.add_theme_color_override("font_color", Color(1, 1, 1, 0.75))
	_fit()


func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size
	_bg.size = size
	_sky.size = size


static func hhmm(m: int) -> String:
	return "%02d:%02d" % [(m / 60) % 24, m % 60]


static func duration(minutes: int) -> String:
	return "%d h %02d min" % [minutes / 60, minutes % 60]


## True while the player can't play (at home / commuting): block input.
func blocking() -> bool:
	return visible and clock.get("place", 0) in [Protocol.PLACE_HOME, Protocol.PLACE_COMMUTING]


func on_clock(p: Dictionary) -> void:
	clock = p
	if p.day > _day:
		# A new personal day: show its card for a moment (unless a full-screen
		# home / commuting card shows the day anyway).
		_day = p.day
		_card_until = Time.get_ticks_msec() / 1000.0 + CARD_SEC
	_render()


func _process(_d: float) -> void:
	if visible and _card_until > 0.0 and Time.get_ticks_msec() / 1000.0 > _card_until:
		_card_until = 0.0
		_render()
	if visible:
		_sky.queue_redraw()


func _render() -> void:
	if clock.is_empty():
		return
	var place: int = clock.place
	var now := Time.get_ticks_msec() / 1000.0
	match place:
		Protocol.PLACE_HOME:
			visible = true
			_bg.color = Color("#0d1330")
			_title.text = "Koniec dnia" if clock.pay_minutes > 0 else "Noc"
			if clock.pay_minutes > 0:
				_sub.text = "Przepracowane: %s · wypłata %d,%02d zł" % [duration(clock.pay_minutes), clock.pay / 100, clock.pay % 100]
			else:
				_sub.text = "Biuro zamknięte do rana."
			_info.text = "Noc… Teraz %s — nowy dzień zaczyna się o 06:00." % hhmm(clock.minute)
		Protocol.PLACE_COMMUTING:
			visible = true
			_bg.color = Color("#f2a65a").darkened(0.35)
			_title.text = "Dzień %d" % clock.day
			_sub.text = "Dojazd do pracy… przyjazd o %s" % hhmm(clock.arrive)
			_info.text = "Teraz %s" % hhmm(clock.minute)
		_:
			# A short day card over the world / the portal.
			visible = _card_until > now
			_bg.color = Color(0.05, 0.06, 0.1, 0.85)
			_title.text = "Dzień %d" % clock.day
			if place == Protocol.PLACE_PORTAL:
				_sub.text = "Szukasz pracy — przejrzyj ogłoszenia w przeglądarce."
			elif clock.day == 2:
				_sub.text = "Pierwszy dzień w pracy!"
			else:
				_sub.text = "Kolejny dzień w pracy."
			_info.text = hhmm(clock.minute)


## Moon at night, rising sun on the way to work.
func _draw_sky() -> void:
	var c := Vector2(size.x / 2, size.y * 0.22)
	match clock.get("place", -1):
		Protocol.PLACE_HOME:
			_sky.draw_circle(c, 36, Color("#f4f1c9"))
			_sky.draw_circle(c + Vector2(14, -8), 32, Color("#0d1330"))
			for i in 24:
				var sp := Vector2(fmod(i * 197.0, size.x), fmod(i * 83.0, size.y * 0.5))
				_sky.draw_rect(Rect2(sp, Vector2(2, 2)), Color(1, 1, 1, 0.3 + 0.5 * fmod(i * 0.37, 1.0)))
		Protocol.PLACE_COMMUTING:
			_sky.draw_circle(c + Vector2(0, 30), 44, Color("#ffd166"))
