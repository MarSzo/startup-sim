## The applicant (with founder): applies for the new Mobile position (its id
## is the first custom one, 20) and answers by itself (--auto-recruit=20,
## set from here); the founder hires: the invitation to the trial day.
extends "res://tests/e2e/scenario.gd"


func run() -> void:
	var portal = main.portal
	portal.auto_offer = 20
	if not await until(func(): return portal.offers.has(20), 60.0, "the new position on the portal"):
		return
	check(portal.offers[20].title == "Programista/ka mobile", "the right position")
	log_step("applying")
	await until(func():
		for id in portal.mails:
			if str(portal.mails[id].subject).begins_with("Zaproszenie na dzień próbny"):
				return true
		return false, 200.0, "the invitation to the trial day")
