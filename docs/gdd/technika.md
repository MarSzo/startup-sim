# Technika

*Część [GDD](README.md). Stack, platformy, sieć, zapis, konta, szyfrowanie, wydajność, awarie, aktualizacje, czat głosowy.*

## 10.2 Stack (zaimplementowany)
- Klient: Godot 4.7, GDScript.
- Serwer: dedykowany, autorytatywny, Rust (`std::net::UdpSocket` + `socket2`, jeden wątek).
- Transport: UDP, własny binarny protokół ([protokół](../dev/protokol.md)), IPv4 + IPv6.

## 10.3 Platformy i crossplay
Decyzja (2026-09-26): zostajemy przy Godocie; Unity rozważone i odrzucone.
Konsole (Switch, Xbox, PlayStation) — „może kiedyś”, przez firmę portującą
(np. W4 Games); wtedy dojdą konta platform, certyfikacja i wymogi crossplay.
Serwer i protokół nie zależą od platformy.

Przygotowane już pod platformy mobilne (gdyby wróciły do planu):
- ✅ IPv6 po obu stronach; ✅ sesja po tokenie (zmiana sieci); ✅ automatyczne ponowne łączenie.
- ⬜ Sterowanie dotykowe, skalowanie UI, eksport Android/iOS, konta sklepów.

## 10.4 Etap 1 — pionowy wycinek sieci

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

## 10.40 Czat głosowy

- **Push-to-talk**: trzymasz **V** — słyszą Cię wszyscy w **tym samym
  pomieszczeniu** (dźwięk przestrzenny: ciszej z daleka); trzymasz **B** —
  **szept** tylko do najbliższej osoby w promieniu ~1,5 kafla (podpowiedź
  pokazuje, do kogo; nikogo obok — nic nie leci).
- Nad mówiącą postacią pojawia się znaczek z falami (szept — spokojniejszy).
- Mikrofon włącza się przy pierwszym naciśnięciu (macOS pyta wtedy o
  zgodę); w polu tekstowym (komunikator, formularze) V / B nie nadają.
- Ustawienia: głośność głosów graczy, wybór mikrofonu.
- Jakość: 16 kHz, IMA ADPCM (~64 kb/s), ramki po 40 ms. Serwer tylko
  przekazuje ramki (nie dekoduje, nie zapisuje, nie loguje) i pilnuje, kto
  słyszy: ten sam pokój albo — szept — jedna najbliższa osoba; limit ~30
  ramek/s na gracza.
- Eksport na macOS będzie potrzebował opisu uprawnienia mikrofonu
  (`privacy/microphone_usage_description` w presecie eksportu).

## 10.41 Zapis postępu (etap 1 kont)

- Serwer zapisuje grę w pliku SQLite (`--save`, domyślnie
  `server/saves/world.db`; `--no-save` wyłącza). Po restarcie serwera, a
  także po wyjściu z gry i powrocie **tym samym nickiem**, wszystko wraca:
  - postać: profil (dane postaci tylko na serwerze), pieniądze, dzień, dział
    i umowa, stawka, potrzeby, ekwipunek (kawa przez restart stygnie — w
    kieszeni zostaje brudny kubek), sposób dojazdu;
  - świat: zegar, pogoda, firma (nazwa, założyciel, opisy stanowisk,
    zatrudnieni), wolne etaty, laptopy na biurkach, tablice zadań, poczta
    służbowa, aneks kuchenny.
- Nie zapisujemy tego, co odtwarza się samo: NPC, pojazdy, dym, dźwięki,
  głos, winda; historia komunikatora i spotkania w kalendarzu — jeszcze nie.
- Świat jest trwały: kto wychodzi, **zachowuje etat, biurko i firmę**
  (założyciel offline nadal jest założycielem). Postać bez umowy wraca na
  portal z pracą (z tymi samymi pieniędzmi i dniem).
- Po restarcie zatrudniony wraca do pracy przy wejściu do budynku (w nocy —
  w domu).
- Zapis co 10 s (tylko zmienione wiersze, w osobnym wątku — pętla gry nie
  czeka), od razu po zmianach pieniędzy i zatrudnienia, a przy Ctrl+C /
  SIGTERM całość przed wyjściem. Raz na dobę kopia w `saves/backups/`
  (7 ostatnich).
- **Uwaga:** na razie postać rozpoznaje się po nicku (bez hasła) — hasła i
  konta to etap 2.

## 10.42 Konta i logowanie (etap 2)

- **Konto = nick postaci + hasło.** Hasło tylko jako skrót Argon2id; 5
  złych prób (na nick i adres) = minuta przerwy; do 10 nowych kont na adres
  na godzinę. Postać z etapu 1 (zapis po nicku) przejmuje pierwsza osoba,
  która założy konto tym nickiem.
- Logowanie idzie przez **HTTPS** (port gry + 1), nie przez UDP. Serwer sam
  robi certyfikat; klient przypina go przy pierwszym połączeniu (jak SSH) i
  ostrzega o zmianie („Zaufaj nowemu certyfikatowi”). Na VPS: `--tls-cert`
  / `--tls-key` (np. Let's Encrypt) — wtedy zwykła weryfikacja.
- Logowanie daje **bilet** do gry (10 min, starczy też na ponowne
  połączenia) i **token odświeżania** (30 dni, jednorazowy, trzymany w bazie
  jako skrót). „Zapamiętaj mnie” trzyma na komputerze tylko token.
- Ekran logowania po „Graj”: serwer z listy (po nazwie, np. „Serwer
  testowy” — adresu gracz nie widzi ani nie wpisuje; z edytora dochodzi
  „Serwer lokalny (dev)”), nick, hasło, *Zaloguj*, *Załóż
  konto*, *Zmień hasło*; z zapamiętanym logowaniem — *Graj* / *Zaloguj na
  inne konto* / *Wyloguj*. Nowe konto → tworzenie postaci (nick już
  ustalony); istniejąca postać → prosto do gry (wygląd przychodzi z serwera).
  W menu gry (Esc) — *Wyloguj*.
- Drugie logowanie na to samo konto wyrzuca poprzednią sesję.
- **Nick jest unikalny** (bez rozróżniania wielkości liter): konto; gość nie
  wejdzie pod nickiem konta, zapisanej postaci ani kogoś, kto akurat gra (a
  logujący się właściciel konta wyrzuca gościa pod swoim nickiem).
- **E-mail postaci jest unikalny** wśród postaci kont (online i zapisanych,
  bez rozróżniania wielkości liter): zajęty — gra zostaje na ekranie postaci
  z komunikatem „Ten e-mail ma już inna postać — wpisz inny” (logowanie
  dalej ważne). Goście (dev, boty) tego nie sprawdzają.
- **Zapomniane hasło**: administrator — `server --reset-password <nick>`
  (jednorazowe hasło; „zapamiętaj mnie” przestaje działać), potem gracz
  zmienia je sam.
- **Goście** (bez konta, bez zapisu) tylko z `--allow-guests` (boty, testy,
  zwiastun) albo na serwerze bez zapisu (`--no-save`).
- Etap 3: pakiety gry zalogowanego gracza są szyfrowane (10.43).

## 10.43 Szyfrowanie gry (etap 3)

- Logowanie (HTTPS) daje oprócz biletu **klucz sesji** (32 B); przez UDP
  nigdy nie leci. Każdy pakiet gry zalogowanego gracza — w obie strony,
  także `Connect` i `Welcome` — jest **szyfrowany i podpisany** (AES-256-CBC
  + HMAC-SHA256, szyfruj-potem-podpisz; IV = licznik zaszyfrowany kluczem;
  podpis obejmuje kierunek, nagłówek i licznik).
- Podsłuchany bilet nic nie daje (Connect musi być podpisany kluczem);
  podrobione / zmienione pakiety są odrzucane; **powtórki** też (okno 64
  ostatnich liczników na sesję; nagrany Connect nie przejdzie drugi raz,
  więc nikt nie wyrzuci gracza jego starym pakietem). Sesja zalogowanego
  gracza przyjmuje tylko pakiety szyfrowane.
- Goście (`--allow-guests`) grają jak dawniej, jawnie.
- Pakiet gry ma do 1152 B (zaszyfrowany ≤ 1200 B).
- Administracja: `--list-accounts` (nick, od kiedy, ostatnie logowanie),
  `--reset-password <nick>`; kopie zapasowe są logowane (`* save: backup …`).

## 10.44 Wydajność klienta

- Klient rysuje najwyżej **60 klatek/s** (także na ekranach 120 Hz); w
  Ustawieniach „Oszczędzanie baterii” — **30 klatek/s**; gdy okno gry jest w
  tle — **20 klatek/s** (gra, sieć i czat głosowy działają dalej).
- Koszt CPU rośnie z liczbą klatek, więc to główne pokrętło: w biurze na
  MacBooku Pro (M-series) ok. 32% jednego rdzenia przy 60 kl./s, ok. 20% przy
  30 kl./s; ekran tytułowy ok. 16% (wcześniej ok. 43%).
- Zasada rysowania: to, co się nie zmienia, rysuje się raz; animacja przez
  przesunięcie / przezroczystość gotowego rysunku, nie przez rysowanie od
  nowa co klatkę (szczegóły: [architektura klienta](../dev/architektura-klient.md), „Wydajność rysowania”).

## 10.45 Raporty awarii klienta

- Gdy gra zamknie się niespodziewanie, przy następnym uruchomieniu pyta:
  „Gra zamknęła się niespodziewanie — wysłać twórcom raport?” (*Wyślij
  raport* / *Nie wysyłaj*, pole „Wysyłaj zawsze bez pytania”; to samo w
  Ustawieniach: „Wysyłaj raporty awarii bez pytania”).
- Raport: koniec dziennika gry z poprzedniej sesji (bez haseł i tokenów),
  wersja gry, system, procesor, karta graficzna. Idzie na serwer, na którym
  gracz się ostatnio logował (albo domyślny).
- Tylko w wydanych wersjach (w edytorze każde zatrzymanie wyglądałoby jak
  awaria; do testów `--crash-test`).

## 10.46 Aktualizacje klienta

- Przy starcie gra sprawdza najnowsze wydanie na GitHubie; jeśli jest nowsze
  niż jej wersja, nad ekranem tytułowym: „Dostępna nowa wersja gry: X (masz
  Y)” — *Pobierz* (strona wydania z dmg) / *Później*.
- Serwer z innym protokołem odrzuca klienta; zamiast suchego błędu: „Ta wersja
  gry nie pasuje do serwera — pobierz najnowszą” z tym samym przyciskiem.
- Na razie gracz sam pobiera i podmienia aplikację; aktualizacja jednym
  kliknięciem (Sparkle / łatki `.pck` / itch.io) — później.
