# Przebieg gry

*Dla graczy · [sterowanie](sterowanie.md) · [dokumentacja](../README.md)*

## Konto i postać

Na starcie logujesz się: nick to login i imię postaci. „Zapamiętaj mnie”
trzyma na komputerze tylko token, nie hasło. Nowe konto tworzy postać — imię,
płeć, wiek, miejscowość, e-mail postaci i wygląd z podglądem (to dane
fikcyjnej postaci; serwer nikomu ich nie pokazuje). Kolejne logowania wchodzą
prosto do gry.

Serwer sam robi sobie certyfikat HTTPS — klient zapamiętuje go przy pierwszym
połączeniu i ostrzega, jeśli się zmieni. Po zalogowaniu cały ruch gry jest
szyfrowany kluczem sesji.

## Rekrutacja

Pierwszy dzień zaczynasz w domu przy komputerze:

1. w **przeglądarce** jest portal z ogłoszeniami (kilka firm; zatrudnia tylko
   nasz startup) — wypełniasz formularz z oczekiwaną pensją i formą umowy;
2. po chwili w **Poczcie** czeka zaproszenie na **rozmowę online**: 3 pytania
   z puli 25 na stanowisko (bez powtórek, dopóki nie przejdziesz całej puli);
   2 poprawne odpowiedzi = przyjęcie;
3. potem przychodzi zaproszenie na dzień próbny — „Idę do biura”.

## Pierwszy dzień w biurze

Startujesz przed budynkiem bez przepustki. Do holu wejdziesz, ale windy,
klatka schodowa i parking otwierają się tylko kartą.

1. **Portiernia** (E): portier da przepustkę gościa i zaprowadzi na recepcję
   na piętrze 1 (schodami).
2. **Recepcja** (E) zaprowadzi do HR.
3. **HR** (E) przedstawi umowę (stawka może być trochę niższa niż na
   rozmowie) — po podpisaniu dostajesz kartę pracownika i laptop.

## Codzienność

- Do pracy dojeżdżasz co rano wybranym sposobem (pieszo, rowerem, autem,
  tramwajem, taksówką) — w deszczu też.
- Przy biurku: tablica zadań działu, poczta, komunikator, kalendarz spotkań
  z zarządem, zamawianie obiadów, Kadry (urlop, umowa), przeglądarka, terminal.
- Potrzeby: głód, energia, pęcherz, higiena… Kawa w aneksie, słodycze w chill
  roomie, zakupy w sklepie na parterze.
- Po pracy — powrót do domu i wypłata za przepracowany czas.
- Założyciel firmy ma panel: stanowiska, rekrutacja, pensje, działy.

Szczegóły mechanik: [GDD](../gdd/README.md).
