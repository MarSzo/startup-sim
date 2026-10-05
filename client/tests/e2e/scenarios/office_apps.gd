## The office computer's apps (an employee, server --start-employed): the HR
## app (the contract, leave for tomorrow), the terminal and the Internet page.
extends "res://tests/e2e/scenario.gd"

const Item = preload("res://game/item_art.gd")


func run() -> void:
	if not await until(in_world, 30.0, "in the office"):
		return
	await until(func(): return holding(Item.LAPTOP), 5.0, "the laptop in hands")
	await press_e()
	if not await hear("Laptop na biurku"):
		return
	await press_e()
	var screen = game().screen
	if not await until(func(): return screen.visible, 5.0, "the computer screen"):
		return

	# Kadry: the contract and two days of leave; one of them for tomorrow.
	await pc("win:hr")
	var hr = screen.hr_view
	if not await until(func(): return not hr.info.is_empty(), 10.0, "the HR file"):
		return
	check(hr.info.leave_days == 2 and hr.info.annexes.size() == 1 and str(hr.info.annexes[0].text).begins_with("Umowa:"),
		"the contract and the leave: %s" % hr.info)
	var tomorrow: int = hr.info.today + 1
	await pc("hr:%d:%d" % [Protocol.HR_REQUEST, tomorrow])
	if not await until(func(): return hr.info.requests.size() == 1 and hr.info.requests[0].status == 1, 10.0, "leave approved"):
		return
	check(hr.info.leave_days == 1 and hr.info.requests[0].day == tomorrow, "one day left")
	hr.show_tab("leave")
	log_step("leave for day %d" % tomorrow)

	# The terminal.
	await pc("term:ls")
	check(screen.terminal.shell.history[-1] == "ls", "typed into the terminal")
	check(screen.terminal._out.text.contains("notatki.txt"), "ls in the terminal")
	await pc("term:sudo rm -rf /")
	check(screen.terminal._out.text.contains("Prezes"), "rm -rf / gets the CEO calling")

	# The Internet in the browser (headless: no WebView - the way out to
	# the player's own browser instead).
	await pc("win:web")
	check(screen.page == "web" and screen.web.is_visible_in_tree(), "the Internet page")
	check(screen.web.url == "https://www.onet.pl/", "starts at onet.pl: %s" % screen.web.url)
	check(screen.web.to_url("wp.pl") == "https://wp.pl" and screen.web.to_url("kot w butach").begins_with(screen.web.SEARCH), "addresses and searches")
	log_step("apps work")
	await pc("close")
