## A page of the office computer's browser beyond the work tools: the news
## portal (headlines from the office and the town), the weather and memes.
## A button opens the real onet.pl in the player's own browser.
extends VBoxContainer

const Ink = preload("res://ui/ink_ui.gd")

const REAL_NEWS := "https://www.onet.pl/"
const TEXT := Color("#1c2430")
const DIM := Color("#4a5566")
const RED := Color("#c0392b")

## What the pages know about the world: {day, minute, weather (name), company, nick}.
var ctx := {}
var kind := "news"
var _sig := ""

const TOWN := [
	"Tramwaj linii 7 znów spóźniony. Pasażerowie: „Przynajmniej jest stabilnie”",
	"Nowa kawiarnia w centrum: kawa za 28 zł, w papierowym kubku",
	"Dzik w parku miejskim. Straż miejska: „Był pierwszy”",
	"Ceny pomidorów na rynku biją rekordy. Działkowicze zacierają ręce",
	"Remont ulicy po raz trzeci w tym roku. „Tym razem na pewno”",
	"Mieszkańcy pytają o ścieżkę rowerową. Miasto: „Pytajcie dalej”",
	"Rekordowe korki na wylotówce. Kierowcy polecają tramwaj, tramwaj poleca rower",
	"Miejski festyn: pierogi, disco polo i loteria fantowa (wygrana: kolejne pierogi)",
]
const OFFICE := [
	"Startup z naszego biurowca szuka ludzi. Widełki: „konkurencyjne”",
	"Tajemnicza kałuża w holu. Sprzątaczka: „Co za cham!”",
	"Ekspres w aneksie kuchennym robi „specjalną” kawę? Pracownicy podzieleni",
	"Pani Wiesia z portierni: „A kiedy ślub?”. Pytanie zadano już 400 razy",
	"Pani Paulina siedzi w fotelu od rana. Eksperci: „To styl życia”",
	"Na balkonie znów dymek. Straż pożarna: „Już znamy drogę”",
	"Prezes rozważa owocowe czwartki codziennie. Owoce rozważają zepsucie się",
	"Kasjer w sklepie na parterze pyta o parówkę. Klienci: „Jaka jest?”",
]
const MEMES := [
	["Kiedy deploy w piątek o 17:59", "„Działa u mnie” — ostatnie słowa przed weekendem"],
	["Ja: przyjdę jutro wcześniej", "Też ja, jutro: *wybiera tramwaj o 9:00*"],
	["Spotkanie, które mogło być mailem", "Mail, który mógł być niczym"],
	["Kubek w zlewie", "Pani Maria: *wpis na #ogólny*"],
	["Widełki: 8–12 tys.", "Umowa: „drobna korekta, standard w branży”"],
	["Kawa nr 1: energia", "Kawa nr 5: widzę dźwięki"],
]


func setup(p_kind: String) -> void:
	kind = p_kind
	add_theme_constant_override("separation", 10)


## The world changed (a new day, the weather): redraw if it matters.
func refresh(p_ctx: Dictionary) -> void:
	ctx = p_ctx
	var sig := "%s|%s|%s" % [kind, ctx.get("day", 0), ctx.get("weather", "")]
	if sig == _sig:
		return
	_sig = sig
	for c in get_children():
		c.queue_free()
	match kind:
		"news": _news()
		"weather": _weather()
		"memes": _memes()


func _news() -> void:
	var head := HBoxContainer.new()
	var logo := _label("Plotek.pl", 30, RED)
	logo.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(logo)
	var real := Ink.button("Otwórz onet.pl w prawdziwej przeglądarce ↗", false)
	real.add_theme_font_size_override("font_size", 14)
	real.pressed.connect(open_real_news)
	head.add_child(real)
	add_child(head)
	add_child(_label("Wiadomości z biura i z miasta · dzień %d" % ctx.get("day", 1), 14, DIM))
	var day: int = ctx.get("day", 1)
	var weather: String = ctx.get("weather", "")
	var top := "Pogoda dziś: %s. Synoptycy: „Za oknem jak za oknem”" % weather if weather != "" else "Pogoda: bez zmian"
	add_child(_card(top, "Pilne", true))
	for i in 3:
		add_child(_card(OFFICE[(day * 3 + i) % OFFICE.size()], "Z biura", false))
	for i in 3:
		add_child(_card(TOWN[(day * 5 + i) % TOWN.size()], "Z miasta", false))
	add_child(_label("Reklama: Kup kubek z logo firmy! (Ostatnie 3 sztuki od 2019 roku.)", 13, DIM))


func _weather() -> void:
	add_child(_label("Pogoda.example", 28, Color("#2e6bd9")))
	var weather: String = ctx.get("weather", "")
	add_child(_label("Teraz przy biurze: %s" % (weather if weather != "" else "brak danych"), 22, TEXT))
	var tips := {"słonecznie": "Idealnie na przerwę na balkonie.", "pochmurno": "Parasol na wszelki wypadek.",
		"deszcz": "Weź parasol (stojak w sklepie na parterze).", "burza": "Lepiej posiedzieć w chill roomie.",
		"mgła": "Tramwaj i tak się spóźni."}
	add_child(_label(tips.get(weather, "Patrz przez okno."), 16, DIM))
	add_child(_label("Jutro: to samo, ale bardziej.", 16, DIM))


func _memes() -> void:
	add_child(_label("Memy.example — najlepsze z biura", 26, TEXT))
	var day: int = ctx.get("day", 1)
	for i in 4:
		var m: Array = MEMES[(day + i) % MEMES.size()]
		add_child(_card(m[0], m[1], false))


func _card(title: String, sub: String, hot: bool) -> PanelContainer:
	var p := PanelContainer.new()
	p.add_theme_stylebox_override("panel", Ink.box("card"))
	var v := VBoxContainer.new()
	p.add_child(v)
	v.add_child(_label(title, 18, RED if hot else TEXT))
	v.add_child(_label(sub, 14, DIM))
	return p


func _label(text: String, size: int, color: Color) -> Label:
	var l := Label.new()
	Ink.style_label(l, size, color)
	l.text = text
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return l


## The real thing, outside the game (never in tests / headless).
static func open_real_news() -> void:
	if DisplayServer.get_name() != "headless":
		OS.shell_open(REAL_NEWS)
