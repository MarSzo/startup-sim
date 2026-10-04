# Zakres i backlog

*Część [GDD](README.md). MVP, rozszerzenia, backlog pomysłów i rozbieżności z kodem.*

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
   Designer/ka — Produkt / IT; Sprzedaż — Sales; Marketing — Marketing); inne firmy
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

**Pomysły 2026-09-26 (II)** — kolejność: sklep → kubki → dym → interfejs.
- Kradzież w sklepie: ochrona, a potem policja — *zrobione (10.29)*.
- Kubki po kawie zostawiane gdzie popadnie; NPC sprzątaczka pod koniec dnia
  obchodzi pokoje i sprząta, przy dużej liczbie kubków narzeka — *zrobione
  (10.30)*.
- Papierosa można zapalić wszędzie; w środku dym rozchodzi się po pokoju, w
  części pomieszczeń czujka → alarm, straż pożarna, ewakuacja, kara —
  *zrobione (10.31)*.
- Interfejs w stylu gry: HUD (ekwipunek, statystyki), komputer, portal z
  ofertami — *zrobione (10.32, styl „papier i atrament”)*.

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
