# Stan implementacji

*Część [GDD](README.md). Co już działa, pomiary, znane ograniczenia, następne kroki.*

## 10.6 Stan implementacji

*Stan na 2026-09-26 — etap 1 (sieć) ukończony; dodane IPv6, sesje po tokenie,
automatyczne ponowne łączenie, budynek wg GDD (parter z terenem zewnętrznym,
piętro 1, schody, winda), uprawnienia (bramki) oraz cała ścieżka nowego
gracza: portal z ofertami → rekrutacja → portier → recepcja → HR → karta
pracownika z działem; oprawa graficzna w pixel arcie (10.9). 2026-09-30:
nowy układ budynku wg odręcznego planu (10.5) — generator map znów jest
jedynym źródłem, otoczenie i stałe punkty w `places` mapy; dwie niezależne
windy (10.18, protokół 36); osobne działy dla każdego zespołu (rozdz. 5,
protokół 37).*

### Zrobione
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
- **Aneks kuchenny i menu**: policzone kubki, zlew, zmywarka, lodówka z
  mlekiem i darmowymi napojami; ekran tytułowy z ustawieniami, menu pod Esc z
  wyjściem (10.35); protokół v28.
- **Okna, światło, kamera, nowa mapa**: okna, włączniki i jasność
  pomieszczeń, zoom kamery, ręcznie rysowane podłogi, ściany i meble (10.34);
  protokół v27.
- **Balkon** na piętrze 1 z widokiem na ulicę i ludzi na dole (10.33).
- **Wygląd „papier i atrament”**: skalowanie z oknem, odręczna czcionka,
  papierowe panele, tarcze statystyk, pasek ekwipunku, efekt tuszu i papieru na
  świecie, postacie z większymi głowami (10.32).
- **Palenie, dym i straż**: papieros wszędzie, dym w pomieszczeniach
  przenikający przez drzwi, czujki, alarm z ewakuacją, strażak i kara (10.31);
  protokół v26.
- **Kubki i sprzątaczka**: pusty kubek po kawie (zostaw / umyj / dolewka z
  ekspresu), popołudniowy obchód Pani Marii (15–16) z narzekaniem i wpisem na #ogólny
  (10.30); protokół v25.
- **Ochrona i policja**: ochroniarz w sklepie goni złodzieja, radiowóz i
  policjant przy recydywie albo ucieczce, mandat (10.29); protokół v24.
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
- **Testy**: 115 jednostkowych w Rust (budynek i pokoje wg GDD, osiągalność
  zależna od uprawnień, bramki, ruch/kolizje, schody, winda, nawigacja,
  portier, recepcja, HR, rekrutacja, ekspres, komunikator, potrzeby, higiena, kabiny, winda, klatka schodowa, sklep, zegar, dojazd, pogoda, zarząd, słodycze, obiady, wakaty, firma, ochrona i policja, kubki, dym, balkon, tablica zadań, poczta służbowa, zapis gry, protokół), 3 golden, 34 e2e serwera
  (m.in. portal: odrzucenie → przyjęcie → spawn; całe wdrożenie aż do karty;
  niewidoczność między piętrami; zgodność stanu serwera z predykcją) —
  łącznie 152 (w tym e2e ekspresu, profilu postaci, pulpitu, przekazywania karty, komputera z komunikatorem, potrzeb, kabin, higieny, windy, sklepu, wypłaty o 22:00 porannego dojazdu samochodem moknięcia w deszczu spotkania z Prezesem tacy ze słodyczami obiadu z odbiorem na recepcji, obsadzonego stanowiska, panelu założyciela kradzieży w sklepie z ochroną i policją , kubka zebranego przez sprzątaczkę oraz papierosa, który uruchamia alarm pożarowy, wcześniejszego powrotu do domu z przystanku i pomijania czekania, zatrzymania przez ochronę, dźwięku ekspresu słyszanego przez innych, tablicy zadań działu z przypisaniem i komentarzem oraz poczty z koszem, głosu słyszanego tylko w pokoju i szeptu do osoby obok, restartu serwera z zapisem i kontami — prawdziwa binarka, rejestracja i logowanie przez HTTPS, odrzucenie gościa, złego i jawnego biletu, szyfrowana sesja, odrzucone powtórki pakietu i Connect, zajęty e-mail postaci, SIGINT i powrót postaci; unikalny nick gościa); 168 sprawdzeń w Godocie (parytet protokołu i ruchu, parsowanie
  adresów).

### Pomiary (MacBook, wszystko lokalnie)
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

### Znane ograniczenia
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

### Następne kroki (propozycja)
Zgodnie z MVP (sekcja 9): zadania działów (1–2 na dział), NPC Zarządu,
palenie + alarm; do tego trwałość postępu (konta), żeby nie przechodzić
rekrutacji przy każdym połączeniu. Punkty
wpięcia opisane w [architekturze](../dev/architektura.md) („Gotowość na rozbudowę”).
