## After the server restart: logging in again brings the character back -
## the card in the pocket, the laptop still on the desk.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")


func run() -> void:
	if not await until(in_world, 60.0, "back in the world"):
		return
	if not await hear("Z powrotem", 15.0):
		return
	await until(func(): return carrying(Item.EMPLOYEE_CARD), 5.0, "the employee card")
	check(not carrying(Item.LAPTOP), "the laptop not in the pockets (it is on the desk)")
	# Back to the desk (the laptop is seen from its room).
	var desk := desk_of(game().department if game().department != 0 else 1)
	if not await walk(desk[0], desk[1].x, desk[1].y, 90.0):
		return
	if not await until(func(): return not game().computers.is_empty(), 10.0, "the laptop on the desk"):
		return
	check(money() == 200_00, "the money kept (%d gr)" % money())
