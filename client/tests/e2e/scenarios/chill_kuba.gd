## The other half of chill_ola: Kuba waits in the chill room; the match
## comes on, then disco polo from the boombox Ola holds.
extends "res://tests/e2e/scenario.gd"


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	if not await walk(4, 38, 12, 90.0):
		return
	if not await until(func(): return not game().tvs.is_empty() and game().tvs.values()[0][0].channel == 4, 90.0, "the match on the TV"):
		return
	log_step("watching the match")
	var ola := -1
	for id in game().nicks:
		if game().nicks[id] == "Ola":
			ola = id
	if not await until(func(): return game().boombox_music.get("track", 0) == 1 and game().boombox_music.holder == ola, 60.0,
			"disco polo from Ola's boombox"):
		return
	check(game().boombox.playing, "the music plays")
	log_step("hearing the music")
