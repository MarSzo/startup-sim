# Wizja gry

*Część [GDD](README.md). Koncepcja, świat, ścieżka gracza, struktura i rozwój firmy, pierwsze mechaniki.*

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
- Wejście z portiernią — windy, klatka schodowa i parking na kartę; portier (NPC) wpuszcza osoby bez karty
- Parking wewnętrzny
- Sklep — zakupy (np. kawa, przekąski, papierosy)
- Winda — panel pięter w kabinie (parter, piętro 3, piętro 4); piętra 1 i 2 zablokowane
- Schody — alternatywa dla windy

### Piętro 4 (biurowe, aktywne od startu)
- Recepcja przy wejściu na piętro
- Pokoje zespołów — każdy dział ma swój (rozdz. 5)
- Pokój działu Biznesu (marketing + sprzedaż)
- Pokój Zarządu
- Pokój HR
- Korytarz
- Chill room — wspólna przestrzeń dla wszystkich
- Łazienka damska i męska

Gracze mogą swobodnie chodzić po korytarzu, pokojach i wspólnych przestrzeniach.

### Piętro 3 (aktywne, wg planu architekta)
- Pokój wypoczynkowy ze stolikami i sofami, otwarty na kuchnię; balkony 3 i 4
- Open space (48 biurek po obu stronach przejścia), na wyspach WC damski, męski
  i dla niepełnosprawnych, lada recepcji; małe balkony 1, 2, 5, 6
- Pokój spotkań, serwerownia (zamknięta), dwa magazyny, toaleta
- Hol windowy z recepcją, klatka schodowa, sale konferencyjne 5 i 6, WC
- Korytarz do skrzydła sal: sale konferencyjne 1, 3 (16 osób) i 4, pokój
  biurowy, poczekalnia; balkon 7

### Piętra 1 i 2 (zablokowane)
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

| Dział (id) | Kto | Pokój (numer z planu) |
|-------|-----|-------|
| Produkt / IT (1) | Gracze — programiści, designerzy | 18, 19, 38 |
| Biznes (2) | Gracze | 27 |
| Zarząd (3) | NPC (prezes, wspólniczka) + założyciel | 37 |
| Mobile (4) | Gracze | 20 |
| DevOps (5) | Gracze | 32 „Mordor” |
| AI (6) | Gracze | 28 |
| Finanse (7) | Gracze | 16 |
| Sales (8) | Gracze — sprzedaż | 43 |
| Marketing (9) | Gracze | 44 |
| Obsługa klienta (10) | Gracze | 42 |
| HR | NPC | 46 (rekrutacja, dzień próbny, umowy, później konflikty i skargi) |

Zespoły = działy: każdy ma swoje biurka, tablicę zadań, kanał w komunikatorze
i skrzynkę działu. Lista działów jest w `server/data/recruitment.json` (nazwa
i skrót przy nicku); klient dostaje ją od serwera (pakiet `Departments`).
Na start rekrutujemy do Produkt / IT, Sales i Marketingu; założyciel może
dodać stanowiska w każdym dziale poza Zarządem.

**NPC na start:** portier; recepcjonista/recepcjonistka; Zarząd (CEO / założyciele); HR.

## 6. Rozwój firmy (wspólny cel serwera)

Praca graczy przynosi firmie przychody, a firma odblokowuje kolejne etapy:

| Etap | Co się odblokowuje |
|------|--------------------|
| Startup | IT, Biznes, Zarząd, HR — tylko piętro 4 |
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
