## The other half of fight_ola: Kuba waits in the chill room and gets
## knocked out (lying, his health at 0 on the HUD).
extends "res://tests/e2e/scenario.gd"


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	if not await walk(1, 35, 10):  # the chill room, next to where Ola stops
		return
	log_step("waiting in the chill room")
	var hud = game().stats_hud
	if not await until(func(): return game().me.status == Protocol.ACT_KNOCKED_OUT, 120.0, "knocked out"):
		return
	await until(func(): return hud.values[7] == 0, 3.0, "no health left")
	check(hud.values[7] == 0, "health 0 (%d)" % hud.values[7])
	log_step("knocked out")
	await wait(3.0)
	check(game().me.status == Protocol.ACT_KNOCKED_OUT, "still out cold")
