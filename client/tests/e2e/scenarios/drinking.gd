## Drinking at work (an employee, server --start-employed): a wine and two
## małpki from the shop's alcohol shelf, paid at the till, drunk one after
## another (30 + 25 + 25) - the last one comes back up (a vomit puddle),
## and the walk away staggers.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")
const ALCOHOL_SHELF := 5


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	await until(func(): return holding(Item.LAPTOP), 5.0, "the laptop in hands")
	await item(Protocol.ITEM_DROP)  # hands free for the bottles
	var start_money := money()
	if not await walk(0, 20, 47, 90.0):  # in front of the alcohol shelf
		return
	for kind in [Item.WINE, Item.MALPKA, Item.MALPKA]:
		game().net.send(Protocol.encode_shop_take(game().net.token, ALCOHOL_SHELF, kind))
		await wait(0.4)
	if not await until(func(): return holding(Item.WINE) and items().count(Item.MALPKA) == 2, 5.0, "wine and two małpki"):
		return
	if not await walk(0, 20, 53):  # at the till
		return
	await press_e()
	if not await until(func(): return money() == start_money - 45_00, 5.0, "paying 45 zł"):
		return
	log_step("bought the drinks")

	var hud = game().stats_hud
	await item(Protocol.ITEM_USE)  # the wine
	if not await until(func(): return hud.values[5] >= 29, 5.0, "tipsy after the wine"):
		return
	check(game().me.drunk == 1, "tipsy (tier %d)" % game().me.drunk)
	for n in 2:
		await item(Protocol.ITEM_TAKE_OUT, items().find(Item.MALPKA) - 1)
		if not await until(func(): return holding(Item.MALPKA), 5.0, "a małpka in hands"):
			return
		await item(Protocol.ITEM_USE)
	if not await until(func(): return game().me.status == Protocol.ACT_VOMITING, 5.0, "throwing up"):
		return
	await hear("Bl")
	check(game().me.drunk == 2, "drunk (tier %d)" % game().me.drunk)
	if not await until(func():
		for pv in game().puddles.values():
			if pv.vomit:
				return true
		return false, 5.0, "a vomit puddle"):
		return
	log_step("threw up")
	await until(func(): return game().me.status != Protocol.ACT_VOMITING, 10.0, "able to move again")
	# Staggering (drunk = slow + drift), but still getting where we go.
	await walk(0, 24, 51)
