# Biuro i otoczenie

*Część [GDD](README.md). Ekspres, sklep, słodycze, ochrona, kubki i sprzątaczka, palenie, aneks kuchenny, telewizor i boombox.*

## 10.10 Ekspres do kawy (chill room)

Ustalenie 2026-09-26: ekspres działa jako **czynność**, bez wpływu na
statystyki — co daje kawa (energia, stres, koszt), zostaje otwartą kwestią
ekonomii (sekcja 7). Przy ekspresie: „[E] Zrób kawę” → „Parzę kawę…” (3 s,
ekspres zajęty dla innych: „Ekspres zajęty — chwilka.”) → „Kawa gotowa!” →
kubek w ręce przez 90 s, widoczny dla innych → „Kawa wypita.”

## 10.20 Sklep i pieniądze

- **Portfel** w złotówkach (grosze na serwerze), widoczny nad paskami potrzeb.
  Na razie jedyny przychód: **200 zł zaliczki** przy podpisaniu umowy w HR
  (dzienna pensja razem z dniami gry — backlog 9b).
- **Sklep na parterze** (przed bramkami, dostępny także dla gości):
  - **E przy półce** pokazuje towary z cenami; „Weź” (albo 1–9) wkłada towar do
    kieszeni / rąk jako **niezapłacony** (widać to w ekwipunku, z ceną);
  - **kasa** (NPC „Kasjer” za ladą): E = płacisz za wszystkie niezapłacone rzeczy;
    za mało pieniędzy — trzeba coś odłożyć;
  - **wyjście z niezapłaconym towarem**: bramka piszczy, towar zostaje w
    sklepie, stres +10.
- Półki: **Kanapki** (z serem 12 zł, z szynką 14 zł, wrap wege 13 zł),
  **Fast food** (hamburger 18 zł, frytki 9 zł — tylko w rękach), **Przekąski**
  (drożdżówka 6 zł, batonik 5 zł, chipsy 7 zł), **Napoje** (woda 4 zł,
  energetyk 8 zł, sok 6 zł), **Alkohol i papierosy** (piwo 7 zł, wino 25 zł,
  **małpka** — setka Żołądkowej Gorzkiej z miętą 10 zł, mieści się w kieszeni;
  papierosy 18 zł / 20 szt.).
- **F = zjedz / wypij** (niezapłaconego nie można): każdy towar zmienia potrzeby
  (np. kanapka głód −35…−40, energetyk energia +30 ale stres +8 i toaleta +10,
  piwo stres −15 i toaleta +20, małpka stres −20, energia −8, toaleta +5 —
  „Na odwagę przed review.”).
- **Palenie wymaga papierosów** (jeden z paczki na przerwę).
- Do przemyślenia: konsekwencje alkoholu w pracy, zwroty, promocje.

## 10.25 Słodycze w chill roomie i nieświeże owoce

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

## 10.29 Ochrona i policja w sklepie

- W sklepie przy drzwiach stoi **ochroniarz** (NPC „Ochrona”, czarny strój z
  żółtą opaską). Wyjście z niezapłaconym towarem: bramka piszczy, ochroniarz
  woła „Stać!” i **goni** złodzieja (trochę szybciej niż gracz, także po
  schodach i przez bramki).
- **Złapany przez ochronę**: towar wraca na półkę, upomnienie, +10 stresu. Jeśli
  w międzyczasie zapłacił przy kasie — tylko uwaga. Ochroniarz wraca pod drzwi.
- **Policja** przyjeżdża, gdy to **druga kradzież tego dnia** (po złapaniu przez
  ochronę) albo gdy złodziej **uciekł** ochronie (20 s pościgu bez skutku) lub
  ochroniarz jest zajęty innym pościgiem. Radiowóz (biało-niebieski, migający
  kogut) podjeżdża ulicą pod wejście, policjant wysiada i idzie po złodzieja
  **gdziekolwiek w budynku** (ma wszystkie uprawnienia).
- **Złapany przez policję**: mandat 300 zł (albo tyle, ile jest w portfelu; przy
  pustym — pouczenie), towar zabezpieczony, +25 stresu, „Ale wstyd…” — wszystko
  słychać w pokoju. Po 3 min pościgu policja odjeżdża, a mandat i tak schodzi z
  konta. Policjant wraca do radiowozu i odjeżdża.
- Licznik kradzieży zeruje się każdego ranka.

## 10.30 Kubki po kawie i sprzątaczka

- Po wypiciu kawy (albo gdy wystygnie i trzeba ją wylać) w rękach zostaje
  **pusty kubek**. Można go:
  - zostawić gdziekolwiek (Q — upuść), np. na stole czy przy biurku;
  - **umyć** przy umywalce w łazience (E) — znika;
  - podstawić pod **ekspres** (E) — kawa leci do tego samego kubka.
- Kubki leżą, dopóki ktoś ich nie podniesie albo nie przyjdzie sprzątaczka —
  zostają nawet po wyjściu gracza z gry.
- **Pani Maria** (NPC, turkusowy fartuch i mop; do 10.49 — Pani Krysia) siedzi w strefie zamkniętej
  na parterze (drzwi obsługi z holu) i do obchodu jej nie widać. **Między 15:00 a 16:00** (o losowej porze, co dzień innej) zaczyna obchód: idzie do najbliższego kubka (najpierw
  na swoim piętrze), zbiera wszystkie w zasięgu, chwilę wyciera stół i idzie
  dalej — po całym budynku (ma klucze wszędzie; kubków w zamkniętej kabinie nie
  zbierze).
- **Narzekanie**: 3 i więcej kubków w jednym pokoju — komentarz na miejscu („No
  nie… 3 kubki w jednym pokoju!”). Na koniec podsumowanie; przy 5 i więcej
  kubkach dziennie także wpis na **#ogólny** z rekordzistą dnia (kto zostawił
  najwięcej). Czysto — pochwała.
- **Kałuże po wpadkach**: w obchodzie idzie też do kałuż (jak do kubków —
  najbliższa, najpierw na jej piętrze), ściera je z komentarzem „Co za cham tu
  naszczał!”. Kałuża zrobiona po obchodzie leży do 22:00. Same kałuże bez
  kubków — bez pochwały za porządek.

## 10.31 Palenie wszędzie, dym i straż pożarna

- Papierosy w rękach + **F (użyj)** = zapalenie **w dowolnym miejscu** (30 s,
  jak przy popielniczce; ruch albo E gasi). Na zewnątrz i w strefie palenia dym
  od razu się rozwiewa.
- **Dym w środku** gromadzi się w pomieszczeniu zależnie od jego wielkości
  (kabina w toalecie — od razu gęsto, duże biuro — po kilkunastu sekundach),
  przenika przez drzwi do sąsiednich pomieszczeń i powoli się rozwiewa (przy
  drzwiach na zewnątrz szybciej). Widać go jako szarą mgłę z kłębami.
- Kto wejdzie w zadymione pomieszczenie (a nie pali), narzeka: „Kto tu pali?!”
  i ma trochę stresu.
- **Czujki dymu** (białe krążki z czerwoną diodą na suficie) są w: holu przy
  windach, wejściu, portierni, sklepie, recepcji, zarządzie, działach (IT,
  Biznes, HR) i korytarzu. **Nie ma** ich w łazienkach, kabinach, chill roomie,
  klatce schodowej i na zewnątrz — tam da się „po cichu”, choć dym z łazienki i
  tak wyjdzie na korytarz.
- Gęsty dym przy czujce = **alarm pożarowy**: ekran pulsuje na czerwono, napis
  „ALARM POŻAROWY — wyjdź z budynku!”, diody migają. Kto zostaje w środku,
  co kilka sekund dostaje przypomnienie i stres.
- Przyjeżdża **wóz strażacki** (czerwony, z drabiną i kogutem), **strażak**
  idzie do pomieszczenia z alarmem, sprawdza je (5 s), wietrzy, ogłasza
  fałszywy alarm i nakłada na palacza **karę 500 zł** (albo tyle, ile ma).
  Administracja budynku pisze na #ogólny, gdzie i o której był alarm i kto
  palił. Gdy strażak wróci do wozu — koniec alarmu.

## 10.35 Aneks kuchenny, menu startowe i menu gry

- **Aneks kuchenny** obok chill roomu: sprzęty w lewym dolnym rogu — wzdłuż
  zachodniej ściany ekspres, lodówka, zmywarka i zlew, przy południowej blat
  i szafka z kubkami (i nożami); stół wyżej, przy oknach. Każdy sprzęt ma
  przed sobą własny kafel (E bierze najbliższą rzecz). W chill roomie misa
  z owocami i płyn do dezynfekcji.
- **Kubki są policzone**: biuro ma 8 kubków. Kawa leci tylko do czystego kubka
  w rękach — najpierw E przy szafce. Wypita (albo wystygła) kawa = brudny
  kubek. Czysty kubek można odłożyć do szafki; brudny:
  - umyć w zlewie (kuchennym albo w łazience) — od razu czysty w rękach;
  - włożyć do zmywarki (do 8); E z pustymi rękami włącza ją (30 min gry),
    potem trzeba ją rozładować (E) — czyste kubki wracają do szafki.
  Gdy szafka jest pusta, a brudne kubki stoją po biurze — trzeba pozmywać.
  Sprzątaczka zebrane kubki wkłada do zmywarki i ją włącza; kubki gracza,
  który wyszedł z gry, wracają do szafki.
- **Lodówka** (E — okno): przechowanie jedzenia i napojów (do 10 rzeczy, z
  podpisem właściciela — każdy może wziąć każdą), **mleko do kawy** („Dolej do
  kawy” → kawa z mlekiem, trochę mniej stresu; karton mleka ze sklepu
  uzupełnia 10 porcji) i **firmowe napoje za darmo** (4 wody, 2 soki, co rano
  nowe).
- **Menu startowe**: ekran tytułowy (miasto o zmierzchu, zapalające się okna,
  chmury) — Graj (→ tworzenie postaci), Ustawienia (pełny ekran,
  oszczędzanie baterii, raporty awarii, przybliżenie kamery, głośności,
  mikrofon; przewijane w niskim oknie; efekt tuszu jest zawsze włączony;
  zapisywane w
  `user://settings.cfg`), Autorzy, Wyjdź.
  Z tworzenia postaci — „Wróć do menu”.
- **Menu gry pod Esc** (gdy nie jest otwarte żadne okno): Wróć do gry,
  Ustawienia, Wyjdź do menu (rozłącza), Wyjdź z gry. Gra na serwerze toczy się
  dalej; postać w tym czasie stoi. Na pulpicie w domu to samo pod przyciskiem
  „StartOS”.

## 10.51 Telewizor i boombox w chill roomie

- **Telewizor** na szafce przy południowej ścianie chill roomu (naprzeciw sof;
  do 2026-10-04 wisiał na ścianie i na nią zachodził, przed nim stał stół). Kanały rysowane w
  grze: *Kreskówki*, *Wiadomości* (pasek z newsami z biura), *Pogoda* (z
  pogody w grze), *Mecz* (gol co ~40 s) i *Przyroda* (akwarium).
- **Pilot** leży przy stoliku: kto ma go w rękach w chill roomie, F — wybór
  kanału albo „Wyłącz”. Wszyscy widzą to samo i w tym samym momencie
  (serwer pamięta, kiedy kanał się zaczął).
- **Boombox** (tylko w rękach): F — *Disco polo na full*, *Lo-fi do
  kodowania*, *Techno z piwnicy*, *Szanty z biura* albo „Wyłącz”. Muzyka gra
  tam, gdzie jest boombox — idzie za tym, kto go niesie, a odłożony gra z
  podłogi; słychać ją na tym samym piętrze, ciszej z daleka. Wyniesiony z
  budynku — cisza.
- Pilot i boombox wracają rano na swoje miejsca, jeśli nikt ich nie ma.
