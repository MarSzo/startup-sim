## Two players at once (with together_kuba): Ola writes to Kuba on the
## messenger, waits for his answer, meets him in the chill room, then takes
## the lift down to the ground floor.
extends "res://tests/e2e/scenario.gd"


func run() -> void:
	if not await until(in_world, 30.0, "at the desk") or not await sit_at_desk():
		return
	if not await until(func(): return messenger_has("Kuba"), 20.0, "Kuba on the messenger"):
		return
	await pc("say:dm:Kuba:Cześć Kuba! Kawa w chill roomie?")
	await pc("open:dm:Kuba")
	if not await until(func(): return got_chat("Jasne, idę!"), 40.0, "Kuba's answer"):
		return
	await pc("close")
	await until(func(): return not game().screen.visible, 5.0, "standing up")
	log_step("Kuba answered")

	if not await walk(1, 34, 10):  # the chill room
		return
	if not await until(func(): return sees("Kuba"), 40.0, "Kuba in the chill room"):
		return
	log_step("met Kuba")

	# The left lift (A) down to the ground floor.
	if not await walk(1, 37, 44):
		return
	await press_e()
	var floor1 = game().building.get_floor(1)
	if not await until(func(): return not floor1.is_closed(37, 43), 10.0, "the lift open"):
		return
	if not await walk(1, 37, 42):  # into the cabin
		return
	await press_e()
	if not await hear("Jedziemy na: Parter"):
		return
	await until(func(): return floor_now() == 0, 10.0, "down on the ground floor")
