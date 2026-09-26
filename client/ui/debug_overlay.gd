## F3 debug overlay.
extends CanvasLayer

var game
var label := Label.new()
var _timer := 0.0


func _ready() -> void:
	layer = 10
	var panel := PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0, 0, 0, 0.6)
	sb.set_content_margin_all(8)
	panel.add_theme_stylebox_override("panel", sb)
	panel.position = Vector2(8, 56)  # below the game clock
	add_child(panel)
	label.add_theme_font_size_override("font_size", 14)
	panel.add_child(label)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.physical_keycode == KEY_F3:
		visible = not visible


func _process(delta: float) -> void:
	_timer -= delta
	if visible and game and _timer <= 0.0:
		label.text = game.debug_text()
		_timer = 0.1
