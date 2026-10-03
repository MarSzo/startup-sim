## The other half of chat_ola: Kuba gets a notice about Ola's mail, goes to
## the chill room and reads what she writes there (and whispers to him).
extends "res://tests/e2e/scenario.gd"


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	var notices = game().notices
	var mail_notice := func():
		for t in notices.history:
			if str(t).contains("Kawa o 11?"):
				return true
		return false
	if not await until(mail_notice, 60.0, "a notice about Ola's mail"):
		return
	log_step("notified")
	if not await walk(1, 35, 10, 60.0):
		return
	if not await hear("Cześć Kuba!", 60.0):
		return
	if not await hear("(szeptem) tajny projekt", 10.0):
		return
	if not await hear("(krzyczy) OBIAD!", 10.0):
		return
	log_step("read the chat")
