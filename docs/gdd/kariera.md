# Rekrutacja i praca

*Część [GDD](README.md). Portal, rozmowa, wdrożenie, umowy i widełki, wakaty, panel założyciela.*

## 10.7 Wdrożenie: portier, recepcja, HR (dzień próbny → karta)

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

## 10.8 Portal z ofertami i rekrutacja

Ustalenia 2026-09-26: **portal na starcie** (zgodnie z sekcją 4), quiz
**3 pytania, 2 poprawne = przyjęcie**, ponowna próba od razu (inne pytania)
lub inna oferta; pytania **humorystyczne**, w klimacie startupu, z jedną
poprawną odpowiedzią. Każde stanowisko ma **25 pytań**; postać dostaje
najpierw te, których jeszcze nie widziała (pamięć per stanowisko, w zapisie
gry; pytanie rozpoznawane po treści), a po przejściu całej puli zaczyna się
nowa runda.

- Po połączeniu gracz widzi „Portal z ofertami pracy · Startup Sim sp. z o.o.”
  z dwiema ofertami: **Programista/ka** (dział Produkt / IT) i **Marketing i
  sprzedaż** (dział Biznes). Nazwa firmy to zaślepka (otwarta kwestia z sekcji 8).
- „Aplikuj” → 3 losowe pytania z puli oferty (po 8 w puli), odpowiedzi w losowej
  kolejności → wynik: przyjęcie („zapraszamy na dzień próbny”) albo „Tym razem
  się nie udało” z możliwością ponownej próby.
- Po przyjęciu gracz pojawia się przed budynkiem i przechodzi wdrożenie (10.7);
  HR podpisuje umowę **na dział z rekrutacji**: „Umowa podpisana — witamy w
  dziale Produkt / IT! Oto karta pracownika.” Od tej chwili inni widzą przy
  nicku dział („Ala · IT”).
- Pytania i oferty są w `server/data/recruitment.json` (edycja bez zmiany kodu;
  pierwsza odpowiedź w pliku jest poprawna — gra ją tasuje). Ocenia serwer.

## 10.11 Tworzenie postaci (etap 1 z 9a)

Ekran startowy to tworzenie postaci: imię, płeć (kobieta / mężczyzna / inna),
wiek (18–70), miejscowość, e-mail postaci oraz wygląd — kolor skóry (4),
fryzura (6: krótkie, długie, kok, jeżyk, kucyk, łysa głowa), kolor włosów (7),
koszula (10), spodnie (5) — z podglądem na żywo („Obróć”, „Losuj wygląd”).
Serwer sprawdza dane; innym graczom pokazuje tylko imię, płeć i wygląd.
Ostatnia postać jest zapamiętywana lokalnie.

## 10.12 Pulpit, portal, poczta i rozmowa online (etap 2 z 9a)

Zastępuje prosty portal z 10.8. Po połączeniu gracz widzi pulpit komputera
(„StartOS”): Przeglądarka, Poczta, Kosz, pasek zadań z zegarem.
- **Przeglądarka** → portal „praca.example”: nasz startup szuka na 4
  stanowiska (Programista/ka, Designer/ka — Produkt / IT; Specjalista/ka ds.
  sprzedaży — Sales, ds. marketingu — Marketing) + 4 fikcyjne firmy (Korpo-Bank S.A.,
  Mega Software Inc., Pizzeria u Stefana, Agencja Kreatywna BUZZ) z
  humorystycznymi ogłoszeniami. Formularz: dane postaci, „Dlaczego chcesz u
  nas pracować?”, zgoda na przetwarzanie danych.
- **Poczta**: po ~4 s zaproszenie na rozmowę (albo zabawna odmowa od innej
  firmy — lub cisza); powiadomienie i licznik nieprzeczytanych.
- **Rozmowa online**: okno wideorozmowy (Kasia z HR i Twoja postać), 3
  pytania z puli stanowiska (po 6–8, humorystyczne odpowiedzi), wynik → mail:
  zaproszenie na dzień próbny z przyciskiem „Idę do biura” albo podziękowanie
  (można aplikować ponownie).

## 10.27 Wakaty i obsadzone stanowiska

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

## 10.28 Panel założyciela (bez płatności)

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
  - **stanowiska** (do 10): dodawanie („Nowe stanowisko”: nazwa 3–40
    znaków, dowolny dział poza Zarządem, zestaw pytań na rozmowę, opis),
    zmiana nazwy, działu i zestawu pytań, liczba miejsc (−/+, 0–5), opis,
    **usuwanie** — kandydaci w trakcie rekrutacji dostają maila
    „Rekrutacja zakończona”, zatrudnieni zostają. Zestawy pytań (po 25):
    Programowanie, Design, Sprzedaż, Marketing, Ogólne (praca w startupie).
    Na start firma ma 4 stanowiska z pliku danych; zmiany są w zapisie gry;
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

## 10.37 Poprawki: pulpit, zatrzymanie, eskorta

- Pierwszy dzień (szukanie pracy): sam pulpit, przeglądarkę otwiera się ikoną.
- Formularz zgłoszeniowy: wartości w jednej linii, zgoda czytelna (ciemny
  tekst także po najechaniu / zaznaczeniu, bez ramki przycisku).
- **Złapany** przez ochroniarza stoi 3 s, przez policję 6 s (status
  „zatrzymany”, ruch ignorowany).
- Portier i recepcjonistka po doprowadzeniu gościa stoją jeszcze 6 s (czas na
  przeczytanie dymka), dymki wiszą dłużej.
- Podpowiedzi w aneksie: najbliższa rzecz (ekspres, owoce, płyn nie są już
  opisywane jako szafka / zlew / lodówka).

## 10.49 Widełki, umowa w HR, recepcja i pani Maria

- **Widełki w ogłoszeniach** (zł brutto miesięcznie), np. Programista/ka
  8 000–12 000 zł. W formularzu trzeba podać **oczekiwane wynagrodzenie** i
  **formę zatrudnienia**: umowa o pracę, B2B albo umowa zlecenie (tylko z
  zaznaczonym „Jestem studentem / studentką” i poniżej 26 lat). Oczekiwania
  powyżej widełek — mail z grzeczną odmową zamiast zaproszenia na rozmowę.
- **Umowa w HR**: zamiast od razu podpisywać, HR pokazuje umowę — kwota jest
  10–25% niższa niż uzgodniona („drobna korekta, standard w branży”), na B2B
  o 20% wyższa niż na umowie o pracę. *Podpisuję*: karta, laptop, praca;
  kwota ustala stawkę godzinową (miesiąc = 168 h), na umowie o pracę także
  200 zł zaliczki (B2B i zlecenie — bez zaliczki). *Rezygnuję*: HR odprowadza
  na portiernię, pani Wiesia zabiera przepustkę i po chwili wraca się na
  portal z ofertami (mail „Rezygnacja z umowy”, miejsce znów wolne).
- **Recepcja** zaczepia przechodzących pracowników (9:00–13:00, raz dziennie):
  „Zamówił/a już Pan/Pani obiad?” — jeśli jeszcze nie.
- **Pani Maria** (dawniej Pani Krysia) robi popołudniowy obchód (15–16) i
  bardzo dużo mówi: każdemu obok co około minutę opowiada coś z życia (Zbyszek,
  wnuczek, działka) albo z miasta (ceny na rynku, tramwaje, dzik w parku). Paulina
  dalej tylko siedzi w fotelu przy wejściu.
