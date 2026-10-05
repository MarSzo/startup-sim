## The applicant (with founder): applies for the new Mobile position (its id
## is the first custom one, 20) and answers by itself (--auto-recruit=20,
## set from here) — correctly, from the server's question file, so the
## interview passes the first time (guessing took 1-40 attempts); the
## founder hires: the invitation to the trial day.
extends "res://tests/e2e/scenario.gd"

const QUESTIONS := "../server/data/recruitment.json"  # from the client dir


func run() -> void:
	var correct := _correct_answers("programming")
	if not check(not correct.is_empty(), "the interview questions read"):
		return
	var portal = main.portal
	portal.auto_answer = func(text: String, options: Array) -> int:
		return options.find(correct.get(text, ""))
	portal.auto_offer = 20
	if not await until(func(): return portal.offers.has(20), 60.0, "the new position on the portal"):
		return
	check(portal.offers[20].title == "Programista/ka mobile", "the right position")
	log_step("applying")
	var mail_with := func(subject: String) -> bool:
		for id in portal.mails:
			if str(portal.mails[id].subject).begins_with(subject):
				return true
		return false
	if not await until(func(): return mail_with.call("Zaproszenie na dzień próbny"), 60.0, "the invitation to the trial day"):
		return
	check(not mail_with.call("Dziękujemy za rozmowę"), "passed the first time")


## Question text -> its correct option (the first one in the file).
func _correct_answers(set_id: String) -> Dictionary:
	var path := ProjectSettings.globalize_path("res://").path_join(QUESTIONS)
	var data = JSON.parse_string(FileAccess.get_file_as_string(path))
	var out := {}
	if data is Dictionary:
		for s in data.get("question_sets", []):
			if s.id == set_id:
				for q in s.questions:
					out[q.q] = q.options[0]
	return out
