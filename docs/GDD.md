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

## 9a. Rozszerzenia (ustalenia 2026-09-26)

Ścieżka nowego gracza, doprecyzowana:
1. **Tworzenie postaci**: imię, płeć, wiek, miejscowość, e-mail (dane
   *postaci*, fikcyjne — widzi je tylko serwer i sam gracz, np. w CV; inni
   widzą imię i wygląd) + wygląd (fryzura, kolory skóry, włosów, ubrań).
2. **Pulpit komputera** → przeglądarka → **portal z ogłoszeniami**: kilka
   fikcyjnych firm i stanowisk. Zatrudnia tylko nasz startup (Programista/ka,
   Designer/ka — IT / Produkt; Sprzedaż, Marketing — Biznes); inne firmy
   odpowiadają zabawną odmową albo milczą.
3. Formularz zgłoszeniowy → po chwili **wiadomość z zaproszeniem na rozmowę**
   → **rozmowa online** (pytania z humorystycznymi odpowiedziami) → zaproszenie
   na dzień próbny.
4. Dzień próbny w biurze; w HR: **karta dostępu i własny komputer**.

Nowe mechaniki:
- **Ekwipunek**: na start małe kieszenie; przedmioty można oglądać, używać,
  wyciągać/odkładać i przekazywać innym. **Karta dostępu i przepustka to
  przedmioty** — bramki otwierają się temu, kto ma je przy sobie (można je
  przekazać lub zgubić).
- **Komputer**: wyciągnięty z ekwipunku i położony na biurku; można go
  **zablokować**. Niezablokowanego może użyć ktoś inny pod nieobecność
  właściciela — np. napisać coś w jego imieniu. Pierwsza aplikacja:
  **firmowy komunikator** dla wszystkich.
- **Statystyki postaci** na ekranie: **głód, energia, stres, potrzeba
  toalety**; zmieniają się z czasem, przywracają je jedzenie, kawa, odpoczynek
  (sofa), przerwa/palenie, toaleta.

Kolejność realizacji: (1) tworzenie postaci → (2) pulpit, portal, rozmowa →
(3) ekwipunek i karta jako przedmiot → (4) komputer i komunikator →
(5) statystyki.

## 9b. Backlog (pomysły 2026-09-26, do realizacji po kolei)

**Zrobione:** sklep na parterze (półki + kasa, złotówki), zaliczka 200 zł przy
podpisaniu umowy, kanapki, przekąski, napoje, fast food, alkohol, papierosy
(potrzebne do palenia) — zob. 10.20.

**Czas i dni** — *zrobione (10.21, pogoda 10.23)*
- Zegar gry (aktualna godzina na ekranie), pory dnia (światło), **zmienna
  pogoda** na zewnątrz.
- Rozgrywka podzielona na **dni**: pierwszy dzień — pełnoekranowa plansza
  „Dzień 1”, szukanie pracy (portal, aplikacja); po zatrudnieniu „Dzień 2” —
  start w pracy rano o losowej godzinie między 7:00 a 10:00.
- Pensja wypłacana za dzień pracy (zastąpi jednorazową zaliczkę).

**Dojazd do pracy** — *zrobione (10.22)*. Wybór: pieszo, rowerem, samochodem (parking), taksówką,
tramwajem (bilet/taksówka kosztują).

**Zarząd i kalendarz** — *zrobione (10.24)*. Do pokoju zarządu nie można wejść bez spotkania;
spotkanie umawia się w kalendarzu (aplikacja na komputerze).

**Rekrutacja i rozwój firmy** — *wakaty i obsadzone stanowiska zrobione (10.27)*
- Na starcie **mało ogłoszeń** (to start firmy); przybywa ich z rozwojem.
- Stanowisko obsadzone przez jednego gracza **znika** dla innych (nie można
  aplikować na zajęte miejsce).

**Chill room** — *zrobione (10.25)*. Oprócz owoców i kawy **losowo pojawiające się ciastka /
słodycze** w ograniczonej ilości (teraz decyduje NPC/serwer, w przyszłości
gracze).

**Obiady** — *zrobione (10.26)*. Aplikacja na komputerze do **zamawiania obiadu** w trakcie pracy
(dostawa do biura).

**Panel założyciela** — *zrobione bez płatności (10.28)*.

**Model biznesowy (przyszłość)**
- Gra **darmowa** dla graczy.
- Każda firma = **osobny serwer / instancja gry**. Założenie firmy (własna
  instancja) i wystawianie ogłoszeń o pracę jest **płatne**.
- Założyciel wybiera nazwę firmy, jest w zarządzie, ma **panel ogłoszeń**
  (wystawia oferty, przegląda aplikacje, zatrudnia).
- Później **mikropłatności**: doładowanie portfela w grze (sklep, bilet,
  taksówka). Wymaga osobnego projektu (płatności, konta, regulaminy) — nie
  robimy tego w ramach prototypu.

Proponowana kolejność: sklep i pieniądze → zegar, pory dnia, dni gry i
pensja dzienna → dojazd do pracy → pogoda → kalendarz i zarząd → słodycze w
chill roomie → zamawianie obiadów → mniej ogłoszeń na start i obsadzone
stanowiska → panel założyciela firmy (i dalej: instancje / płatności).

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
| Skutki działu | zespoły, zadania działów | dział jest przypisany i widoczny, ale na razie nic nie zmienia (decyzja 2026-09-26) |
| Rekrutacja AI | później rozmowa z NPC napędzana AI | quiz (sekcja 10.8) |
| Trwałość | umowa, karta, stanowisko zapisują się między sesjami | przepustka i karta żyją do końca sesji (brak kont) |

Rozwiązane 2026-09-26: układ parteru i piętra 1 zgodny z sekcją 3 (10.5);
bramki na kartę działają, portier wpuszcza i odprowadza osoby bez karty,
recepcja prowadzi do HR, HR wydaje kartę pracownika (10.7); portal z ofertami
i rekrutacja z przydziałem do działu (10.8).

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

**Poruszanie między piętrami:** schody — wejście na biegi schodów przenosi do
klatki schodowej (osobny widok: bieg, półpiętro, drugi bieg), a jej koniec
na drugie piętro; winda — trzeba ją wezwać (E przy drzwiach), poczekać, wejść
i wybrać piętro (E w kabinie) — szczegóły w 10.18.

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

### 10.8 Portal z ofertami i rekrutacja

Ustalenia 2026-09-26: **portal na starcie** (zgodnie z sekcją 4), quiz
**3 pytania, 2 poprawne = przyjęcie**, ponowna próba od razu (inne pytania)
lub inna oferta; pytania **humorystyczne**, w klimacie startupu, z jedną
poprawną odpowiedzią.

- Po połączeniu gracz widzi „Portal z ofertami pracy · Startup Sim sp. z o.o.”
  z dwiema ofertami: **Programista/ka** (dział IT / Produkt) i **Marketing i
  sprzedaż** (dział Biznes). Nazwa firmy to zaślepka (otwarta kwestia z sekcji 8).
- „Aplikuj” → 3 losowe pytania z puli oferty (po 8 w puli), odpowiedzi w losowej
  kolejności → wynik: przyjęcie („zapraszamy na dzień próbny”) albo „Tym razem
  się nie udało” z możliwością ponownej próby.
- Po przyjęciu gracz pojawia się przed budynkiem i przechodzi wdrożenie (10.7);
  HR podpisuje umowę **na dział z rekrutacji**: „Umowa podpisana — witamy w
  dziale IT / Produkt! Oto karta pracownika.” Od tej chwili inni widzą przy
  nicku dział („Ala · IT”).
- Pytania i oferty są w `server/data/recruitment.json` (edycja bez zmiany kodu;
  pierwsza odpowiedź w pliku jest poprawna — gra ją tasuje). Ocenia serwer.

### 10.9 Oprawa graficzna (placeholder → pixel art)

Ustalenia 2026-09-26: grafika **rysowana proceduralnie w kodzie** (bez
zewnętrznych pakietów i licencji), jeden spójny przeskok: otoczenie, meble i
postacie naraz.

- Kafle 16 px, kamera 3×; ściany w rzucie 3/4 z frontem, listwą i obrazkami;
  podłogi z fakturą: deski/płytki biurowe, niebieska wykładzina w działach,
  płytki w łazienkach, kamienna posadzka holu, asfalt z liniami miejsc, kostka
  chodnika, trawa z kwiatkami, żwir strefy palenia, żywopłot.
- Meble: biurka z monitorami i krzesłami (IT, Biznes, HR), lady (portiernia,
  recepcja, kasa sklepu), regały z towarem, sofa i stolik (chill room), stół
  Zarządu z krzesłami, ekspres do kawy i aneks kuchenny z owocami (chill
  room), rośliny, szafy serwerowe (zaplecze), toalety i
  umywalki, ławka i popielniczka (strefa palenia), samochody w kolorach.
- Postacie: fryzura, kolor skóry i ubrań losowane z id gracza, 4 kierunki,
  animacja chodu; portier w mundurze z czapką, recepcja i HR w koszulach z
  krawatem.
- Przy okazji: portiera widać z holu wejściowego przez drzwi portierni
  (pokoje mogą „widzieć” inne pokoje — ustawienie w mapie).

### 10.10 Ekspres do kawy (chill room)

Ustalenie 2026-09-26: ekspres działa jako **czynność**, bez wpływu na
statystyki — co daje kawa (energia, stres, koszt), zostaje otwartą kwestią
ekonomii (sekcja 7). Przy ekspresie: „[E] Zrób kawę” → „Parzę kawę…” (3 s,
ekspres zajęty dla innych: „Ekspres zajęty — chwilka.”) → „Kawa gotowa!” →
kubek w ręce przez 90 s, widoczny dla innych → „Kawa wypita.”

### 10.11 Tworzenie postaci (etap 1 z 9a)

Ekran startowy to tworzenie postaci: imię, płeć (kobieta / mężczyzna / inna),
wiek (18–70), miejscowość, e-mail postaci oraz wygląd — kolor skóry (4),
fryzura (6: krótkie, długie, kok, jeżyk, kucyk, łysa głowa), kolor włosów (7),
koszula (10), spodnie (5) — z podglądem na żywo („Obróć”, „Losuj wygląd”).
Serwer sprawdza dane; innym graczom pokazuje tylko imię, płeć i wygląd.
Ostatnia postać jest zapamiętywana lokalnie.

### 10.12 Pulpit, portal, poczta i rozmowa online (etap 2 z 9a)

Zastępuje prosty portal z 10.8. Po połączeniu gracz widzi pulpit komputera
(„StartOS”): Przeglądarka, Poczta, Kosz, pasek zadań z zegarem.
- **Przeglądarka** → portal „praca.example”: nasz startup szuka na 4
  stanowiska (Programista/ka, Designer/ka — IT / Produkt; Specjalista/ka ds.
  sprzedaży, ds. marketingu — Biznes) + 4 fikcyjne firmy (Korpo-Bank S.A.,
  Mega Software Inc., Pizzeria u Stefana, Agencja Kreatywna BUZZ) z
  humorystycznymi ogłoszeniami. Formularz: dane postaci, „Dlaczego chcesz u
  nas pracować?”, zgoda na przetwarzanie danych.
- **Poczta**: po ~4 s zaproszenie na rozmowę (albo zabawna odmowa od innej
  firmy — lub cisza); powiadomienie i licznik nieprzeczytanych.
- **Rozmowa online**: okno wideorozmowy (Kasia z HR i Twoja postać), 3
  pytania z puli stanowiska (po 6–8, humorystyczne odpowiedzi), wynik → mail:
  zaproszenie na dzień próbny z przyciskiem „Idę do biura” albo podziękowanie
  (można aplikować ponownie).

### 10.13 Ekwipunek (etap 3 z 9a)

- **Kieszenie (3)** na małe przedmioty i **ręce** na jeden dowolny (duże —
  laptop, kawa — tylko w rękach). Pasek ekwipunku w prawym dolnym rogu.
- Przedmioty: **przepustka gościa** (portier), **karta pracownika** z imieniem
  i działem oraz **laptop** (HR — wymaga wolnych rąk), **kawa** (ekspres).
- Klawisze: **1–3** wyjmij / schowaj, **Q** upuść, **G** podaj osobie obok,
  **F** użyj (wypij kawę, pokaż kartę), **E** podnieś z podłogi.
- **Dostęp wynika z tego, co masz przy sobie**: kartę można upuścić, podnieść
  lub komuś dać — dostęp idzie razem z nią. Przedmiot w rękach i leżący na
  podłodze widzą wszyscy.
- Laptop można na razie tylko nosić — kładzenie na biurku i używanie to etap 4.

### 10.14 Komputer i komunikator (etap 4 z 9a)

- **Hot-desking**: laptop od HR kładziesz (E, laptop w rękach) na dowolnym
  wolnym biurku **w pokoju swojego działu**; biurka działów nie mają już
  stałych monitorów. E przy biurku z laptopem otwiera jego ekran (jedna osoba
  naraz; odejście od biurka zamyka ekran).
- Komputer jest **zawsze zalogowany na właściciela**: przy cudzym odblokowanym
  komputerze piszesz w jego imieniu (ekran ostrzega „Uwaga: piszesz jako …”).
- **Blokada tylko ręczna** — przycisk „Zablokuj” (może nacisnąć każdy);
  odblokować może tylko właściciel („odcisk palca”). Kto zapomni, ryzykuje
  żart kolegów.
- **Laptop może zabrać każdy**, także zablokowany (z ekranu: „Zabierz laptop”,
  potrzebne wolne ręce) — używać go i tak nie da się bez właściciela.
- **Komunikator**: #ogólny, kanał własnego działu (#it-produkt / #biznes —
  widzi tylko dział) i wiadomości prywatne do każdego pracownika; licznik
  nieprzeczytanych. Wiadomości czyta się tylko przy komputerze.
- Bez trwałych kont: laptop i rozmowy prywatne gracza znikają, gdy wyjdzie z
  gry; historia kanałów trwa do restartu serwera.

### 10.15 Statystyki postaci (etap 5 z 9a)

- Paski w prawym górnym rogu: **Głód**, **Energia**, **Stres**, **Toaleta**
  (0–100, z zielonego na czerwony; krytyczne migają).
- Tempo (czas rzeczywisty): głód 0→100 w ~25 min, energia 100→0 w ~35 min,
  toaleta 0→100 w ~20 min; stres rośnie, gdy któraś potrzeba jest zaniedbana
  (głód ≥ 70, energia ≤ 25, toaleta ≥ 80), a bez tego powoli spada.
- Co pomaga:
  - **Owoc** — darmowa misa na blacie w chill roomie („owocowe czwartki,
    codziennie”): E = weź (jabłko, banan, gruszka, mandarynka), F = zjedz
    (głód −20, energia +3).
  - **Kawa** (F) — energia +25, stres −3, ale toaleta +8.
  - **Sofa** (E) — odpoczynek: energia i stres szybko w dobrą stronę.
  - **Toaleta** (E) — opróżnia pęcherz w ~8 s („Ulga!”).
  - **Papieros** przy popielniczce w strefie palenia (E) — 30 s, stres −25.
  - Ruch albo ponowne E kończy odpoczynek.
- Konsekwencje (miękkie): jednorazowe ostrzeżenia w dymku, stres z
  zaniedbania, głód 100 = energia spada 2× szybciej, toaleta 100 = „wpadka”
  (komunikat dla pokoju, stres +30), a przy energii ≤ 10 albo toalecie ≥ 90
  postać **chodzi wolniej** (kropla potu nad głową).
- **Łazienki wg płci**: „nie ta” łazienka działa, ale z zawstydzonym
  komentarzem i odrobiną stresu (postać o płci „inna” — bez komentarza).
- Inni widzą, co robisz: siedzenie (sofa, toaleta, komputer), papieros z
  dymkiem, „zzz” na sofie, zmęczenie.

### 10.16 Kabiny toaletowe

- W każdej łazience 3 kabiny (toaleta, miejsce do stania, drzwi); partycje
  oddzielają je od siebie i od przejścia przy umywalkach.
- **Nie widać, kto jest w kabinie** — ani z łazienki, ani z korytarza. Z
  kabiny widać (i słychać) łazienkę.
- **L** w kabinie zamyka / otwiera drzwi (znak na drzwiach: zielony = wolne,
  czerwony = zajęte). Zamkniętych nie da się otworzyć z zewnątrz.
- **Niezamkniętą kabinę można otworzyć**: kto wejdzie w drzwi, widzi osobę w
  środku (i ona jego). Nie da się zamknąć drzwi, gdy ktoś w nich stoi.
- Wyjście z kabiny albo z gry otwiera zamek automatycznie.

### 10.17 Higiena

- Piąty pasek **Higiena** (100 = czysto): spada powoli (100→0 w ~60 min).
- **Toaleta brudzi ręce** (napis „Brudne ręce” pod paskami, higiena −5).
- **Umywalka** (E, ~5 s mycia, ruch przerywa): czyste ręce, higiena +40.
- **Płyn antybakteryjny** z dozownika (E, od razu): czyste ręce, higiena +10.
  Dozowniki: w obu łazienkach i w chill roomie przy owocach.
- Konsekwencje: higiena < 25 — zielona „chmurka” nad postacią (widzą ją
  wszyscy) i rosnący stres; wyjście z łazienki z brudnymi rękami przy
  świadku — komentarz „Ej, …, a ręce?!” (i trochę stresu); owoc jedzony
  brudnymi rękami — „Fuj…” i stres +5.

### 10.18 Winda

- Drzwi są zamknięte, dopóki winda nie stoi na piętrze. **E przy drzwiach
  wzywa windę**; nad drzwiami wyświetlacz: piętro (P, 1) i strzałka jazdy.
- Jazda: ~3 s na piętro; po przyjeździe drzwi otwarte ~4 s (nie zamkną się na
  kimś w drzwiach). **E w kabinie** wybiera piętro (przy dwóch aktywnych —
  drugie); drzwi zamykają się po 1 s i jadą wszyscy w kabinie.
- **Maksymalnie 6 osób**: z większą liczbą winda nie ruszy — drzwi zostają
  otwarte, a ktoś w kabinie woła „Przeciążenie!”. Kabina jest mała (3×2
  pola), więc 6 osób stoi ciasno.
- W trakcie jazdy **widać tylko kabinę** (reszta ekranu jest wygaszona,
  kabina lekko drga).

### 10.19 Klatka schodowa i półpiętro

- Schody między parterem a piętrem 1 prowadzą przez **osobny widok klatki
  schodowej**: bieg w górę, **półpiętro** (podest), drugi bieg. Widać tylko
  klatkę i osoby na niej; przejście trwa kilka sekund.
- Przy wyjściach etykiety, dokąd prowadzą (Parter / Piętro 1 / Klatka
  schodowa).

### 10.20 Sklep i pieniądze

- **Portfel** w złotówkach (grosze na serwerze), widoczny nad paskami potrzeb.
  Na razie jedyny przychód: **200 zł zaliczki** przy podpisaniu umowy w HR
  (dzienna pensja razem z dniami gry — backlog 9b).
- **Sklep na parterze** (przed bramkami, dostępny także dla gości):
  - **E przy półce** pokazuje towary z cenami; „Weź” (albo 1–9) wkłada towar do
    kieszeni / rąk jako **niezapłacony** (widać to w ekwipunku, z ceną);
  - **kasa** (NPC „Kasa” za ladą): E = płacisz za wszystkie niezapłacone rzeczy;
    za mało pieniędzy — trzeba coś odłożyć;
  - **wyjście z niezapłaconym towarem**: bramka piszczy, towar zostaje w
    sklepie, stres +10.
- Półki: **Kanapki** (z serem 12 zł, z szynką 14 zł, wrap wege 13 zł),
  **Fast food** (hamburger 18 zł, frytki 9 zł — tylko w rękach), **Przekąski**
  (drożdżówka 6 zł, batonik 5 zł, chipsy 7 zł), **Napoje** (woda 4 zł,
  energetyk 8 zł, sok 6 zł), **Alkohol i papierosy** (piwo 7 zł, wino 25 zł,
  papierosy 18 zł / 20 szt.).
- **F = zjedz / wypij** (niezapłaconego nie można): każdy towar zmienia potrzeby
  (np. kanapka głód −35…−40, energetyk energia +30 ale stres +8 i toaleta +10,
  piwo stres −15 i toaleta +20).
- **Palenie wymaga papierosów** (jeden z paczki na przerwę).
- Do przemyślenia: konsekwencje alkoholu w pracy, zwroty, promocje.

### 10.21 Zegar, pory dnia i dni gry

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

### 10.22 Dojazd do pracy

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

### 10.23 Pogoda

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

### 10.24 Kalendarz i spotkania z zarządem

- **Zarząd** to dwoje NPC: **Prezes** (podwyżki, skargi, luźne rozmowy) i
  **Wspólniczka** (pomysły na produkt), w pokoju zarządu na piętrze 1.
- **Drzwi zarządu są zamknięte** — wchodzi tylko osoba z umówionym
  spotkaniem, **od 10 min przed do 10 min po jego początku**; wyjść można
  zawsze. Przy drzwiach podpowiedź „wstęp tylko na umówione spotkanie”.
- **Kalendarz** to druga zakładka na komputerze (obok komunikatora): sloty po
  30 min, 10:00–17:30, na dziś; wybór tematu, „Umów” / „Odwołaj”; jedno
  spotkanie dziennie, rezerwacja min. 10 min wcześniej. Kalendarz należy do
  **właściciela komputera** — z cudzego odblokowanego laptopa można komuś
  umówić spotkanie (np. „prośbę o podwyżkę” za niego).
- **Spotkanie**: E przy właściwej osobie → okno rozmowy z trzema odpowiedziami
  (1–3); NPC odpowiada w dymku.
  - *Prośba o podwyżkę* (Prezes): szansa rośnie ze stażem i dobrą odpowiedzią;
    sukces = +5 zł na godzinę (stawka od 30 zł/h); ponownie najwcześniej za 3
    dni.
  - *Pomysł na produkt* (Wspólniczka): dwa pytania; dwie dobre odpowiedzi =
    pochwała na #ogólny i stres −10, jedna = −3, żadna = +3.
  - *Skarga* (stres −8), *luźna rozmowa* (stres −5) — z humorystycznymi
    odpowiedziami Prezesa.
- **Spóźnienie ponad 10 min** — spotkanie przepada, Prezes daje znać („Nie było
  Cię na spotkaniu o 14:30. Szkoda.”), stres +5.

### 10.25 Słodycze w chill roomie i nieświeże owoce

- **Taca ze słodyczami**: 1–2 razy dziennie, o losowej godzinie między 9:00 a
  16:00, na stole w chill roomie pojawia się taca **pączków, ciastek albo
  sernika** (4–8 sztuk). HR ogłasza to na #ogólny; kto pierwszy, ten lepszy.
  E przy stole = jedna sztuka (do kieszeni), F = zjedz: trochę syci, dodaje
  energii, obniża stres. Na razie decyduje serwer (w przyszłości — gracze).
- **Nieświeże owoce**: ok. 15% owoców z misy jest „nie pierwszej świeżości”
  (widać to w nazwie przedmiotu — można zaryzykować). Po zjedzeniu:
  **rozstrój żołądka** — potrzeba toalety skacze do min. 70 i rośnie o ~1
  pkt/s, w HUD czerwone ostrzeżenie. Kto nie zdąży do toalety w ~30 s, ma
  „wpadkę”; toaleta leczy żołądek.

### 10.26 Zamawianie obiadów

- **Aplikacja „Obiady”** — trzecia zakładka na komputerze. Sześć dań z
  fikcyjnych lokali: pierogi ruskie (Pierogarnia u Zosi, 24 zł), pizza
  margherita (Pizza Bella, 32 zł), zestaw sushi (Sushi Koi, 45 zł), schabowy
  (Bar Mleczny „Pod Kogutem”, 22 zł), sałatka z kurczakiem (Zielona Miska,
  27 zł), kebab (Kebab u Ahmeda, 25 zł) — każde z czasem dostawy (25–50 min)
  i działaniem: mocno syci, zwykle trochę usypia, obniża stres.
- **Zamówienia 10:00–15:00**, jedno naraz; płaci **konto właściciela
  komputera** (z cudzego odblokowanego laptopa można więc komuś zamówić
  obiad na jego koszt).
- **Dostawa na recepcję** (piętro 1) po czasie dostawy ±10 min, w deszczu +15
  min. Recepcja daje znać zamawiającemu („Kurier był! Kebab czeka na
  recepcji.”), E przy recepcji = pudełko do rąk (trzeba mieć wolne ręce), F =
  zjedz. Nieodebrane obiady wieczorem trafiają do kosza.

### 10.27 Wakaty i obsadzone stanowiska

- To start firmy, więc **ogłoszeń jest mało**: na początku tylko Programista/ka
  i Specjalista/ka ds. sprzedaży, po jednym miejscu. **Każdego ranka** firma
  otwiera jedno nowe miejsce na losowym stanowisku (maks. 3 na stanowisko); w
  przyszłości tempo wyznaczy wzrost firmy.
- Na portalu przy ofertach naszego startupu widać „Wolne miejsca: N”.
  Stanowiska bez wolnych miejsc **znikają** z portalu (chyba że już się na nie
  aplikowało).
- **Kto pierwszy zda rozmowę, ten dostaje miejsce.** Gdy ostatnie wolne miejsce
  zostanie obsadzone, pozostali w trakcie tej rekrutacji (czekający na
  zaproszenie, zaproszeni, w trakcie rozmowy) dostają maila „Stanowisko
  obsadzone”; kto zda rozmowę po czasie, dowiaduje się tego zamiast
  zaproszenia na dzień próbny.
- Gdy zatrudniony gracz opuści grę, jego miejsce znów jest wolne (brak trwałych
  kont).

### 10.28 Panel założyciela (bez płatności)

- Serwer bez założyciela pokazuje na portalu kartę **„Załóż własną firmę”**:
  nazwa (3–40 znaków) i przycisk. Kto pierwszy, ten zakłada: trafia od razu do
  budynku jako **Zarząd** (dział 3) — z umową, zaliczką, kartą i laptopem, przy
  stole w sali zarządu, z dostępem do niej na stałe. Jeden założyciel na serwer
  (płatne instancje firm to osobny, późniejszy projekt).
- **Nazwa firmy** jest wszędzie: w ofertach na portalu, w nadawcy maili
  („<firma> — Rekrutacja”), w rozmowie online i w `Clock`.
- **Panel** to zakładka **„Firma”** na komputerze założyciela (widoczna tylko
  przy jego własnym koncie):
  - zmiana nazwy;
  - ogłoszenia: liczba miejsc (−/+, 0–5) i opis stanowiska;
  - **kandydaci** po zdanej rozmowie: Zatrudnij / Odrzuć. Kandydat dostaje
    maila „Decyzja zarządu wkrótce”; jeśli założyciel nie zdecyduje w 30 min
    gry albo nie ma go w grze, kandydat jest zatrudniany automatycznie (jak
    dotąd);
  - **zespół** (zatrudnieni, także jeszcze przed umową) z dniem zatrudnienia i
    przyciskiem **Zwolnij**: zwolniony traci kartę, laptopy i pojazd, wraca na
    portal z mailem „Rozwiązanie umowy” (świeża skrzynka), a jego miejsce
    znowu jest wolne.
- Gdy założyciel wyjdzie z gry, firma zostaje bez założyciela (nazwa
  zostaje), a portal znowu proponuje jej założenie.

### 10.6 Stan implementacji

*Stan na 2026-09-26 — etap 1 (sieć) ukończony; dodane IPv6, sesje po tokenie,
automatyczne ponowne łączenie, budynek wg GDD (parter z terenem zewnętrznym,
piętro 1, schody, winda), uprawnienia (bramki) oraz cała ścieżka nowego
gracza: portal z ofertami → rekrutacja → portier → recepcja → HR → karta
pracownika z działem; oprawa graficzna w pixel arcie (10.9).*

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
- **Pulpit i rekrutacja**: pulpit komputera z przeglądarką (portal kilku
  firm, formularz), pocztą i rozmową online; quiz oceniany na serwerze
  (3 pytania, 2 poprawne); przydział do działu z umową w HR, dział przy
  nicku (10.12); protokół v7.
- **Grafika**: proceduralny pixel art otoczenia, mebli i postaci (10.9).
- **Tworzenie postaci**: dane postaci i edytor wyglądu (10.11); protokół v6.
- **Ekwipunek**: kieszenie i ręce, przedmioty (przepustka, karta, laptop,
  kawa), upuszczanie / podnoszenie / podawanie, dostęp z przedmiotów (10.13);
  protokół v8.
- **Panel założyciela**: zakładanie firmy z portalu, nazwa firmy w grze,
  zakładka „Firma” (miejsca, opisy, kandydaci, zespół, zwalnianie) (10.28);
  protokół v23.
- **Wakaty**: mało ogłoszeń na start, nowe miejsca co rano, obsadzone
  stanowiska znikają, maile „obsadzone” (10.27); protokół v22.
- **Zamawianie obiadów**: aplikacja z menu 6 dań, płatność z konta
  właściciela komputera, dostawa na recepcję z powiadomieniem (10.26);
  protokół v21.
- **Słodycze i nieświeże owoce**: losowe tace w chill roomie z ogłoszeniem na
  #ogólny, rozstrój żołądka po nieświeżym owocu (10.25); protokół v20.
- **Kalendarz i zarząd**: Prezes i Wspólniczka, drzwi otwierane na spotkanie,
  kalendarz na komputerze, cztery tematy z dialogami i skutkami (podwyżka,
  pochwała, stres), przepadające spotkania (10.24); protokół v19.
- **Pogoda**: słońce, chmury, deszcz, burza, mgła; moknięcie, parasol, wpływ na
  dojazd, efekty na ekranie (10.23); protokół v18.
- **Dojazd do pracy**: poranny wybór pięciu sposobów, czas, koszt, wpływ na
  potrzeby, pojazdy z przyjazdem na parking / stojak / przystanek, spóźnienia
  (10.22); protokół v17.
- **Zegar i dni gry**: wspólny zegar, biuro 6–22, noc przewijana, poranne
  przyjazdy 7–10, dni gracza, pensja godzinowa, oświetlenie wg pory dnia
  (10.21); protokół v16.
- **Sklep i pieniądze**: portfel, zaliczka 200 zł, półki + kasa + bramka,
  14 towarów, jedzenie z efektami, papierosy do palenia (10.20); protokół v15.
- **Higiena, winda, klatka schodowa**: pasek higieny i brudne ręce, umywalki i
  dozowniki (10.17); winda wzywana, jadąca, z drzwiami, limitem 6 osób i
  widokiem samej kabiny w czasie jazdy (10.18); klatka schodowa z półpiętrem
  (10.19); protokół v14.
- **Kabiny toaletowe**: zamykane od środka, ukrywają osobę w środku, otwarte
  można podejrzeć (10.16); protokół v11.
- **Statystyki postaci**: głód, energia, stres, toaleta; owoce, kawa, sofa,
  toaleta, papieros; ostrzeżenia, „wpadka”, wolny chód; łazienki wg płci
  (10.15); protokół v10.
- **Komputer i komunikator**: laptop na biurku działu, ekran komputera z
  komunikatorem (kanały, prywatne, nieprzeczytane), blokada, pisanie z cudzego
  komputera w imieniu właściciela, zabieranie laptopa (10.14); protokół v9.
- **Ekspres do kawy**: parzenie, kubek w ręce widoczny dla innych, jedna
  osoba naraz (10.10); protokół v5.
- **Boty** (`cargo run --release --bin bots`): 50 domyślnie, chodzą po BFS po
  całym budynku (schodami), część zbiera się w wybranym pokoju (domyślnie
  Chill room na piętrze 1).
- **Sieć mobilna**: serwer dual-stack IPv4/IPv6; gracz identyfikowany tokenem
  (zmiana adresu w trakcie gry przenosi sesję); klient przepina gniazdo po
  ciszy/powrocie z tła i sam łączy się ponownie po utracie sesji.
- **Testy**: 88 jednostkowych w Rust (budynek i pokoje wg GDD, osiągalność
  zależna od uprawnień, bramki, ruch/kolizje, schody, winda, nawigacja,
  portier, recepcja, HR, rekrutacja, ekspres, komunikator, potrzeby, higiena, kabiny, winda, klatka schodowa, sklep, zegar, dojazd, pogoda, zarząd, słodycze, obiady, wakaty, firma, protokół), 2 golden, 25 e2e serwera
  (m.in. portal: odrzucenie → przyjęcie → spawn; całe wdrożenie aż do karty;
  niewidoczność między piętrami; zgodność stanu serwera z predykcją) —
  łącznie 115 (w tym e2e ekspresu, profilu postaci, pulpitu, przekazywania karty, komputera z komunikatorem, potrzeb, kabin, higieny, windy, sklepu, wypłaty o 22:00 porannego dojazdu samochodem moknięcia w deszczu spotkania z Prezesem tacy ze słodyczami obiadu z odbiorem na recepcji, obsadzonego stanowiska i panelu założyciela); 145 sprawdzeń w Godocie (parytet protokołu i ruchu, parsowanie
  adresów).

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
| rekrutacja + wdrożenie w oknie klienta (zgadywanie odpowiedzi) | przyjęta (2/3) po kilku próbach, umowa „IT / Produkt”, przy nicku „Zosia · IT” |
| 50 botów: portal (zgadywanie) → Chill room (`--start-with-card`) | 50/50 przyjętych w < 5 s, 0 zgubionych ticków, 0 błędnych predykcji |
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
- Przepustka, karta i dział znikają po rozłączeniu (brak kont i zapisu
  postępu) — po każdym połączeniu rekrutację i wdrożenie trzeba przejść od nowa.
- Boty bez `--start-with-card` zostają w strefie publicznej (nie rozmawiają z
  portierem).

#### Następne kroki (propozycja)
Zgodnie z MVP (sekcja 9): zadania działów (1–2 na dział), NPC Zarządu,
palenie + alarm; do tego trwałość postępu (konta), żeby nie przechodzić
rekrutacji przy każdym połączeniu. Punkty
wpięcia opisane w `docs/ARCHITECTURE.md` („Gotowość na rozbudowę”).
