# Protokół — historia wersji

*Zmiany formatu w kolejnych wersjach ([specyfikacja](protokol.md)). Nowa wersja = podbite `VERSION` w `server/src/protocol/` i wpis na górze tej listy.*

- **47** — piętro biurowe to teraz piętro 4 (`floor4.json`), nowe piętro 3 (`floor3.json`), piętra 1 i 2 zablokowane; klatki schodowe jako mapy 5 (parter–3) i 6 (3–4); panel pięter w windzie jako `Dialog` z `npc` 0 + `DialogAnswer` (zamiast „E jedzie na następne piętro”).
- **46** — `Dialog`: do 9 odpowiedzi (było 4, jak w pytaniach rekrutacji) — menu telewizora, boomboxa, R i apteczka były ucinane, bez „Wyłącz” / „Nic”; na końcu `Dialog` lista przedmiotów przy opcjach (szafka, apteczka, magazynek, barek — rysowane jak ekwipunek); `Dialog` 249 = głosowanie „pomiń czekanie” (alkomat: 200–248); kałuża `held` 3 = krew po dźgnięciu; `SkipWait` zaczyna głosowanie (`Clock::skip` 1 = głosowanie trwa).
- **45** — czat i powiadomienia: `ChatSay` (60, C→S), `Notice` (61, S→C); przedmioty 52–55 (barek); zagubiony przechodzień (NPC, pytanie jako zwykły `Dialog`).
- **44** — skręty i zapasy: `Roll` (59, C→S), przedmioty 43–51, uprawnienie 16 (klucz do magazynku), `Dialog` 255 (apteczka / magazynek).
- **43** — telewizor i boombox: `Media` (58, S→C), przedmioty 41 pilot, 42 boombox, `Dialog` 253 (kanały), 254 (utwory).
- **42** — Kadry: `HrAction` (56, C→S), `HrInfo` (57, S→C); `Clock` + `leave u8` na końcu (dzień urlopu: w domu).
- **41** — widełki i umowa: `JobOffers` oferta + `salary_min u32`, `salary_max u32` (zł brutto / mies., po `vacancies`); `Apply` + `salary u32`, `form u8` (1 umowa o pracę, 2 B2B, 3 umowa zlecenie — tylko student < 26 lat), `student u8` (po `motivation`); `Dialog` 252 = umowa w HR (0 podpisuję, 1 rezygnuję).
- **40** — psoty i bójki: `Stats` + `bowels`, `health` (po `alcohol`); czynności 11 nokaut, 12 cios, 13 sikanie, 14 kucanie; dźwięki 20 cios, 21 dźgnięcie, 22 sikanie, 23 kupa; kałuża `held` 2 = kupa; przedmiot 40 nóż; wygląd NPC 7 = kasjer; `Action` (55, C→S); `Dialog` 250 (menu R) i 251 (szafka).
- **39** — upojenie alkoholem: `Stats` + `alcohol` (po `hygiene`); `Snapshot` + `self_drunk` (po `self_slow`, zataczanie w symulacji — też w wektorach golden ruchu); flagi encji gracza bity 4–5 = upojenie; czynności 9 wymioty, 10 odsypianie; dźwięki 18 beknięcie, 19 wymioty; kałuża `held` 1 = wymiociny; `CompanyPeople` pracownik + `reprimands u8` (przed `nick`); przedmiot 39 alkomat (pytanie o naganę to zwykły `Dialog`).
- **38** — kałuża po wpadce: encja `kind` 6 (`flags`, `held`, `activity` = 0), id z puli od `0xE000`; widoczna jak przedmioty w pokoju; ściera ją sprzątaczka, inaczej znika o 22:00.
- **37** — osobne działy: pakiet `Departments` (54, S→C) z listą działów; działy 4–10 (Mobile, DevOps, AI, Finanse, Sales, Marketing, Obsługa klienta), dział 1 nazywa się „Produkt / IT”.
- **36** — dwie windy: `Doors` kończy się listą wind `n u8` (≤ 16) × {`floor u8`, `target u8` (255 = stoi), `moving u8`} zamiast jednej trójki `lift_*`.
- **35** — stanowiska firmy: `CompanyOffers` w częściach, z działem, zestawem pytań i listą zestawów; `CompanyAction` 8–12 (dodaj / nazwa / dział / zestaw / usuń stanowisko).
- **34** (uzup.) — `Reject` 7 = nick zajęty (konto, zapisana postać albo ktoś w grze), 8 = e-mail postaci zajęty.
- **34** — szyfrowanie: pakiety `0xF0` (sesja) i `0xF1` (Connect) dla zalogowanych; pakiet gry ≤ 1152 B (datagram ≤ 1200 B); klucz sesji w odpowiedzi logowania (`key`).
- **33** — konta: `Connect` + `ticket` str16 (≤ 64 B, z logowania HTTPS; pusty = gość); `Reject` 5 (bilet wygasł), 6 (serwer wymaga konta), 7 (nick ma konto).
- **32** — czat głosowy: `Voice` (52, C→S), `VoiceFrom` (53, S→C).
- **31** — firmowy komputer: `TaskAction` (46, C→S), `TaskBoard` (47), `TaskDetail` (48), `MailAction` (49, C→S), `WorkMail` (50), `MailState` (51).
- **30** — dźwięki: `Sound` (45, S→C): n u8 (≤ 64) × {kind u8, x i32, y i32} — zdarzenia słyszalne na piętrze odbiorcy w promieniu 28 kafli (1 ekspres, 2 kasa, 3 bramka sklepu, 4 winda, 5 zamek kabiny, 6 włącznik, 7 spłuczka, 8 kran, 9 zapalniczka, 10 zmywarka, 11 lodówka, 12 szafka, 13 podniesienie, 14 upuszczenie, 15 jedzenie, 16 picie, 17 gwizdek).
- **29** — `SkipWait` (44, C→S: token u32) — „Pomiń czekanie” w domu; `Clock` + `skip` u8 (0 nie, 1 poproszono, 2 czas pędzi); aktywność 8 = zatrzymany (ochrona / policja).
- **28** — aneks kuchenny: `Fridge` (42), `FridgeAction` (43); przedmioty 35 kubek (czysty), 36 mleko (karton), 37 kawa z mlekiem; 34 = brudny kubek.
- **27** — światło: pakiet `Lights` (41); w mapie `room_defs.*.light` / `switch` / `lit_by` / `windows`.
- **26** — dym i straż: `Clock` + `alarm`, pakiet `Smoke` (40), wygląd NPC 6 (strażak), pojazd 6 (wóz strażacki).
- **25** — kubki i sprzątaczka: przedmiot 34 = pusty kubek, wygląd NPC 5 (sprzątaczka); bez nowych pakietów.
- **24** — ochrona i policja: wygląd NPC 3 (ochroniarz) i 4 (policjant), pojazd 5 (radiowóz); bez nowych pakietów.
- **23** — panel założyciela: `Clock` + `company`, `founded`; `CompanyOffers`, `CompanyPeople`, `CompanyAction`; dział 3 (Zarząd).
- **22** — wakaty: `JobOffers` + `vacancies u8` po `applied` (wolne miejsca; stanowiska z 0 portal ukrywa, chyba że gracz już aplikował).
- **21** — obiady: `LunchMenu`, `LunchOrder`, przedmioty 28–33.
- **20** — słodycze: encja tacy (`kind` 5: `held` = słodycz, `activity` = liczba sztuk), przedmioty 25 pączek, 26 ciastko, 27 sernik, `Stats.flags` bit 1 = rozstrój żołądka.
- **19** — zarząd: `Calendar`, `CalendarBook`, `Dialog`, `DialogAnswer`, uprawnienie 8 (drzwi zarządu).
- **18** — pogoda: `Clock` + `weather`, flaga parasola (bit 3 u graczy), przedmiot 24 = parasol.
- **17** — dojazd: `Clock` + `mode`, `depart`, `money`; `CommuteChoice`; encja pojazdu; czynność 7 (jedzie).
- **16** — zegar i dni: `Clock`.
- **15** — sklep: `Stats` + `money`, `Shelf`, `ShopTake`, przedmioty 10–23.
- **14** — `Doors` + `lift_moving`; limit 6 osób w windzie; mniejsza kabina (3×2).
- **13** — higiena: `Stats` + `hygiene`, `flags` (brudne ręce), flaga encji 7 = niska higiena, czynność 6 = mycie rąk.
- **12** — winda poza symulacją (wzywanie, jazda, drzwi w `Doors`), `Doors` + `lift_floor`, `lift_target`; mapa klatki schodowej (piętro 3).
- **11** — kabiny toaletowe: `Doors`, `DoorAction`; zamknięte drzwi blokują ruch (także w predykcji klienta).
- **10** — potrzeby: `activity` w encji (14 B) i `self_activity` w miejsce bitów `self_status`, `self_slow` (wolny chód w symulacji — też w wektorach golden ruchu), flaga 6 = wolny chód, pakiet `Stats`, przedmiot 5 = owoc.
- **9** — komputer i komunikator: encja laptopa (`kind` 3), bit „przy komputerze” (`self_status` 0 / flaga 6, w miejsce „trzyma kawę”), `Computer`, `ComputerAction`, `Chat`.
- **8** — ekwipunek: `held` w encji (13 B), encje przedmiotów na podłodze, `Inventory`, `ItemAction`; kubek kawy jako przedmiot.
- **7** — pulpit: firmy i flaga `applied` w `JobOffers` (dzielonych na pakiety), `motivation` w `Apply`, `Mail`, `PortalAction`.
- **6** — profil postaci w `Connect` (płeć, wiek, wygląd, miejscowość, e-mail); płeć i wygląd w `PlayerInfo`; `Reject(4)`.
- **5** — `self_status` w snapshocie, bity czynności 6–7 we `flags` encji; `Say` także od graczy.
- **4** — portal i rekrutacja (typy 12–16); `department` w `PlayerInfo`.
- **3** — snapshot: `self_access`; pakiet `Say`; encje NPC (`kind` 1) z imionami w
  `PlayerInfo`; wygląd w bitach 3–5 `flags` (dodany bez zmiany formatu).
- **2** — snapshot: pola `self_lock`, `self_prev_input`; bit inputu 16 (interakcja);
  `map_crc` liczone z całego budynku (wiele pięter).
- **1** — wersja początkowa.
