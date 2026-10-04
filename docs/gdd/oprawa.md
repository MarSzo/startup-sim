# Oprawa

*Część [GDD](README.md). Grafika „papier i atrament”, okna i światło, dźwięk.*

## 10.9 Oprawa graficzna (placeholder → pixel art)

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

## 10.32 Wygląd „papier i atrament” (w duchu Don't Starve)

Pierwsza wersja (pikselowa czcionka i ramki) była nieczytelna, zwłaszcza na
pełnym ekranie — zastąpiona stylem inspirowanym Don't Starve:

- **Skalowanie**: cały obraz (świat i interfejs) skaluje się z oknem (tryb
  `canvas_items`, podstawa 1280×720) — na dużym ekranie wszystko rośnie.
- **Czcionka** odręczna, czytelna: **Patrick Hand** (SIL OFL, `client/fonts/`),
  z polskimi znakami, zawsze nie mniejsza niż 18 px.
- **Panele** jak z papieru: pergamin (okna) i ciemne drewno (HUD) z nierównym
  konturem tuszu, postrzępionymi krawędziami, fakturą i cieniem; przyciski
  pergaminowe albo bordowe; pola tekstowe jak kartka.
- **Statystyki**: okrągłe tarcze w prawym górnym rogu — w każdej „płyn” w kolorze
  potrzeby (głód, energia, stres, toaleta, higiena), który opada, gdy jest
  gorzej, z falującą powierzchnią i ikoną; liczba po najechaniu myszą, a przy
  stanie krytycznym tarcza pulsuje na czerwono i pokazuje liczbę. Portfel jako
  złota moneta z kwotą. Ostrzeżenia (brudne ręce, żołądek) na karteczce pod
  tarczami.
- **Ekwipunek**: drewniany pasek na dole pośrodku — ręce (większy slot, złota
  ramka, gdy coś trzymasz) i 3 kieszenie z numerami klawiszy; nazwa trzymanej
  rzeczy nad paskiem, skróty pod nim.
- **Świat**: efekt na cały obraz gry — kontury tuszu tam, gdzie zmienia się
  kolor, ciepłe, lekko przygaszone barwy, faktura papieru i winieta. Napisy w
  świecie (nicki, dymki, strzałki schodów) są nad efektem, więc zostają ostre.
  (`--no-mood` wyłącza efekt.)
- **Postacie** narysowane od nowa, w duchu Don't Starve: duża okrągła głowa z
  dużymi ciemnymi oczami (z błyskiem), uszy, mały tułów-trapez, cienkie kończyny
  jak kreski, wszystko z konturem tuszu; fryzury (krótkie, długie, kok, kolce,
  kucyk, łysa), czapki, kask strażaka, krawat, fartuch i mop, opaska ochrony;
  krok, machanie rękami, siedzenie; w rękach kubek (z parą), laptop, karta,
  owoc; czynności: kropki (parzenie, pisanie), „zzz”, papieros z kłębami dymu,
  bańki mydlane, zielone smugi zapachu, kropla potu, parasol.
- **Dym w pomieszczeniu**: szara zasłona, której krycie rośnie nieliniowo z
  gęstością (w małym, zadymionym pomieszczeniu nic nie widać), i kłęby jak
  chmury z jednym konturem tuszu i miękkim światłocieniem, falujące powoli.
- **Pogoda**: deszcz jako skośne kreski z konturem i rozpryski, mgła jako
  dryfujące chmury, burza przyciemnia niebo.
- Podczas jazdy windą dym, czujki i nazwy pomieszczeń piętra są ukryte.
- Komputer w biurze: ciemna drewniana ramka monitora, bordowy pasek tytułu,
  pergaminowy ekran; pulpit: miękka tapeta z miastem o zmierzchu.

## 10.34 Okna, światło, kamera i ręcznie rysowana mapa

- **Okna**: część pomieszczeń ma okna w ścianach zewnętrznych (biura IT,
  Biznes, HR, zarząd, chill room, portiernia, wejście, sklep); łazienki,
  recepcja, korytarze i zaplecze — nie. Okna widać na ścianach.
- **Światło**: każde pomieszczenie ma własną jasność:
  - na zewnątrz — pora dnia × pogoda (pochmurno, mgła, deszcz, burza ciemniej);
  - z oknami — to samo, trochę słabiej; bez okien — prawie ciemno;
  - **włącznik** przy drzwiach (biura, łazienki, chill room, HR, zarząd,
    portiernia, zaplecze): E zapala / gasi lampę dla wszystkich — jasno i
    ciepło, nawet przy burzy; kabiny świecą lampą łazienki;
  - korytarze, hole, klatka, winda, sklep, parking wewnętrzny i recepcja są
    oświetlone zawsze;
  - rano lampy są zgaszone, o 22:00 gasną wszystkie.
- **Kamera**: kółko myszy albo `+` / `-` przybliża i oddala (0,6×–2×); nicki,
  dymki i nazwy pomieszczeń zostają tej samej wielkości.
- **Mapa narysowana od nowa** w stylu reszty gry: podłogi (deski, wykładzina,
  kafle, kamień, trawa, chodnik, asfalt, tory), ściany z konturem tuszu i
  tynkiem, drzwi, bramki, żywopłot, barierki i wszystkie meble — rysowane
  wektorowo raz do tekstury w potrójnej rozdzielczości. Ikony pulpitu też.
- Kłęby dymu nie wychodzą już za ściany (przy ścianach są mniejsze).
- Drzwi kabin (okienko wolne / zajęte), drzwi windy i wszystkie ikony
  przedmiotów (ekwipunek, półki, obiady, rzeczy w rękach i na podłodze)
  narysowane w tym samym stylu z konturem tuszu.

## 10.38 Dźwięk

- Wszystkie dźwięki są **syntetyzowane skryptem** (`tools/sounds/gen_sounds.py`
  → `client/sounds/*.wav`) — bez próbek i cudzych licencji.
- **Kroki** zależne od podłoża (podłoga, wykładzina, płytki / balkon, na
  zewnątrz); własne i ludzi obok.
- **Zdarzenia w świecie** (serwer wysyła `Sound` wszystkim na piętrze w
  promieniu 28 kafli, słychać je z miejsca zdarzenia): ekspres, kasa, bramka
  sklepu, gwizdek ochroniarza, winda (ding), zamek kabiny, włącznik światła,
  spłuczka, kran, zapalniczka, zmywarka, lodówka, szafka z kubkami,
  podniesienie / upuszczenie, jedzenie, picie.
- **Po stronie klienta**: klik przycisków, szelest okien, „blip” przy dymkach
  (wysokość zależna od osoby), powiadomienie o mailu, moneta przy wypłacie,
  grzmot po błyskawicy, syreny policji / straży na pojazdach, dzwonek alarmu.
- **Otoczenie**: ulica (głośno na zewnątrz, cicho w środku), szum biura,
  deszcz (na zewnątrz / stłumiony w środku).
- **Muzyka** (pętle lo-fi): w menu i przy tworzeniu postaci; spokojniejsza w
  domu (pulpit, noc, wybór dojazdu). W biurze tylko otoczenie.
- **Ustawienia**: suwaki Efekty / Otoczenie / Muzyka (szyny SFX, Ambient,
  Music), zapisywane w `user://settings.cfg`.

## 10.54 Tabliczki na drzwiach

- Nazw pomieszczeń nie ma już na podłodze. Przy drzwiach wisi mała mosiężna
  tabliczka (na ścianie obok, od strony, z której się ją czyta): nazwa
  pomieszczenia **za** drzwiami.
- **E** przy drzwiach (gdy E nie robi nic innego) pokazuje tabliczkę dużą, na
  środku ekranu; znika po E / Esc albo gdy odejdziesz. Podpowiedź: „[E]
  Przeczytaj tabliczkę” (przy drzwiach Zarządu dopisana do informacji o
  spotkaniach).
- **Toalety** mają na tabliczce sam znaczek: damska, męska, wózek (WC dla
  niepełnosprawnych, `accessible` w mapie) albo oba (toaleta przy portierni).
  Gdy obok jest dwoje drzwi, czyta się te, w które się patrzy — także stojąc
  już w progu. Kto wejdzie do WC dla niepełnosprawnych, rzuca żartem pod nosem
  („Tylko na chwilkę… Nikt nie widział, prawda?” i podobne).
- Tabliczki czyta się tylko z miejsc wspólnych (korytarz, hol, chill room,
  na zewnątrz) — w środku łazienki kabiny ich nie mają. Nie mają ich też
  rzeczy oczywiste: korytarze, hol, klatka schodowa, chill room, aneks
  kuchenny, sklep, balkon, windy. Liczy to klient z mapy
  (`MapData.plaque_at`), serwer nic o tym nie wie.
- Klatka schodowa nie ma napisu na mapie — nad drzwiami do niej świeci
  zielony znak **EXIT** (na obu piętrach; na półpiętrze strzałki „▸ Parter” /
  „▸ Piętro 1” zostają).

## 10.55 Poprawki piętra 1 i parteru (2026-10-04)

- Wejście na klatkę schodową na piętrze 1 jest z prawej strony (z holu
  windowego), tak jak na parterze; drzwi z zachodniego korytarza zniknęły.
- **Łazienka damska** w wyspie: wejście z korytarza do części z umywalką,
  dwie kabiny WC za nią (nie prosto z korytarza do kibla).
- Drzwi kabin w ścianach poziomych rysują się poziomo.
- Nazwy: „DevOps (Mordor)” → **Mordor**, „Pokój z biurkiem” → **Pokój do
  wyjebywania**; „Kasa” → **Kasjer**.
- Na parterze w fotelu w holu (tam, gdzie stała pani Maria) siedzi Paulina;
  pani Maria pokazuje się dopiero na obchód (15–16).
- Taca ze słodyczami stoi na blacie między sofami w chill roomie.

## 10.56 Usprawnienia (2026-10-04)

- **Terminal** (i komunikator, komentarze na tablicy) po Enterze pisze dalej
  — wcześniej pole kończyło edycję i wyglądało na zawieszone.
- **Komunikator**: kto się wylogował, zostaje na liście wiadomości
  prywatnych jako „poza biurem” (wyszarzony, bez rozmowy).
- **Krew**: dźgnięcie nożem — rozprysk kropli, na podłodze plama krwi
  (sprzątaczka: „Krew?! Co tu się działo… Ja już nic nie widziałam.”;
  inaczej znika o 22:00).
- **Szafka, apteczka, magazynek, barek** pokazują zawartość jak ekwipunek:
  sloty z obrazkami (klik albo klawisz numeru), „Zamknij” pod spodem.
- **Tab — menu akcji**: lista tego, co da się zrobić tu i teraz, z
  klawiszami (E przy rzeczy obok, F / Q z tym, co w rękach, kieszenie, G / X
  przy osobie obok, L w kabinie, R, czat, dziennik); wybór numerem albo
  kliknięciem.
- **Pomiń czekanie = głosowanie** wszystkich grających (większość, 30 s):
  przechodzi → wszyscy do domu z wypłatą i czas leci do rana. Rano zegar
  zwalnia — każdy wybiera dojazd (wcześniej po pominięciu wyjście z domu
  działo się od razu i nie dało się wybrać środka transportu).
- **Noc w domu regeneruje**: rano przy wyjściu do pracy postać jest
  wypoczęta, po śniadaniu i prysznicu, trzeźwa i zdrowa (potrzeby jak na
  starcie; dojazd zmienia je jak dotąd). Wcześniej potrzeby w domu stały w
  miejscu — kto wyszedł zmęczony i głodny, wracał rano taki sam.

