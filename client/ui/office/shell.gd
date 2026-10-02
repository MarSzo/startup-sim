## The office computer's terminal: a make-believe shell with the company's
## files, git, ssh to prod and the usual jokes. Nothing runs on the player's
## machine - it's all text.
extends RefCounted

## `run` returns this to clear the screen / close the window.
const CLEAR := "\u0001clear"
const EXIT := "\u0001exit"

var nick := "ja"
var company := "Startup Sim sp. z o.o."
var department := ""
var day := 1
var minute := 8 * 60
var weather := ""
var cwd := ""
var history: PackedStringArray = []
var _files := {}  # path -> content; directories end with "/"


func setup(p_nick: String, p_company: String, p_department: String) -> void:
	nick = p_nick.to_lower().replace(" ", "_") if p_nick != "" else "ja"
	company = p_company
	department = p_department
	cwd = home()
	_files = {
		"/": "", "/home/": "", home() + "/": "",
		home() + "/notatki.txt": "- zapytać o podwyżkę (znowu)\n- nie pić kawy z ekspresu w aneksie (?)\n- oddać kubki, zanim Pani Maria znowu napisze na #ogólny",
		home() + "/todo.md": "# TODO\n- [x] przyjść do pracy\n- [ ] zrobić zadanie z tablicy\n- [ ] udawać, że robię zadanie z tablicy",
		home() + "/.bash_history": "git push --force\nsudo rm -rf /\nexit",
		"/srv/": "", "/srv/startup/": "",
		"/srv/startup/README.md": "# %s\n\nRewolucyjny produkt. Wkrótce.\nUruchomienie: ./deploy.sh (tylko w piątek po 17:00)" % company,
		"/srv/startup/deploy.sh": "#!/bin/sh\necho \"Wdrażam na produkcję…\"\nsleep 1\necho \"Działa u mnie.\"",
		"/srv/startup/src/": "",
		"/srv/startup/src/main.rs": "fn main() {\n    // TODO: produkt\n    println!(\"Hello, inwestorzy!\");\n}",
		"/etc/": "",
		"/etc/motd": "Witaj na serwerze firmowym. Nie wyłączaj zasilania. Nie podlewaj serwera.",
		"/var/": "", "/var/log/": "",
		"/var/log/obiady.log": "08:12 zamówienie: pierogi ruskie\n11:47 kurier zgubił się na parkingu\n12:30 dostarczono (zimne)",
	}


func home() -> String:
	return "/home/" + nick


## The prompt shown before the input: nick@startup:~$
func prompt() -> String:
	var where := cwd.replace(home(), "~") if cwd.begins_with(home()) else cwd
	return "%s@startup:%s$ " % [nick, where]


## Run one line; the output (or CLEAR / EXIT).
func run(line: String) -> String:
	line = line.strip_edges()
	if line == "":
		return ""
	history.append(line)
	var args := Array(line.split(" ", false))
	var cmd: String = args.pop_front()
	var rest := " ".join(args)
	var out := _basic(cmd, args, rest)
	return out if out != NONE else _fun(cmd, rest)


## Not one of these commands.
const NONE := "\u0002"


## Files, the system, git.
func _basic(cmd: String, args: Array, rest: String) -> String:
	match cmd:
		"help":
			return "Komendy: ls, cd, pwd, cat, echo, whoami, date, clear, history, uname, git, ping, top, ssh, sudo, rm,\n" \
				+ "fortune, cowsay, neofetch, curl, npm, vim, kawa, exit"
		"clear":
			return CLEAR
		"exit", "logout":
			return EXIT
		"pwd":
			return cwd
		"whoami":
			return nick
		"hostname":
			return "startup"
		"date":
			return "Dzień %d, %02d:%02d (czas biurowy, nie mylić z wolnym)" % [day, minute / 60, minute % 60]
		"echo":
			return rest
		"history":
			var out := PackedStringArray()
			for i in history.size():
				out.append("%4d  %s" % [i + 1, history[i]])
			return "\n".join(out)
		"uname":
			return "StartOS 4.2.0-startup #1 SMP PREEMPT (zbudowane w piątek o 17:59) x86_64"
		"ls":
			return _ls(args)
		"cd":
			return _cd(rest)
		"cat", "less", "more":
			return _cat(rest)
		"git":
			return _git(args)
	return NONE


## The rest (and the jokes).
func _fun(cmd: String, rest: String) -> String:
	match cmd:
		"ping":
			var host := rest if rest != "" else "localhost"
			return "PING %s: 64 bajty, czas=1337 ms (Wi-Fi w biurze)\nPING %s: 64 bajty, czas=2048 ms\n--- 2 pakiety, 0%% strat, 100%% cierpliwości ---" % [host, host]
		"top", "htop":
			return "  PID UŻYTK.  CPU%%  POLECENIE\n  1 root     99.9  slack\n 42 %s    87.3  chrome (148 kart)\n 77 root     12.0  ekspres-do-kawy\n" % nick \
				+ "  3 root      0.1  produktywność"
		"ssh":
			if rest.contains("prod"):
				return "Łączenie z prod… Ostrzeżenie: jesteś na PRODUKCJI. Prezes patrzy.\nprod$ (połączenie zamknięte przez administratora: \"nie dziś\")"
			return "ssh: brak klucza. Klucz ma tylko DevOps (i on jest na urlopie)."
		"sudo":
			if rest.begins_with("rm") and rest.contains("-rf /"):
				return _rm_rf()
			return "%s nie jest w pliku sudoers. Ten incydent zostanie zgłoszony (do Pani Marii)." % nick
		"rm":
			if rest.contains("-rf /") or rest == "-rf /*":
				return _rm_rf()
			return "rm: nie ruszaj, to działa (nikt nie wie dlaczego)"
		"fortune":
			var f := ["Dziś dobry dzień na refaktoryzację. Jutro też. Nigdy się nie skończy.",
				"Kto rano wstaje, ten ma więcej spotkań.", "Kod bez testów to kod z niespodzianką.",
				"Działa u mnie — najstarsza dokumentacja świata.", "Kawa z ekspresu w aneksie: pij na własne ryzyko."]
			return f[(day + history.size()) % f.size()]
		"cowsay":
			var text := rest if rest != "" else "Muuu… deploy w piątek?"
			var bar := "-".repeat(text.length() + 2)
			return " %s\n< %s >\n %s\n        \\   ^__^\n         \\  (oo)\\_______\n            (__)\\       )\\/\\\n                ||----w |\n                ||     ||" % [bar, text, bar]
		"neofetch":
			return "   _____      %s@startup\n  / ___/      OS: StartOS 4.2\n  \\__ \\       Firma: %s\n ___/ /       Dział: %s\n/____/        Kawa: %s\n              Uptime: dzień %d" \
				% [nick, company, department if department != "" else "—", "tak" if minute < 12 * 60 else "za dużo", day]
		"curl", "wget":
			if rest.contains("wttr"):
				return "Pogoda przy biurze: %s. Za oknem jak za oknem." % (weather if weather != "" else "nie wiadomo (brak okna)")
			return "curl: (6) Nie można rozwiązać hosta — proxy firmowe blokuje wszystko poza pracą."
		"npm", "yarn", "pnpm":
			return "added 1487 packages, and audited 1488 packages in 42s\n\n34 vulnerabilities (12 moderate, 21 high, 1 critical)\n\nnode_modules: 1,3 GB. Gratulacje."
		"vim", "vi", "nano", "emacs":
			return "Otworzono %s. Nikt nie wie, jak z tego wyjść. (:q!)" % cmd
		"kawa", "coffee", "make":
			if cmd == "make" and rest != "coffee":
				return "make: *** Brak reguły do zrobienia '%s'. Stop." % rest
			return "HTTP 418: Jestem czajnikiem. Ekspres jest w aneksie kuchennym."
		"./deploy.sh", "sh":
			return "Wdrażam na produkcję…\nDziała u mnie."
	return "%s: nie znaleziono polecenia (spróbuj: help)" % cmd


func _rm_rf() -> String:
	return "Usuwanie /bin… /etc… /home…\n\n*dzwoni telefon* — Prezes: „Co się stało z serwerem?!”\nNa szczęście to tylko symulacja. Tym razem."


func _abs(p: String) -> String:
	if p == "" or p == "~":
		return home()
	if p.begins_with("~/"):
		p = home() + p.substr(1)
	elif not p.begins_with("/"):
		p = (cwd if cwd != "/" else "") + "/" + p
	var out := PackedStringArray()
	for part in p.split("/", false):
		if part == "..":
			if not out.is_empty():
				out.remove_at(out.size() - 1)
		elif part != ".":
			out.append(part)
	return "/" + "/".join(out)


func _is_dir(path: String) -> bool:
	return path == "/" or _files.has(path + "/")


func _cd(p: String) -> String:
	var path := _abs(p)
	if not _is_dir(path):
		return "cd: %s: nie ma takiego katalogu" % p
	cwd = path
	return ""


func _ls(args: Array) -> String:
	var all := "-la" in args or "-a" in args or "-al" in args
	var target := cwd
	for a in args:
		if not String(a).begins_with("-"):
			target = _abs(a)
	if not _is_dir(target):
		return "ls: %s: nie ma takiego pliku ani katalogu" % target
	var base := target if target.ends_with("/") else target + "/"
	if target == "/":
		base = "/"
	var names := PackedStringArray()
	for path in _files:
		var p: String = path
		if p == base or not p.begins_with(base):
			continue
		var tail := p.substr(base.length())
		var cut := tail.find("/")
		if cut != -1 and cut != tail.length() - 1:
			continue  # deeper down
		if tail.begins_with(".") and not all:
			continue
		names.append(tail)
	names.sort()
	return "  ".join(names)


func _cat(p: String) -> String:
	if p == "":
		return "cat: podaj plik"
	var path := _abs(p)
	if _is_dir(path):
		return "cat: %s: to katalog" % p
	if not _files.has(path):
		return "cat: %s: nie ma takiego pliku" % p
	return _files[path]


func _git(args: Array) -> String:
	var sub: String = args[0] if not args.is_empty() else ""
	match sub:
		"status":
			return "Na gałęzi main\nZmiany nieprzygotowane do zatwierdzenia:\n\tzmieniono: src/main.rs (od trzech tygodni)"
		"log":
			return "commit a1b2c3d (HEAD -> main)\n    Poprawka poprawki\ncommit 9f8e7d6\n    Poprawka\ncommit 0000001\n    Pierwszy commit (działa)"
		"blame":
			return "%s (dzień %d) // TODO: produkt" % [nick, day]
		"push":
			return "! [rejected] main -> main (non-fast-forward)\nPodpowiedź: ktoś był szybszy. Jak zawsze."
		"pull":
			return "Już aktualne. (Podejrzanie.)"
	return "git: '%s' nie jest poleceniem gita. Zobacz 'git --help' (nikt nie zobaczy)." % sub
