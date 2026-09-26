## Job portal (GDD section 4): offers -> quiz -> result. The server decides
## everything; this screen shows whatever it last sent and re-sends our last
## action if the server shows the same screen again (a lost UDP packet).
extends Control

const Protocol = preload("res://net/protocol.gd")

signal apply(offer_id: int)
signal answer(attempt: int, index: int, choice: int)
## Player accepted the job offer result and goes to the building.
signal done

const DEPT_NAMES := {1: "IT / Produkt", 2: "Biznes"}
const RESEND_MSEC := 600

var offers: Array = []
var hired := false
var job_title := ""
var department := 0
## Dev: apply for this offer and answer at random until hired (0 = off),
## waiting `auto_delay` seconds before each action.
var auto_offer := 0
var auto_delay := 0.0

var _screen := ""             # loading / offers / question / waiting / result
var _answered := {}           # "attempt:index" -> choice
var _question_key := ""
var _last_offer := 0
var _applied_at := -1         # msec of the last Apply without a reply yet
var _result_attempt := -1
var _notice := ""

var _content := VBoxContainer.new()


func _ready() -> void:
	# Lives in a CanvasLayer that starts hidden, so anchors alone don't size it:
	# follow the viewport explicitly.
	get_viewport().size_changed.connect(_fit)
	visibility_changed.connect(_fit)
	_fit()
	var bg := ColorRect.new()
	bg.color = Color("#eef1f6")
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	var page := VBoxContainer.new()
	page.set_anchors_preset(Control.PRESET_FULL_RECT)
	page.add_theme_constant_override("separation", 0)
	add_child(page)

	var bar := PanelContainer.new()
	var bar_style := StyleBoxFlat.new()
	bar_style.bg_color = Color("#1f3a5f")
	bar_style.set_content_margin_all(14)
	bar.add_theme_stylebox_override("panel", bar_style)
	var bar_label := Label.new()
	bar_label.text = "  Portal z ofertami pracy   ·   Startup Sim sp. z o.o."
	bar_label.add_theme_font_size_override("font_size", 20)
	bar_label.add_theme_color_override("font_color", Color.WHITE)
	bar.add_child(bar_label)
	page.add_child(bar)

	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	page.add_child(scroll)
	var center := CenterContainer.new()
	center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(center)
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_top", 28)
	margin.add_theme_constant_override("margin_bottom", 28)
	center.add_child(margin)
	_content.custom_minimum_size = Vector2(720, 0)
	_content.add_theme_constant_override("separation", 16)
	margin.add_child(_content)
	_show_loading()


func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size


func reset() -> void:
	offers = []
	hired = false
	_answered.clear()
	_question_key = ""
	_applied_at = -1
	_result_attempt = -1
	_notice = ""
	visible = true
	_show_loading()


## Feed every packet from the net client; recruitment ones are handled here.
func on_packet(p: Dictionary) -> void:
	match p.type:
		Protocol.T_JOB_OFFERS:
			offers = p.offers
			if _screen == "result":
				return
			if _applied_at >= 0:
				if Time.get_ticks_msec() - _applied_at > RESEND_MSEC:
					_send_apply(_last_offer)  # our Apply got lost
				return
			if _screen == "waiting":
				_notice = "Tym razem się nie udało — spróbuj jeszcze raz."
			if _screen != "offers":
				_show_offers()
		Protocol.T_QUESTION:
			_applied_at = -1
			var key := "%d:%d" % [p.attempt, p.index]
			if _answered.has(key):
				answer.emit(p.attempt, p.index, _answered[key])  # our Answer got lost
				return
			if key != _question_key:
				_show_question(p, key)
		Protocol.T_RECRUIT_RESULT:
			if p.attempt != _result_attempt:
				_result_attempt = p.attempt
				_show_result(p)


## The world started sending us snapshots: we were hired even if the result
## packet itself got lost.
func on_entered_world() -> void:
	if not hired and _screen != "result":
		hired = true
		visible = false


func _clear() -> void:
	for c in _content.get_children():
		c.queue_free()


func _label(text: String, size: int, color := Color("#1c2430"), wrap := true) -> Label:
	var l := Label.new()
	l.text = text
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", color)
	if wrap:
		l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return l


func _card() -> VBoxContainer:
	var panel := PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color.WHITE
	sb.border_color = Color("#d5dbe5")
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(10)
	sb.set_content_margin_all(20)
	panel.add_theme_stylebox_override("panel", sb)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	panel.add_child(box)
	_content.add_child(panel)
	return box


func _button(text: String, primary := true) -> Button:
	var b := Button.new()
	b.text = text
	b.custom_minimum_size = Vector2(0, 42)
	b.add_theme_font_size_override("font_size", 17)
	if primary:
		var sb := StyleBoxFlat.new()
		sb.bg_color = Color("#2e6bd9")
		sb.set_corner_radius_all(8)
		sb.set_content_margin_all(10)
		var hover := sb.duplicate()
		hover.bg_color = Color("#3b7bef")
		b.add_theme_stylebox_override("normal", sb)
		b.add_theme_stylebox_override("hover", hover)
		b.add_theme_stylebox_override("pressed", hover)
		b.add_theme_color_override("font_color", Color.WHITE)
		b.add_theme_color_override("font_hover_color", Color.WHITE)
	else:
		var sb := StyleBoxFlat.new()
		sb.bg_color = Color("#f7f9fc")
		sb.border_color = Color("#c9d2df")
		sb.set_border_width_all(1)
		sb.set_corner_radius_all(8)
		sb.content_margin_left = 14
		sb.content_margin_right = 14
		sb.content_margin_top = 10
		sb.content_margin_bottom = 10
		var hover := sb.duplicate()
		hover.bg_color = Color("#e3edfc")
		hover.border_color = Color("#2e6bd9")
		b.add_theme_stylebox_override("normal", sb)
		b.add_theme_stylebox_override("hover", hover)
		b.add_theme_stylebox_override("pressed", hover)
		b.add_theme_stylebox_override("focus", hover)
		for c in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
			b.add_theme_color_override(c, Color("#1c2430"))
	return b


func _show_loading() -> void:
	_screen = "loading"
	_clear()
	_content.add_child(_label("Ładowanie ofert…", 22))


func _show_offers() -> void:
	_screen = "offers"
	_question_key = ""
	_clear()
	_content.add_child(_label("Szukamy ludzi do zespołu!", 32))
	_content.add_child(_label("Mała firma, wielkie ambicje. Wybierz ofertę i odpowiedz na kilka pytań — odezwiemy się szybciej, niż myślisz.", 17, Color("#4a5566")))
	if _notice != "":
		_content.add_child(_label(_notice, 17, Color("#c0392b")))
		_notice = ""
	for o in offers:
		var box := _card()
		box.add_child(_label(o.title, 26))
		box.add_child(_label("Dział: %s" % DEPT_NAMES.get(o.department, "?"), 15, Color("#2e6bd9"), false))
		box.add_child(_label(o.description, 16, Color("#4a5566")))
		var b := _button("Aplikuj")
		var id: int = o.id
		b.pressed.connect(func(): _send_apply(id))
		box.add_child(b)
	if auto_offer > 0:
		_auto(func(): if _screen == "offers": _send_apply(auto_offer))


func _send_apply(offer_id: int) -> void:
	_last_offer = offer_id
	_applied_at = Time.get_ticks_msec()
	apply.emit(offer_id)
	if _screen == "offers":
		_screen = "applying"
		_clear()
		_content.add_child(_label("Wysyłanie aplikacji…", 22))


func _offer_title(id: int) -> String:
	for o in offers:
		if o.id == id:
			return o.title
	return ""


func _show_question(p: Dictionary, key: String) -> void:
	_screen = "question"
	_question_key = key
	_clear()
	_content.add_child(_label("Rekrutacja: %s" % _offer_title(_last_offer), 18, Color("#4a5566")))
	var bar := ProgressBar.new()
	bar.max_value = p.total
	bar.value = p.index
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(0, 8)
	_content.add_child(bar)
	var box := _card()
	box.add_child(_label("Pytanie %d z %d" % [p.index + 1, p.total], 15, Color("#2e6bd9"), false))
	box.add_child(_label(p.text, 24))
	for i: int in p.options.size():
		var b := _button(p.options[i], false)
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		var choice: int = i
		b.pressed.connect(func(): _send_answer(p.attempt, p.index, choice))
		box.add_child(b)
	if auto_offer > 0:
		var n: int = p.options.size()
		_auto(func(): _send_answer(p.attempt, p.index, randi() % n))


func _send_answer(attempt: int, index: int, choice: int) -> void:
	var key := "%d:%d" % [attempt, index]
	if _answered.has(key):
		return
	_answered[key] = choice
	answer.emit(attempt, index, choice)
	_screen = "waiting"
	_clear()
	_content.add_child(_label("Sprawdzamy odpowiedź…", 22))


func _show_result(p: Dictionary) -> void:
	_screen = "result"
	_clear()
	var box := _card()
	if p.passed:
		hired = true
		job_title = _offer_title(_last_offer)
		department = p.department
		box.add_child(_label("Gratulacje — zapraszamy na dzień próbny!", 28, Color("#1e8449")))
		box.add_child(_label("Stanowisko: %s (dział %s). Wynik: %d/%d." % [job_title, DEPT_NAMES.get(department, "?"), p.score, p.total], 18))
		box.add_child(_label("Przyjdź do biura i zgłoś się na portierni — portier zaprowadzi Cię na recepcję, a w HR podpiszesz umowę i dostaniesz kartę.", 16, Color("#4a5566")))
		var b := _button("Idę do biura")
		b.pressed.connect(_finish)
		box.add_child(b)
		if auto_offer > 0:
			_auto(_finish)
	else:
		box.add_child(_label("Tym razem się nie udało", 28, Color("#c0392b")))
		box.add_child(_label("Wynik: %d/%d. Możesz spróbować jeszcze raz — pytania będą inne — albo wybrać inną ofertę." % [p.score, p.total], 18))
		var b := _button("Wróć do ofert")
		b.pressed.connect(_back_to_offers)
		box.add_child(b)
		if auto_offer > 0:
			_auto(_back_to_offers)


func _auto(action: Callable) -> void:
	get_tree().create_timer(maxf(auto_delay, 0.05)).timeout.connect(action)


func _back_to_offers() -> void:
	if offers.is_empty():
		_show_loading()
	else:
		_show_offers()


func _finish() -> void:
	if not visible:
		return
	visible = false
	done.emit()
