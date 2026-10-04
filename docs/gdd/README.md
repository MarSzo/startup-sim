# Symulator pracy — Game Design Document

*Wersja robocza, aktualizowana razem z kodem. Sekcje 1–9 to design, sekcja 10 — ustalenia z implementacji (numeracja jak w dawnym jednym pliku, żeby odwołania „GDD 10.47” dalej działały).*

## [Wizja gry](wizja.md)

Koncepcja, świat, ścieżka gracza, struktura i rozwój firmy, pierwsze mechaniki.

- **1** Koncepcja
- **2** Założenia techniczne
- **3** Świat — budynek firmy
- **4** Ścieżka nowego gracza
- **5** Struktura firmy — start (startup)
- **6** Rozwój firmy (wspólny cel serwera)
- **7** Mechaniki
- **8** Otwarte kwestie

## [Zakres i backlog](zakres.md)

MVP, rozszerzenia, backlog pomysłów i rozbieżności z kodem.

- **9** Proponowany zakres MVP
- **9a** Rozszerzenia (ustalenia 2026-09-26)
- **9b** Backlog (pomysły 2026-09-26, do realizacji po kolei)
- **10** Implementacja — ustalenia i stan
- **10.1** Rozbieżności między GDD a obecnym kodem (do decyzji)

## [Technika](technika.md)

Stack, platformy, sieć, zapis, konta, szyfrowanie, wydajność, awarie, aktualizacje, czat głosowy.

- **10.2** Stack (zaimplementowany)
- **10.3** Platformy i crossplay
- **10.4** Etap 1 — pionowy wycinek sieci
- **10.40** Czat głosowy
- **10.41** Zapis postępu (etap 1 kont)
- **10.42** Konta i logowanie (etap 2)
- **10.43** Szyfrowanie gry (etap 3)
- **10.44** Wydajność klienta
- **10.45** Raporty awarii klienta
- **10.46** Aktualizacje klienta

## [Budynek i świat](swiat.md)

Mapy, windy, schody, czas, dojazd, pogoda, balkon, powrót do domu.

- **10.5** Budynek (mapy)
- **10.18** Windy
- **10.19** Klatka schodowa i półpiętro
- **10.21** Zegar, pory dnia i dni gry
- **10.22** Dojazd do pracy
- **10.23** Pogoda
- **10.33** Balkon
- **10.36** Powrót z pracy do domu

## [Rekrutacja i praca](kariera.md)

Portal, rozmowa, wdrożenie, umowy i widełki, wakaty, panel założyciela.

- **10.7** Wdrożenie: portier, recepcja, HR (dzień próbny → karta)
- **10.8** Portal z ofertami i rekrutacja
- **10.11** Tworzenie postaci (etap 1 z 9a)
- **10.12** Pulpit, portal, poczta i rozmowa online (etap 2 z 9a)
- **10.27** Wakaty i obsadzone stanowiska
- **10.28** Panel założyciela (bez płatności)
- **10.37** Poprawki: pulpit, zatrzymanie, eskorta
- **10.49** Widełki, umowa w HR, recepcja i pani Maria

## [Komputer i komunikacja](komputer.md)

Pulpit, komunikator, kalendarz, obiady, poczta i zadania, Kadry, przeglądarka, terminal, czat i powiadomienia.

- **10.14** Komputer i komunikator (etap 4 z 9a)
- **10.24** Kalendarz i spotkania z zarządem
- **10.26** Zamawianie obiadów
- **10.39** Firmowy komputer: pulpit, poczta, tablica zadań
- **10.50** Komputer w biurze: Kadry, przeglądarka, terminal
- **10.53** Czat, dziennik i powiadomienia, barek, zagubiony przechodzień

## [Postać](postac.md)

Ekwipunek, statystyki i potrzeby, toalety, higiena, alkohol, psoty i bójki, skręty i leki.

- **10.13** Ekwipunek (etap 3 z 9a)
- **10.15** Statystyki postaci (etap 5 z 9a)
- **10.16** Kabiny toaletowe
- **10.17** Higiena
- **10.47** Upojenie alkoholem
- **10.48** Psoty, bójki i obsługa na parterze
- **10.52** Skręty, apteczka i magazynek

## [Biuro i otoczenie](biuro.md)

Ekspres, sklep, słodycze, ochrona, kubki i sprzątaczka, palenie, aneks kuchenny, telewizor i boombox.

- **10.10** Ekspres do kawy (chill room)
- **10.20** Sklep i pieniądze
- **10.25** Słodycze w chill roomie i nieświeże owoce
- **10.29** Ochrona i policja w sklepie
- **10.30** Kubki po kawie i sprzątaczka
- **10.31** Palenie wszędzie, dym i straż pożarna
- **10.35** Aneks kuchenny, menu startowe i menu gry
- **10.51** Telewizor i boombox w chill roomie

## [Oprawa](oprawa.md)

Grafika „papier i atrament”, okna i światło, dźwięk.

- **10.9** Oprawa graficzna (placeholder → pixel art)
- **10.32** Wygląd „papier i atrament” (w duchu Don't Starve)
- **10.34** Okna, światło, kamera i ręcznie rysowana mapa
- **10.38** Dźwięk

## [Stan implementacji](stan.md)

Co już działa, pomiary, znane ograniczenia, następne kroki.

- **10.6** Stan implementacji
