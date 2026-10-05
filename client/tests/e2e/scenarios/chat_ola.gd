## Typed chat and notices (with chat_kuba): Ola mails Kuba from her desk
## (he gets a notice), then in the chill room writes to the room, whispers
## and shouts.
extends "res://tests/e2e/scenario.gd"


func run() -> void:
	if not await until(in_world, 30.0, "at the desk") or not await sit_at_desk():
		return
	if not await until(func(): return messenger_has("Kuba"), 30.0, "Kuba in the office"):
		return
	await pc("mail:Kuba:Kawa o 11?")
	if not await hear("Wysłane", 10.0):  # sent while still at the computer
		return
	await pc("close")
	await until(func(): return not game().screen.visible, 5.0, "standing up")
	log_step("mail sent")
	if not await walk(4, 34, 10, 60.0):  # the chill room
		return
	if not await until(func(): return sees("Kuba"), 60.0, "Kuba in the chill room"):
		return
	await wait(1.0)
	var chat = game().chat_box
	chat.open()
	chat._submit("Cześć Kuba!")
	await wait(1.0)
	chat.open()
	chat._submit("/s tajny projekt")
	await wait(1.0)
	chat.open()
	chat._submit("/k obiad!")
	if not await hear("(krzyczy) OBIAD!", 5.0):  # her own shout comes back too
		return
	await wait(4.0)  # Kuba reads
	check(game().log_history.lines.size() >= 3, "the day's log keeps the lines (%d)" % game().log_history.lines.size())
