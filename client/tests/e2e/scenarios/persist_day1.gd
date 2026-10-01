## Saving (with persist_day2): a new account at its desk (server
## --start-employed with a save file) leaves the laptop on the desk and
## quits; the runner restarts the server.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")


func run() -> void:
	if not await until(in_world, 60.0, "at the desk"):
		return
	await until(func(): return holding(Item.LAPTOP), 5.0, "the laptop in hands")
	await press_e()
	if not await hear("Laptop na biurku"):
		return
	await until(func(): return not carrying(Item.LAPTOP), 5.0, "the laptop out of the hands")
	log_step("laptop on the desk, %d gr" % money())
	await wait(1.0)
