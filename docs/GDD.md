# Symulator pracy — Game Design Document

*Wersja robocza. Aktualizowana na bieżąco wraz z kolejnymi ustaleniami.*

## 1. Koncepcja

Gra 2D multiplayer — symulator pracy w startupie IT. Gracz zaczyna od szukania
pracy, przechodzi rekrutację, dostaje umowę i kartę dostępu, a potem wykonuje
obowiązki na swoim stanowisku. Firma rośnie razem z graczami — od małego
startupu do korporacji.

- **Grafika:** płaska, pikselowa, widok z góry — w stylu The Escapists (tylko jako inspiracja wizualna).
- **Gatunek:** symulator + social + rywalizacja.

**Filary rozgrywki:**
- **Cele i rywalizacja** — wydajność, awanse, rankingi, biurowa polityka.
- **Social i zadania** — współpraca w zespołach, wspólne przestrzenie, wydarzenia dla całego budynku.
- **Świat trwały** — umowa, stanowisko, karta, zespół i postęp zapisują się między sesjami.

## 2. Założenia techniczne

| Obszar | Decyzja |
|--------|---------|
| Platforma | Aplikacja desktopowa (Windows, macOS, Linux, Steam Deck), nie przeglądarka — ze względu na płynność i UDP |
| Klient | Godot 4 (GDScript) |
| Serwer gry | Dedykowany, autorytatywny, w Ruście (UDP, np. renet). Na etapie prototypu możliwy Godot headless |
| Backend kont i postępu | Konta, profile, umowy, stanowiska, statystyki — np. Rails (API) + baza danych |
| Dystrybucja | Steam (logowanie, znajomi, aktualizacje) |
| Hosting | VPS w Europie; jedna instancja świata („biuro”) do ~50 graczy, kilka instancji na serwer |
| Skala | Do ~50 graczy jednocześnie w jednym świecie, w tym wszyscy naraz w jednym pomieszczeniu |

### Założenia sieciowe
- Klient wysyła tylko inputy, serwer liczy stan (ochrona przed cheatami).
- Tick serwera ~20 Hz, niezależny od FPS klienta.
- Predykcja ruchu własnej postaci, interpolacja pozostałych graczy.
- Binarny protokół, wysyłanie tylko zmian stanu (delta).
- Interest management po pomieszczeniach — gracz dostaje aktualizacje głównie o osobach w swoim pokoju.

## 3. Świat — budynek firmy

### Na zewnątrz
- Parking zewnętrzny
- Strefa palenia — jedyne miejsce, gdzie palenie jest bez konsekwencji

### Parter
- Wejście z portiernią — bramki na kartę; portier (NPC) wpuszcza osoby bez karty
- Parking wewnętrzny
- Sklep — zakupy (np. kawa, przekąski, papierosy)
- Winda — dwa piętra do wyboru, na początku aktywne tylko jedno
- Schody — alternatywa dla windy

### Piętro 1 (aktywne od startu)
- Recepcja przy wejściu na piętro
- Pokój działu IT / Produkt
- Pokój działu Biznesu (marketing + sprzedaż)
- Pokój Zarządu
- Pokój HR
- Korytarz
- Chill room — wspólna przestrzeń dla wszystkich
- Łazienka damska i męska

Gracze mogą swobodnie chodzić po korytarzu, pokojach i wspólnych przestrzeniach.

### Piętro 2 (zablokowane)
Odblokowywane wraz z rozwojem firmy (patrz sekcja 6).

## 4. Ścieżka nowego gracza
1. Portal z ogłoszeniami o pracę — gra zaczyna się od widoku strony z ofertami.
2. Wybór stanowiska i aplikacja.
3. Rekrutacja — pytania zależne od stanowiska.
4. Dzień próbny — gracz nie ma karty, więc portier wprowadza go do budynku i odprowadza na recepcję.
5. Podpisanie umowy (w HR).
6. Otrzymanie karty dostępu — od tej pory swobodne wejście do budynku.
7. Przydział do działu / zespołu i rozpoczęcie właściwej pracy.

**Do ustalenia:** forma rekrutacji: quiz / minigry zadaniowe / rozmowa z NPC
napędzana AI (propozycja: na start quizy i minigry, AI później).

## 5. Struktura firmy — start (startup)

| Dział | Kto | Uwagi |
|-------|-----|-------|
| IT / Produkt | Gracze — programiści | Na start bez podziału na backend/frontend |
| Biznes | Gracze — marketing + sprzedaż | Zadania powiązane z IT (klienci, potrzeby produktu) |
| Zarząd | NPC | Wyznacza cele firmy, decyduje o awansach |
| HR | NPC | Rekrutacja, dzień próbny, umowy, później konflikty i skargi |

Zespoły na start = działy (każdy dział ma swój pokój).

**NPC na start:** portier; recepcjonista/recepcjonistka; Zarząd (CEO / założyciele); HR.

## 6. Rozwój firmy (wspólny cel serwera)

Praca graczy przynosi firmie przychody, a firma odblokowuje kolejne etapy:

| Etap | Co się odblokowuje |
|------|--------------------|
| Startup | IT, Biznes, Zarząd, HR — tylko piętro 1 |
| Scale-up | Podział IT na backend / frontend / mobile, osobne działy marketingu i sprzedaży, DevOps |
| Korporacja | Piętro 2, dział data science / AI, sala konferencyjna na eventy dla wszystkich |

Docelowo role w Zarządzie i HR mogą stać się dostępne dla graczy (awanse).

## 7. Mechaniki

### Palenie
- Palenie na zewnątrz w strefie palenia — bez konsekwencji.
- Palenie w środku — zapach rozchodzi się po pomieszczeniach.
- W niektórych miejscach czujniki włączają alarm przeciwpożarowy → ewakuacja budynku (naturalny event dla wszystkich graczy).

### Sklep i ekonomia
- Zakupy w sklepie na parterze.
- Do ustalenia: pensja, ceny, wpływ zakupów na postać (np. energia, stres).

### Pomysły do rozważenia (nieprzesądzone)
- Punkty wydajności i ranking (np. „pracownik dnia”).
- Biurowa polityka: przypisywanie sobie cudzych zadań, plotki, reputacja — z ryzykiem przyłapania.
- Zadania wymagające współpracy kilku osób.
- Czat głosowy / tekstowy zależny od zasięgu (słyszysz osoby w tym samym pokoju).
- Wydarzenia dla całego budynku: zebranie firmowe, awaria prądu, kontrola, alarm pożarowy.
- Kto płaci za fałszywy alarm pożarowy.
- Personalizacja postaci i biurka.

## 8. Otwarte kwestie
- [ ] Obowiązki i zadania na poszczególnych stanowiskach (IT, Biznes)
- [ ] Przebieg dnia pracy i czas gry vs czas rzeczywisty
- [ ] Forma i treść rekrutacji dla każdego stanowiska
- [ ] Ekonomia: pensja, sklep, statystyki postaci
- [ ] Zasady awansów i progresji gracza
- [ ] Warunki przejścia firmy do kolejnego etapu rozwoju
- [ ] Mechanika zespołów po rozrośnięciu się firmy (zespoły działowe czy mieszane)
- [ ] Nazwa gry i nazwa firmy

## 9. Proponowany zakres MVP
- Parter + piętro 1
- Dwa działy (IT, Biznes) + NPC: portier, recepcja, Zarząd, HR
- Uproszczona rekrutacja
- Jedno–dwa zadania na dział
- Multiplayer: ruch, pomieszczenia, synchronizacja do ~50 graczy
- Palenie + alarm jako pierwsza mechanika systemowa

---

## 10. Implementacja — ustalenia i stan

Sekcja techniczna prowadzona przez zespół; sekcje 1–9 to design.

### 10.1 Rozbieżności między GDD a obecnym kodem (do decyzji)

| Temat | GDD | Obecnie w kodzie |
|-------|-----|------------------|
| Snapshoty | tylko zmiany stanu (delta) | pełne snapshoty; w `Input` jest `ack_tick` przygotowany pod deltę — przy 50 graczach w pokoju to ~12 KB/s na klienta |
| Transport | „np. renet” | własny protokół na UDP (decyzja z briefu etapu 1: klient w GDScript nie obsłuży renet) |
| Platformy | desktop: Windows, macOS, Linux, Steam Deck | w rozmowie 2026-09-26 rozszerzone o Android/iOS z crossplayem („później”); konsole „może kiedyś”. Obecny priorytet: **macOS** |
| Konta / tożsamość | Steam + backend kont (Rails) | nick + token sesji, bez kont i zapisu postępu |
| Przydział do działu | po umowie przydział do działu / zespołu | HR wydaje kartę bez działu (decyzja 2026-09-26); przydział dojdzie z rekrutacją |
| Trwałość | umowa, karta, stanowisko zapisują się między sesjami | przepustka i karta żyją do końca sesji (brak kont) |

Rozwiązane 2026-09-26: układ parteru i piętra 1 zgodny z sekcją 3 (10.5);
bramki na kartę działają, portier wpuszcza i odprowadza osoby bez karty,
recepcja prowadzi do HR, HR wydaje kartę pracownika (10.7).

### 10.2 Stack (zaimplementowany)
- Klient: Godot 4.7, GDScript.
- Serwer: dedykowany, autorytatywny, Rust (`std::net::UdpSocket` + `socket2`, jeden wątek).
- Transport: UDP, własny binarny protokół (`docs/PROTOCOL.md`), IPv4 + IPv6.

### 10.3 Platformy i crossplay
Decyzja (2026-09-26): zostajemy przy Godocie; Unity rozważone i odrzucone.
Konsole (Switch, Xbox, PlayStation) — „może kiedyś”, przez firmę portującą
(np. W4 Games); wtedy dojdą konta platform, certyfikacja i wymogi crossplay.
Serwer i protokół nie zależą od platformy.

Przygotowane już pod platformy mobilne (gdyby wróciły do planu):
- ✅ IPv6 po obu stronach; ✅ sesja po tokenie (zmiana sieci); ✅ automatyczne ponowne łączenie.
- ⬜ Sterowanie dotykowe, skalowanie UI, eksport Android/iOS, konta sklepów.

### 10.4 Etap 1 — pionowy wycinek sieci

Fundament multiplayera: wielu graczy chodzi po wspólnej mapie parteru.
Bez rekrutacji, sklepu, palenia, NPC i zadań.

Zakres:
1. Serwer: stały tick 20 Hz; handshake connect → player_id → spawn; timeout
   5 s; klient wysyła tylko inputy, serwer liczy pozycje i kolizje; snapshoty
   tylko z graczami w tym samym pomieszczeniu; pakiety binarne ≤ ~1200 B.
2. Mapa parteru w siatce kafelków, jeden format danych dla serwera i klienta.
3. Klient: ekran startowy (nick, adres, „Połącz”), placeholderowe kafelki,
   predykcja + rekoncyliacja, interpolacja innych (~100 ms), nick nad
   postacią, kamera, overlay F3.
4. Boty: N wirtualnych klientów chodzących losowo, część w jednym pokoju.

Kryteria akceptacji: `cargo run` startuje serwer, kilka klientów łączy się
lokalnie; własny ruch natychmiastowy, inni płynni; 50 botów w jednym pokoju →
klient 60 FPS, serwer bez zgubionych ticków (logi czasu ticka i transferu);
lag ~100 ms + 2% strat → nadal płynnie; zmiana pokoju zmienia widocznych
graczy; testy jednostkowe serializacji i ruchu/kolizji.

### 10.5 Budynek (mapy)

Obie kondygnacje 60×48 kafli po 16 px; pliki `client/maps/building.json`,
`floor0.json`, `floor1.json` generuje `tools/build_maps.py`. Piętro 2 jest w
`building.json` jako zablokowane (bez pliku).

**Parter + teren zewnętrzny** — gracz startuje na chodniku przed wejściem.

```
FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
FvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fv###############################################vvvvvvvvvvF
Fv#================#####################::::::::#vvvvvvvvvvF
Fv#================#####################:TTTTTT:#vvvvvvvvvvF
Fv#==XXX======XXX==#####################::::::::#vvvvvvvvvvF
Fv#==XXX======XXX==############.SSSSSS.#::::::::#vvvvvvvvvvF
Fv#================############.SSSSSS.#::::::::#vvvvvvvvvvF
Fv#================#####eeeee##........#:TT:::::#vvvvvvvvvvF
Fv#================#####eeeee##........#:TT:::::#vvvvvvvvvvF
Fv#================#####eeeee##........#:TT:::::#vvvvvvvvvvF
Fv#==XXX======XXX==#####eeeee##........#:TT:::::#vvvvvvvvvvF
Fv#==XXX======XXX==#####eeeee##........#::::::::#vvvvvvvvvvF
Fv#================######EEE######DD#######LL#############vF
Fv#================#.....................................#vF
Fv#================#.....................................#vF
Fv#================D.....................................#vF
Fv#==XXX======XXX==D.....................................#vF
Fv#==XXX======XXX==#.....................................#vF
Fv#================#.....................................#vF
Fv#================#.....................................#vF
Fv#================#########B#B#B#B#B#B###################vF
Fv#================#......#______________#...............#vF
Fv#==XXX======XXX==#......#______________#...............#vF
Fv#==XXX======XXX==#......#______________#...TTTTTTTTTT..#vF
Fv#================#......#______________#...............#vF
Fv#================#......#______________#...............#vF
Fv#================#.T....#______________#...TTTTTTTTTT..#vF
Fv#================#.T....D______________#...............#vF
Fv#==XXX======XXX==#.T....D______________D...............#vF
Fv#==XXX======XXX==#.T....#______________D...............#vF
Fv#================#......#______________#.........TTTTT.#vF
Fv#================#......#______________#...............#vF
Fv######ggggg##################GGGGG######################vF
Fpppppp=======pppppppppppppppppppppppppppppppppppppppppppppF
Fpppppp=======pppppppppppppppppppppppppppppppppppppppppppppF
Fpppppp=======pppppppppppppppppppppppppppppppppppppppppppppF
Fvvvvvv=======vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fvv============================vvvvvvvvvvvvvvvvvvvvvvvvvvvvF
Fvv=XXX==XXX==XXX==XXX==XXX====vvvvvvvvvvvzzzzzzzzzzzzzvvvvF
Fvv=XXX==XXX==XXX==XXX==XXX====vvvvvvvvvvvzzzzzzzzzzzzzvvvvF
Fvv============================vvvvvvvvvvvzzTTTzzzzzzzzvvvvF
Fvv============================vvvvvvvvvvvzzzzzzzzzzzzzvvvvF
Fvv=XXX==XXX==XXX==XXX==XXX====vvvvvvvvvvvzzzzzzzzTzzzzvvvvF
Fvv=XXX==XXX==XXX==XXX==XXX====vvvvvvvvvvvzzzzzzzzzzzzzvvvvF
Fvv============================vvvvvvvvvvvzzzzzzzzzzzzzvvvvF
FvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvF
FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
```

**Piętro 1** (dolna część to pustka poza obrysem budynku)

```
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
~~###############################################~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,#####################,,,,,,,,#~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,#####################,,,,,,,,#~~~~~~~~~~~
~~#,,TTTTT,,TTTTT,,#####################,,,,,,,,#~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,############.SSSSSS.#,,TTTT,,#~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,############.SSSSSS.#,,,,,,,,#~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,#####eeeee##........#,,,,,,,,#~~~~~~~~~~~
~~#,,TTTTT,,TTTTT,,#####eeeee##........#,,,,,,,,#~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,#####eeeee##........#,,,,,,,,#~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,#####eeeee##........#,,,,,,,,#~~~~~~~~~~~
~~#,,,,,,,,,,,,,,,,#####eeeee##........#,,,,,,,,#~~~~~~~~~~~
~~#,,TTTTT,,TTTTT,,######EEE######DD#######DD#############~~
~~#,,,,,,,,,,,,,,,,#.........................#,,,,,,,,,,,#~~
~~#,,,,,,,,,,,,,,,,#.........................#,,,,,,,,,,,#~~
~~#,,,,,,,,,,,,,,,,#.........................#,,,TTTTT,,,#~~
~~#,,TTTTT,,TTTTT,,#..........TTTTT..........#,,,TTTTT,,,#~~
~~#,,,,,,,,,,,,,,,,#.........................#,,,,,,,,,,,#~~
~~#,,,,,,,,,,,,,,,,#.........................#,,,,,,,,,,,#~~
~~#,,,,,,,,,,,,,,,,#.........................#,,,,,,,,,,,#~~
~~########DD##################DDDDD################DD#####~~
~~#......................................................#~~
~~#......................................................#~~
~~#......................................................#~~
~~##########DD##################DDD###########D######D####~~
~~#,,,,,,,,,,,,,,,,,,,,#,,,,,,,,,,,,,,,,,,,#::::::#::::::#~~
~~#,,,,,,,,,,,,,,,,,,,,#,,,,,,,,,,,,,,,,,,,#::::::#::::::#~~
~~#,,TTTTT,,,TTTTT,,,,,#,,,TTT,,,,,,,,,,,,,#::::::#::::::#~~
~~#,,,,,,,,,,,,,,,,,,,,#,,,TTT,,,,,,,,,,,,,#::::::#::::::#~~
~~#,,,,,,,,,,,,,,,,,,,,#,,,,,,,,,,,,TTTT,,,#::::::#::::::#~~
~~#,,TTTTT,,,TTTTT,,,,,#,,,,,,,,,,,,,,,,,,,#::::::#::::::#~~
~~#,,,,,,,,,,,,,,,,,,,,#,,,,,,,,,,,,,,,,,,,#::::::#::::::#~~
~~########################################################~~
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
```

Legenda: `#` ściana, `.` podłoga, `,` wykładzina, `:` płytki, `_` posadzka holu,
`=` parking, `v` trawa, `p` chodnik, `z` strefa palenia, `F` ogrodzenie,
`~` pustka, `D` drzwi, `G` szklane drzwi wejściowe, `B` bramka na kartę
(otwarta), `L` drzwi zamknięte (zaplecze), `g` brama garażowa, `E` drzwi windy,
`e` kabina windy, `S` biegi schodów, `T` meble / regały / lady, `X` samochody.
Bramki (`B`) i brama garażowa (`g`) wymagają przepustki lub karty przy
wejściu, wyjście jest wolne; drzwi zaplecza (`L`) — uprawnień obsługi.

| Piętro | Pomieszczenia (id) |
|--------|--------------------|
| Parter | Na zewnątrz (1), Parking wewnętrzny (2), Hol (3), Portiernia (4), Wejście (5), Sklep (6), Parking zewnętrzny (7), Strefa palenia (8), Zaplecze techniczne (9, zamknięte), Winda (20), Klatka schodowa (21) |
| Piętro 1 | Recepcja (1), Zarząd (2), IT / Produkt (3), HR (4), Korytarz (5), Biznes (6), Chill room (7), Łazienka damska (8), Łazienka męska (9), Winda (20), Klatka schodowa (21) |

Ustalenia: sklep i portiernia są przed bramkami (dostępne bez karty); parking
wewnętrzny duży, z bramą garażową na zewnątrz (też na kartę); wolne miejsce nad
holem to zamknięte „Zaplecze techniczne”.

**Poruszanie między piętrami:** schody — wejście na biegi schodów przenosi na
drugie piętro (bez odbijania, gdy trzymasz klawisz); winda — w kabinie
klawisz **E** jedzie na następne aktywne piętro (podpowiedź na ekranie).

### 10.7 Wdrożenie: portier, recepcja, HR (dzień próbny → karta)

Ustalenia 2026-09-26: dopóki nie ma HR i umowy, dostęp za bramki daje
**przepustka gościa od portiera**, ważna do końca sesji; portier **odprowadza**
na recepcję; **wyjście przez bramki jest wolne**.

Przebieg: gracz startuje przed budynkiem bez przepustki → bramki w holu go
zatrzymują (podpowiedź: „porozmawiaj z portierem”) → przy portierni wciska E →
portier: „Dzień dobry! Pierwszy dzień? Zaprowadzę na recepcję — proszę za mną.”,
gracz dostaje przepustkę → portier idzie przez bramki i schodami na recepcję
piętra 1, czekając na gracza, gdy ten zostaje w tyle („Proszę za mną!”) → na
recepcji: „To recepcja — tutaj proszę się zgłosić. Przepustka gościa jest ważna
do końca dnia.” → portier wraca na portiernię. Jeśli gracz nie idzie za nim
przez 30 s, portier rezygnuje i odbiera przepustkę. Prowadzi jedną osobę naraz
(„Chwileczkę, właśnie kogoś prowadzę.”); osobom z przepustką mówi, że mogą iść.

Ciąg dalszy (ustalenia 2026-09-26: recepcja **odprowadza** do HR; HR wydaje
**kartę bez działu**; przydział do działu na razie nic nie zmienia): na
recepcji gracz wciska E → „Witamy! Zaprowadzę do HR — tam podpisuje się
umowę.” → recepcja idzie do pokoju HR (czeka i przypomina jak portier; po
30 s wraca, nie odbierając przepustki) → „To dział HR — tutaj podpisuje się
umowę i odbiera kartę.” → gracz wciska E przy HR → „Umowa podpisana — witamy
w firmie! Oto karta pracownika.” — **karta pracownika zastępuje przepustkę
gościa**. Osoby z kartą recepcja i HR tylko witają; osoby bez przepustki
odsyłają na portiernię.

NPC: Portier (mundur z czapką), Recepcja i HR (koszula z krawatem) — jako
neutralne nazwy stanowisk. Wciśnięcie E trafia najpierw do NPC stojącego na
swoim stanowisku (portier, który właśnie przyprowadził gościa pod ladę, nie
zasłania recepcji).

### 10.6 Stan implementacji

*Stan na 2026-09-26 — etap 1 (sieć) ukończony; dodane IPv6, sesje po tokenie,
automatyczne ponowne łączenie, budynek wg GDD (parter z terenem zewnętrznym,
piętro 1, schody, winda), uprawnienia (bramki) oraz wdrożenie nowego gracza:
portier → recepcja → HR → karta pracownika.*

#### Zrobione
- **Serwer Rust** (`server/`): tick 20 Hz bez dryfu z liczeniem zgubionych
  ticków; handshake z `nonce`/`token`, odrzucenia (pełny serwer, wersja,
  nick), timeout 5 s; kolejka inputów z limitem 6/tick; kolizje ze ścianami i
  meblami; snapshoty z interest management po `(piętro, pokój)`,
  fragmentowane ≤ 1200 B; nicki przez `PlayerInfo`/`InfoRequest`; ping;
  statystyki co 5 s; symulator `--lag-ms/--jitter-ms/--loss`.
- **Budynek**: parter (z parkingiem zewnętrznym i strefą palenia) + piętro 1
  wg sekcji 3, piętro 2 zablokowane; schody i winda (E) jako część
  deterministycznej symulacji, przewidywane przez klienta; JSON-y wspólne dla
  serwera i klienta, weryfikowane jednym CRC32 budynku (protokół v2).
- **Klient Godot** (`client/`): ekran startowy, mapa z kolorowych kafli i
  podpisów pomieszczeń, predykcja + rekoncyliacja z wygładzaniem korekt,
  interpolacja innych graczy (100 ms), nicki, kamera, overlay F3.
- **Wdrożenie i uprawnienia**: bramki i brama garażowa na przepustkę/kartę z
  wolnym wyjściem, zamknięte zaplecze; NPC serwera: portier (przepustka
  gościa, odprowadza na recepcję), recepcja (odprowadza do HR), HR (umowa →
  karta pracownika); dymki wypowiedzi, podpowiedzi, wygląd NPC; protokół v3
  (uprawnienia w snapshocie, pakiet `Say`, encje NPC).
- **Boty** (`cargo run --release --bin bots`): 50 domyślnie, chodzą po BFS po
  całym budynku (schodami), część zbiera się w wybranym pokoju (domyślnie
  Chill room na piętrze 1).
- **Sieć mobilna**: serwer dual-stack IPv4/IPv6; gracz identyfikowany tokenem
  (zmiana adresu w trakcie gry przenosi sesję); klient przepina gniazdo po
  ciszy/powrocie z tła i sam łączy się ponownie po utracie sesji.
- **Testy**: 41 jednostkowych w Rust (budynek i pokoje wg GDD, osiągalność
  zależna od uprawnień, bramki, ruch/kolizje, schody, winda, nawigacja,
  portier, recepcja, HR, protokół), 2 golden, 7 e2e serwera (m.in. całe
  wdrożenie przez sieć aż do karty, niewidoczność między piętrami, zgodność
  stanu serwera z predykcją) — łącznie 50; 107 sprawdzeń w Godocie (parytet
  protokołu i ruchu z bramkami i piętrami, parsowanie adresów).

#### Pomiary (MacBook, wszystko lokalnie)
| scenariusz | wynik |
|------------|-------|
| 50 botów w jednym pokoju | serwer: 0 zgubionych ticków, tick śr. ~1,2 ms (max ~2,6 ms), ~12 KB/s na klienta |
| klient + 50 botów w tym samym pokoju | 60 FPS (vsync), 50 widocznych, 0 korekt predykcji |
| RTT ~117 ms, jitter 10 ms, 2% strat | 60 FPS, 0 korekt, bufor interpolacji pusty w 0,40% klatek |
| RTT ~226 ms, jitter 20 ms, 2% strat | 60 FPS, 0 korekt, bufor pusty w 0,25% klatek |
| przejście z Wejścia do Korytarza (stara mapa) | widoczni: 42 → 5 |
| 40 botów po całym budynku, połowa w Chill roomie (piętro 1) | serwer: 0 zgubionych ticków, tick śr. ~1,1 ms; boty: 0 błędnych predykcji mimo schodów |
| klient w recepcji piętra 1, 26 widocznych | 60 FPS, 0 korekt, bufor interpolacji pusty w 0,00% klatek |
| gość bez przepustki: bramka → rozmowa z portierem → schody → recepcja | zatrzymany na bramce, przepustka po rozmowie, portier doprowadza na recepcję; 0 korekt |
| pełne wdrożenie w oknie klienta: portier → recepcja → HR | karta pracownika w ~35 s gry, 0 korekt, 60 FPS |
| 50 botów z kartą (`--start-with-card`) w Chill roomie | 50/50 dochodzi przez bramki i schody, 0 zgubionych ticków, 0 błędnych predykcji |
| klient IPv6 + klient IPv4, serwer zamrożony na 3 s | obie sesje zachowane (nowe porty, te same id) |
| restart serwera | obaj klienci połączeni ponownie automatycznie w < 1 s od startu serwera |

#### Znane ograniczenia
- Brak kolizji między graczami (celowo — to biuro, nie bijatyka).
- Ping w F3 ma rozdzielczość klatki (~16 ms), bo `Pong` jest czytany w `_process`.
- Eksport klienta: przy eksporcie trzeba dodać `*.json` do filtra zasobów
  nie-Godotowych, inaczej `maps/*.json` nie trafią do paczki.
- Kilka okien klienta naraz na jednym Macu: macOS spowalnia zasłonięte okna,
  więc ich metryki płynności (F3) są wtedy zaniżone — to nie błąd gry.
- Przepustka i karta znikają po rozłączeniu (brak kont i zapisu postępu) —
  po każdym połączeniu wdrożenie trzeba przejść od nowa.
- Boty bez `--start-with-card` zostają w strefie publicznej (nie rozmawiają z
  portierem).

#### Następne kroki (propozycja)
Zgodnie z MVP (sekcja 9): uproszczona rekrutacja (portal ofert + pytania) z
przydziałem do działu, NPC Zarządu, zadania działów, palenie + alarm; do tego
trwałość postępu (konta). Punkty
wpięcia opisane w `docs/ARCHITECTURE.md` („Gotowość na rozbudowę”).
