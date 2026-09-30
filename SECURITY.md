# Bezpieczeństwo

Gra ma konta graczy (hasła, sesje) i szyfrowany ruch, więc luki traktujemy
poważnie.

## Zgłaszanie luk

**Nie zgłaszaj luk w publicznych Issues.** Użyj prywatnego zgłoszenia na
GitHubie: zakładka **Security → Report a vulnerability** w tym repozytorium.
Opisz, czego dotyczy luka, jak ją odtworzyć i jakie może mieć skutki.

Odpowiemy możliwie szybko, ustalimy poprawkę i dopiero po jej wydaniu
opiszemy lukę publicznie (z podziękowaniem, jeśli chcesz).

## Co nas szczególnie interesuje

- logowanie i konta (`server/src/auth.rs`, `server/src/http.rs`),
- szyfrowanie ruchu gry (`server/src/crypto.rs`, `client/net/seal.gd`),
- dane postaci widoczne tylko dla serwera i gracza (e-mail, wiek, miejscowość),
- wszystko, co pozwala jednemu graczowi wpłynąć na innych poza zasadami gry
  (serwer jest autorytatywny).

Nie testuj na serwerze, na którym grają inni — uruchom własny
(`cd server && cargo run`).
