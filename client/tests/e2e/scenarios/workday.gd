## A working day (an employee, guest login, server --start-employed): laptop
## on the desk, lunch ordered in the app, a coffee from the kitchenette, the
## lunch picked up at the reception, then home early by tram (paid).
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	await until(func(): return money() >= 0 and holding(Item.LAPTOP), 5.0, "the laptop in hands")
	var start_money := money()
	log_step("at the desk with %d gr" % start_money)

	await press_e()
	if not await hear("Laptop na biurku"):
		return
	await press_e()
	if not await until(func(): return game().screen.visible, 5.0, "the computer screen"):
		return
	await pc("lunch:%d" % Item.KEBAB)
	if not await until(func(): return money() == start_money - 25_00, 5.0, "paying 25 zł for the kebab"):
		return
	await pc("close")
	await until(func(): return not game().screen.visible, 5.0, "standing up")
	log_step("lunch ordered")

	if not await walk(1, 21, 8):  # the cupboard with mugs
		return
	await press_e()
	if not await until(func(): return game().dialog.visible, 5.0, "the cupboard open"):
		return
	game().dialog._choose(0)  # Weź kubek
	if not await until(func(): return holding(Item.CUP), 5.0, "a mug from the cupboard"):
		return
	if not await walk(1, 25, 8):  # the coffee machine
		return
	await press_e()
	if not await hear("Kawa gotowa", 8.0):
		return
	await until(func(): return holding(Item.COFFEE), 5.0, "the coffee in hands")
	await item(Protocol.ITEM_USE)
	if not await until(func(): return not holding(Item.COFFEE), 5.0, "drinking the coffee"):
		return
	await item(Protocol.ITEM_DROP)  # free hands for the lunch box
	log_step("coffee drunk")

	if not await walk(1, 36, 36):  # in front of the reception desk
		return
	if not await hear("Kurier był!", 40.0):
		return
	await press_e()
	if not await until(func(): return holding(Item.KEBAB), 5.0, "the kebab from the reception"):
		return
	await item(Protocol.ITEM_USE)
	log_step("lunch eaten")

	if not await walk(0, 36, 69):  # the tram stop
		return
	await press_e()
	if not await hear("Wracam do domu?"):
		return
	await press_e()
	await until(func():
		var c: Dictionary = last.get(Protocol.T_CLOCK, {})
		return c.get("place", -1) == Protocol.PLACE_HOME and c.get("pay", 0) > 0, 10.0, "home, paid for the day")
