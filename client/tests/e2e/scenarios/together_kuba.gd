## The other half of together_ola: Kuba reads Ola's message, answers and
## goes to the chill room.
extends "res://tests/e2e/scenario.gd"


func run() -> void:
	if not await until(in_world, 30.0, "at the desk") or not await sit_at_desk():
		return
	if not await until(func(): return messenger_has("Ola"), 20.0, "Ola on the messenger"):
		return
	await pc("open:dm:Ola")
	if not await until(func(): return got_chat("Kawa w chill roomie?"), 40.0, "Ola's message"):
		return
	await pc("say:dm:Ola:Jasne, idę!")
	await pc("close")
	await until(func(): return not game().screen.visible, 5.0, "standing up")
	log_step("answered Ola")
	if not await walk(4, 36, 12):  # the chill room
		return
	if not await until(func(): return sees("Ola"), 40.0, "Ola in the chill room"):
		return
	log_step("met Ola")
	await wait(4.0)  # stay until she has seen us too
