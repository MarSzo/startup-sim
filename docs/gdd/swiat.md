# Budynek i świat

*Część [GDD](README.md). Mapy, windy, schody, czas, dojazd, pogoda, balkon, powrót do domu.*

## 10.5 Budynek (mapy)

Układ wg odręcznego planu (numery w nawiasach to numery z rysunku). Mapy
70×72 kafli po 16 px (1 kafel ≈ 1 m; skala: pokój 18 mieści 8 biurek);
pliki `client/maps/building.json`, `floor0.json`, `floor1.json`, `floor3.json`
(klatka schodowa) generuje `tools/build_maps.py` — jedyne miejsce, gdzie się je
zmienia. Piętro 2 jest w `building.json` jako zablokowane (bez pliku).

**Parter + teren zewnętrzny** — gracz startuje na chodniku przed wejściem.
Wiatrołap (2) → hol (3) z ladą portiera (5, portier siedzi na 6), toaletą (7)
i bramkami na kartę; za bramkami windy (8, 9),
klatka schodowa (4) i drzwi na parking wewnętrzny (12, brama od północy,
dojazd wzdłuż zachodniej ściany). Sklep (1) z wejściem od ulicy, strefa
zamknięta (11) za zablokowanymi drzwiami, sprzątaczka siedzi w holu (10).
Na zewnątrz: chodnik, ulica, parking zewnętrzny, postój taksówek, przystanek
tramwaju, stojak na rowery, strefa palenia i popielniczka przy wejściu.

```
FvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fvvvvvvvvvvv=============================vvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fvvvvvvvvvvv=============================vvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fvvvvvvvvvvv=============================vvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fvvvvvvvvvvv=============================vvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v###############gggggg################vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX====XXX===============XXX====XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#=======XXX===============XXX=======#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#XXX=============================XXX#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#===================================#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#############DD######################vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#SSS.....#P______#eee#eee#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#SSS.....D_______#eee#eee#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#........#_______#EEE#EEE#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v##########_______________#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#........#_______________x..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#H.HHH..H#BBBBBBB#########..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#H......H#_______K___#:U:#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#H......H#_______K___#:::#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#H.HHH..H#_______K___#:::#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#........#_______K___#::V#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#..HHH...#_______K___#:::#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#.......H#___________##k##..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#.......H####DD###_______#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#KKK.....#_______#_______#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#........#_______#_______x..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v#........#P______#______P#..........#vvvvvvvvvvvvvvF
Fvvvvvvvvvvv=====v####GG######GGG######################vvvvvvvvvvvvvvF
FpppppppppppppppppppppppApppppppppppbbbbpppppppppppppppppppppppppppppF
FppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppF
FppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppppF
FrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrF
Fvv============================vvvvvvvvvvvzzzzzzzzzzzzzvvvvvvvvvvvvvvF
Fvv=XXX==XXX==XXX==XXX==XXX====vvvvvvvvvvvzzzzzzzzzzzzzvvvvvvvvvvvvvvF
Fvv=XXX==XXX==XXX==XXX==XXX====vvvvvvvvvvvzzNNNzzzzzzzzvvvvvvvvvvvvvvF
Fvv============================vvvvvvvvvvvzzzzzzzzzzzzzvvvvvvvvvvvvvvF
Fvv============================vvvvvvvvvvvzzzzzzzzAzzzzvvvvvvvvvvvvvvF
Fvv============================vvvvvvvvvvvzzzzzzzzzzzzzvvvvvvvvvvvvvvF
Fvv============================vvvvvvvvvvvzzzzzzzzzzzzzvvvvvvvvvvvvvvF
Fvv============================vpppppppppvvvvvvvvvvvvvvvvvvvvvvvvvvvvF
FttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttF
FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
```

**Piętro 1** — z klatki schodowej (24) na korytarz zachodni (17), hol windowy
(13, nad windami 14/15 zamknięta szafa 58) i główny korytarz (33) z wyspą:
WC damskie (54), łazienka damska (53), WC męskie (52) przez łazienkę męską (51),
przejście, WC dla niepełnosprawnych (50), szafa (55), lada recepcji (49,
recepcjonistka na 56), kosz (57). Na górze chill room (34) otwarty na aneks
kuchenny (35) z wyjściem na balkon (48, można palić). Lewa kolumna: serwerownia
(36, zamknięta), Zarząd / pokój prezesa (37), Produkt/IT (38), sale spotkań
(39, 40). Prawa: sala spotkań (47), HR (46), Marketing (44) ze składzikiem
sprzątaczki (45), Sales (43), Obsługa klienta (42), magazynek (41). Skrzydło:
Mobile (20), łazienka damska (21) z kabinami (22, 23), Produkt/IT (19, 18),
Finanse (16), korytarz wschodni (25), pokój z jednym biurkiem (26), Biznes (27),
AI team (28), DevOps „Mordor” (32), łazienka męska (29) z pisuarami (30) i
kabiną (31).

```
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~hhhhhhhhhhhhhhhh~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~hnNNNnnnnnnnnAnh~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~hnnnnnnnnnnnnnnh~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~hnnnnnnnnnnnnnnh~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~hnnnnnnnnnnnnnnh~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#####GG#############################~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#fccJ::CJid,,,,,,,,,,,,OY#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#::::::::::,,QQQ,,,QQQ,,,#,,TTTTT,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#::::::::::,,QQQ,,,QQQ,,,D,,TTTTT,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#:TTT::::::,,,,,,,,,,,,,,#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#:TTT::::::,,,,,,TTT,,,,,###########~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#::::#######,,,,,,,,,,,,,#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#::::#RRRRR#,,,,,,,,,,,,,#,,WWWW,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#::::#:::::#P....#####...D,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#::::#:::::x.....#:::#...###########~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#::::#RRR::#.....#U::k...#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~############.....#####...#,,,,WWWW,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....D::V#...#####,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,TTTTT,,,Z.....#::Y#...#HH:#,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,TTTTT,,,#.....#####...L:::#,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....#U::#...#####WWWW,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~############.....#:::#...#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....##k##...D,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,WWWW,,,,#.....#V:Y#...###########~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....#:::#...#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....##D##...#,,WWWW,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,WWWW,,,,D.............#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....##k##...#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....#V::#...D,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~############.....#:::#...#,,WWWW,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#.....#::U#...#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,TTTTT,,,D.....#####...###########~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#......www....#,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~############.............#,,WWWW,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,,,,,,,,,#....KKKKK....D,,,,,,,,,#~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~#,,TTTTT,,,D............o#,,,,,,,,,#~~~~~~~~~~~~~
~~#################,,,,,,,,,,#P............#,,,,,,,,,#############~
~~#,,,,,,,,,,,,,,,##############DD####################,,,,,,,,,,,#~
~~#,,WWWWWW,,WWW,,#SSS.......#P....x:::::::#HHHH:HHHH#,,,,,,,,,,,#~
~~#,,,,,,,,,,,,,,,#SSS.......#.....#########:::::::::#,,WWWW,WWW,#~
~~#,,,,,,,,,,,,,,,#..........#.....#eee#eee#:::::::::#,,,,,,,,,,,#~
~~#,,,,,,,,,,,,,,,#..........#.....#eee#eee#:::::::::#,,,,,,,,,,,#~
~~###########DD#######DD######.....#EEE#EEE#####D#####,,,,,,,,,,,#~
~~#|||::::V#.................#.............#.........#,,WWWW,WWW,#~
~~#U:k::::V#.................D.............D.........D,,,,,,,,,,,#~
~~#|||:::::D.................D.............D.........D,,,,,,,,,,,#~
~~#U:k:::::#.................#.............#.........#,,,,,,,,,,,#~
~~#|||::::Y#.................#............P#.........#,,,,,,,,,,,#~
~~##########DD#######DD##########DD#############.....#############~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,#,,,#..........#VV:uuu#~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,#W,,D..........D::::::#~
~~#,,WWWW,WWW,#,,,WWWW,,,,,#,,WWWWW,,WWWW,,#,,,#..........#:::|||#~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,#,,,#..........#Y::k:U#~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,######DD####DD#########~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,#,,,,,,,,,#,,,,,,,,,,,#~
~~#,,WWWW,WWW,#,,,WWWW,,,,,#,,WWWWW,,WWWW,,#,,,,,,,,,#,,,,,,,,,,,#~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,#,,WWWWW,,#,,WWWWWWW,,#~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,#,,,,,,,,,#,,,,,,,,,,,#~
~~#,,,,,,,,,,,#,,,,,,,,,,,,#,,,,,,,,,,,,,,,#,,,,,,,,,#,,,,,,,,,,,#~
~~################################################################~
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
```

Legenda: `#` ściana, `.` podłoga, `,` wykładzina, `:` płytki, `_` posadzka holu,
`=` parking / podjazd, `v` trawa, `p` chodnik, `r` ulica, `t` tory, `z` strefa
palenia, `F` ogrodzenie, `~` pustka, `n`/`h` balkon i barierka, `D` drzwi, `G`
szklane drzwi, `B` bramka na kartę, `L` drzwi obsługi (składzik), `x` drzwi
zablokowane, `Z` drzwi zarządu (tylko na spotkanie), `k` drzwi kabiny WC, `g`
brama garażowa, `E`/`e` drzwi i kabina windy, `S` biegi schodów, `W` biurka,
`T` stoły, `K` lady, `H` regały / półki, `Q` sofy, `P` rośliny, `X` samochody,
`U`/`u` toalety / pisuary, `V` umywalki, `Y` płyn do rąk, `C` ekspres, `c`
szafka z kubkami, `i` zlew, `d` zmywarka, `f` lodówka, `O` owoce, `w` szafa,
`o` kosz, `R` szafy serwerowe, `A` popielniczki, `N` ławki, `b` stojak rowerowy.

Bramki (`B`) i brama garażowa (`g`) wymagają przepustki lub karty przy
wejściu, wyjście jest wolne; składzik (`L`) — uprawnień obsługi; drzwi `x`
nie przepuszczają nikogo. Biurka należą do działu pokoju (pole `department`
w definicji pokoju, patrz rozdział 5): każdy zespół to osobny dział; stół w
pokoju prezesa to miejsce Zarządu; pokój z jednym biurkiem (26) jest niczyj. Punkty otoczenia
(ulica, przystanki, miejsca parkingowe, stojak, gdzie staje policja i straż),
taca ze słodyczami, miejsce założyciela i półki sklepu są w `places` mapy.

**Poruszanie między piętrami:** schody — wejście na biegi schodów przenosi do
klatki schodowej (osobny widok: bieg, półpiętro, drugi bieg), a jej koniec
na drugie piętro; winda — trzeba ją wezwać (E przy drzwiach), poczekać, wejść
i wybrać piętro (E w kabinie) — szczegóły w 10.18.

## 10.18 Windy

- **Dwie windy obok siebie** (8/9 na parterze, 14/15 na piętrze), każda jeździ
  osobno: własne wezwania, drzwi i wyświetlacz. Stojąc między nimi, E wzywa
  bliższą.
- Drzwi są zamknięte, dopóki winda nie stoi na piętrze. **E przy drzwiach
  wzywa windę**; przy drzwiach (po lewej) wyświetlacz: piętro (P, 1) i
  strzałka jazdy.
- Jazda: ~3 s na piętro; po przyjeździe drzwi otwarte ~4 s (nie zamkną się na
  kimś w drzwiach). **E w kabinie** wybiera piętro (przy dwóch aktywnych —
  drugie); drzwi zamykają się po 1 s i jadą wszyscy w kabinie.
- **Maksymalnie 6 osób**: z większą liczbą winda nie ruszy — drzwi zostają
  otwarte, a ktoś w kabinie woła „Przeciążenie!”. Kabina jest mała (3×2
  pola), więc 6 osób stoi ciasno.
- W trakcie jazdy **widać tylko kabinę** (reszta ekranu jest wygaszona,
  kabina lekko drga).

## 10.19 Klatka schodowa i półpiętro

- Schody między parterem a piętrem 1 prowadzą przez **osobny widok klatki
  schodowej**: bieg w górę, **półpiętro** (podest), drugi bieg. Widać tylko
  klatkę i osoby na niej; przejście trwa kilka sekund.
- Przy wyjściach etykiety, dokąd prowadzą (Parter / Piętro 1 / Klatka
  schodowa).

## 10.21 Zegar, pory dnia i dni gry

- **Wspólny zegar serwera**: 1 godzina gry = 5 minut realnych. W HUD (lewy górny
  róg): „Dzień N · 09:41 · rano”; na domowym pulpicie w pasku zadań.
- **Biuro czynne 6:00–22:00.** O 22:00 wszyscy w budynku wracają do domu:
  plansza „Koniec dnia” z przepracowanym czasem i wypłatą. **Noc przewija się
  w ~1 minutę** (22:00 → 6:00).
- **Rano (6:00)** każdy zatrudniony dostaje **losową godzinę przyjazdu
  7:00–10:00**; do tego czasu plansza „Dzień N — dojazd do pracy… przyjazd o
  8:36”, potem pojawia się przed budynkiem.
- **Dni gracza**: dzień 1 = szukanie pracy (plansza „Dzień 1” nad pulpitem).
  Zatrudnienie („Jadę do biura”) = dzień 2 — w dzień od razu do biura, w
  nocy rano z losowym przyjazdem. Każdy poranek to kolejny dzień.
- **Pensja: 30 zł za godzinę gry w biurze** (liczy się czas w budynku po
  podpisaniu umowy), wypłacana o 22:00. Zaliczka 200 zł zostaje na start.
- **Oświetlenie**: świt fioletowo-chłodny, dzień biały, wieczór złoty, a
  przed 22:00 granatowy.
- Pogoda: 10.23.
- Poranny przyjazd zależy teraz od wybranego dojazdu (10.22).

## 10.22 Dojazd do pracy

- **Rano (od 6:00) wybierasz, jak jedziesz** — plansza z pięcioma
  przyciskami; wyjazd o losowej godzinie **6:15–8:45**, do tego czasu można
  zmienić zdanie (domyślnie ostatni wybór; na start tramwaj).

| Sposób | Czas | Koszt | Na potrzeby | Przyjazd |
|---|---|---|---|---|
| Pieszo | 45 min | 0 zł | energia −5, stres −3 | chodnikiem od zachodu |
| Rower | 25 min | 0 zł | energia −8, stres −5, higiena −10 | rower przy stojaku przed wejściem (zostaje do wieczora) |
| Samochód | 20 min + korki 0–20 | 12 zł | stres +5 | wjeżdża z ulicy na parking zewnętrzny i tam zostaje |
| Taksówka | 15 min | 35 zł | — | wysadza przy krawężniku i odjeżdża |
| Tramwaj | 30 min | 4,40 zł | energia −2, stres +4, higiena −3 | przystanek przy torach; tramwaj jedzie dalej |

- Bez pieniędzy na wybrany środek — **pieszo** (droższe przyciski są
  wyszarzone).
- Przyjazd **widać**: gracz siedzi w pojeździe (kamera jedzie z nim), a inni na
  zewnątrz widzą auto, taksówkę, tramwaj czy rower. Przed budynkiem jest ulica,
  torowisko z peronem i stojak na rowery.
- **Spóźnienie po 9:00**: stres +10 i „Spóźnienie… Oby nikt nie zauważył.”

## 10.23 Pogoda

- Wspólna dla serwera: **słonecznie, pochmurno, deszcz, burza, mgła**; zmienia
  się co 1–3 godziny gry z sensownymi przejściami (deszcz zwykle po chmurach,
  burza tylko z deszczu). Widać ją w zegarze HUD i rano przy wyborze dojazdu.
- **Na zewnątrz** (chodnik, ulica, parkingi, strefa palenia): krople deszczu,
  ulewa z błyskawicami, mgła, ciemniejsze niebo; w środku biura światło mniej
  się zmienia, a błyski widać przez okna.
- **Deszcz moczy**: higiena spada (~0,5 pkt/s, w burzy 2×), stres rośnie
  („Ale leje! Przemoczenie gwarantowane.”); **słońce** lekko odpręża.
- **Parasol** (sklep, stojak przy wejściu: 25 zł, mieści się w kieszeni):
  chroni przed deszczem na zewnątrz i w drodze pieszo; rozłożony widać nad
  głową.
- **Dojazd w deszczu**: pieszo bez parasola i rowerem — przemoczenie (higiena
  −12, w burzy −20; stres +5 / +8); samochód — dodatkowe korki (+10 / +20 min);
  tramwaj i taksówka bez zmian.

## 10.33 Balkon

- Na piętrze 1 przy chill roomie są drzwi na **balkon** nad wejściem do budynku
  (drewniany pomost z barierką). Balkon jest pod gołym niebem: pada deszcz,
  można palić bez czujek, dym od razu się rozwiewa.
- **Z balkonu widać, co się dzieje na dole**: chodnik, ulicę, parking
  zewnętrzny i strefę palenia (trochę przyciemnione — to niżej), razem z
  ludźmi, pojazdami i ich dymkami. Serwer dokłada do widoczności balkonu te
  pomieszczenia parteru (`below` w definicji pokoju); piętra mają wspólną
  siatkę, więc klient rysuje je tam, gdzie są.

## 10.36 Powrót z pracy do domu

- Do domu wraca się **tak, jak się przyjechało**: E przy **swoim**
  zaparkowanym aucie/rowerze albo — pieszo / tramwajem / taksówką — na
  zachodnim końcu chodnika, na przystanku tramwajowym lub na postoju taksówek
  (parter, podpowiedź „[E] Wracam do domu”).
- Pierwsze E pyta („E jeszcze raz — tak”, 5 s), drugie potwierdza: auto/rower
  odjeżdża ulicą, gracz ląduje w domu.
- **Wypłata** za przepracowane minuty (jak o 22:00), od razu przy wyjściu.
- **Dom do rana**: ekran „W domu” (w dzień ze słońcem); do biura można wrócić
  dopiero następnego ranka (zwykły dojazd). Gdy **wszyscy** gracze są w domu
  (i nikt nie jest w drodze), zegar leci w tempie nocnym.
- Nie da się wyjść w nocy ani bez umowy (kandydaci, portal).
- **„Pomiń czekanie”** na ekranie domu / wyboru dojazdu / w drodze: gdy
  poprosili o to wszyscy gracze (a wszyscy są w domu lub w drodze), zegar
  pędzi (10 min gry na tick) aż do przyjazdu do biura; inaczej przycisk
  pokazuje „Czekam na pozostałych…”.
