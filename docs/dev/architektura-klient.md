# Architektura — klient

*Architektura: [przegląd](architektura.md) · [serwer](architektura-serwer.md) · [klient](architektura-klient.md) · [protokół](protokol.md)*

**Pulpit** (`ui/desktop.gd`, osobna warstwa nad grą): „StartOS” z ikonami
Przeglądarka / Poczta / Kosz, paskiem zadań i zegarem; okna (przeciągane):
przeglądarka z portalem i formularzem zgłoszeniowym (dane postaci + „Dlaczego
chcesz u nas pracować?” + zgoda), poczta (lista, podgląd, przyciski akcji),
rozmowa online (kafelki wideo: rekruterka i Twoja postać, pytania). Pokazuje
stan z serwera, ponawia swoje akcje, gdy serwer ich nie odnotował, i znika,
gdy przyjdą pierwsze snapshoty. Dopóki jest widoczny, postać nie
dostaje inputu. Dział gracza widać przy nicku („Ala · IT”, po umowie) i w F3.

**Wydajność rysowania.** W Godocie drogie jest nagrywanie poleceń
rysowania (`_draw` po `queue_redraw()`), a samo przesunięcie albo zmiana
`modulate` gotowego elementu jest prawie darmowe. Dlatego:
ekran tytułowy to warstwy rysowane raz (gwiazdy mrugają przez `modulate`
grupy, chmury płyną przez `position`, okna miasta co 0,5 s); światło
(`light_view.gd`) i dym (`smoke_view.gd`) przerysowują się tylko przy zmianie
(godzina, pogoda, lampa, przejście jasności; dym — gdy jest albo mrugnie
czujka) i rysują pokoje jako scalone prostokąty (`game/tile_rects.gd`), nie
kafel po kaflu; plakietki potrzeb mają szkło i obręcz rysowane raz, ciecz to
jeden wielokąt ~30 razy/s, pulsowanie to `scale`; deszcz / mgła tylko gdy są.
Limit klatek: `run/max_fps=60`, `Settings.apply_fps` (30 przy oszczędzaniu
baterii, 20 w tle). Pomiar: `--perf` wypisuje co 2 s FPS, wywołania
rysowania, liczbę elementów i węzły, które przerysowują się najczęściej.

**Raporty awarii** (`net/crash_reports.gd` → `POST /api/crash` →
`server/src/crash.rs`): sesja zapisuje „running” w `user://session.cfg`, a przy
zwykłym wyjściu (`main._exit_tree`) „clean”; „running” na starcie = awaria.
Dziennik poprzedniej sesji to najnowszy `user://logs/godot<data>.log` (Godot
odkłada go przy starcie). Po zgodzie (albo „zawsze”) klient wysyła koniec
logu przez `AuthClient.send_crash` (to samo przypinanie certyfikatu co
logowanie). Serwer: limity na adres i godzinę, usuwa znaki sterujące, plik na
raport, rotacja do 200. Tylko w wydanych wersjach (`--crash-test` w edytorze).

**Aktualizacje** (`net/updates.gd`): przy starcie (tylko wydane wersje albo
`--update-check`) `GET api.github.com/repos/<repo>/releases/latest`,
porównanie `tag_name` z `application/config/version` (`is_newer`, po
częściach); nowsze → panel na `update_layer` (warstwa 60, nad wszystkim) z
`OS.shell_open` na stronę wydania. `Reject(2)` (zła wersja) pokazuje ten sam
panel jako wymagany. `--pretend-version` do testów.

**Połączenie** (`net_client.gd`): parsowanie adresów z IPv6, rozwiązywanie
nazw `TYPE_ANY`, nowe gniazdo po 1,5 s ciszy lub powrocie z tła (ta sama
sesja), automatyczne ponowne łączenie przez 30 s po utracie sesji — `main.gd`
zostawia wtedy scenę gry, a `game.gd` zamraża się (`on_reconnecting`) i
czyści stan po nowym `Welcome` (`reset_session`).

**Predykcja własnej postaci** (`game.gd`): w `_physics_process` (60 Hz)
klient próbkuje klawisze, nadaje inputowi `seq`, od razu liczy nową pozycję
(`movement.gd`) i wysyła `Input` z 4 ostatnimi inputami (redundancja).
Render interpoluje między dwoma ostatnimi krokami fizyki
(`Engine.get_physics_interpolation_fraction()`), więc ruch jest płynny przy
dowolnym odświeżaniu monitora.

**Rekoncyliacja**: z każdym nowym tickiem klient bierze stan serwera
(`floor`, pozycja, `self_lock`, `self_prev_input`), usuwa inputy
`≤ last_input_seq` i odtwarza pozostałe. Zmiana piętra w wyniku korekty
przełącza widok bez wygładzania. Jeśli wynik różni się
od predykcji (np. zgubione 4+ pakiety inputu z rzędu), różnica trafia do
`error_offset`, który wygasa wykładniczo (~70 ms) — bez teleportów. Przy
jednakowej symulacji po obu stronach korekt praktycznie nie ma (0 w testach).

**Interpolacja innych graczy** (`remote_player.gd`): próbki `(tick, pozycja)`
w buforze; render w czasie `est_tick − 2 ticki` (100 ms). `est_tick` rośnie z
zegarem lokalnym i jest łagodnie (10%/snapshot) korygowany do ticku ostatnio
odebranego snapshotu; przy dużym rozjeździe (>5 ticków) jest przestawiany.
Gdy brakuje danych, krótka ekstrapolacja (max 2 ticki), potem stop.

**Widoczność**: przy zmianie piętra klient usuwa wszystkich zdalnych graczy;
przy zmianie pokoju — tych, których nie ma w nowym snapshocie (osoby widoczne
z obu pokoi, np. portier, zostają bez mrugnięcia). Gracz
nieobecny w snapshotach przez 5 ticków znika.

**Grafika** — proceduralny pixel art (16 px), bez zewnętrznych plików:
- `map_art.gd` rysuje piętro raz, przy wczytaniu (~20–40 ms): podłogi z
  wariantami tekstur (bez powtarzalnego wzoru), ściany w rzucie 3/4 (ciemny
  wierzch, jasny front z listwą i obrazkami tam, gdzie widać pokój poniżej),
  cienie ścian, drzwi / bramki / szklane drzwi / schody / winda, a meble jako
  całe obiekty (spójne grupy tego samego znaku: biurka z monitorami i
  krzesłami, lady, regały z towarem, sofa, stoły z krzesłami, rośliny, szafy
  serwerowe, ławka, popielniczka, toalety, umywalki, samochody z liniami
  miejsc). Typ mebla bierze się z legendy mapy; kolizje zależą tylko od
  `solid`, więc zmiana wyglądu nie rusza symulacji.
- `player_view.gd` rysuje postać prostokątami w `_draw()`: głowa, fryzura
  (6 stylów), koszula, ręce, spodnie, buty; 4 kierunki; cykl chodu liczony z
  przebytej drogi (ten sam dla własnej postaci i interpolowanych innych).
  Wygląd gracza wybiera on sam na ekranie tworzenia postaci (indeksy palet w
  `PlayerInfo`); przed nadejściem `PlayerInfo` — zastępczy wygląd z id; NPC mają stroje
  (portier: mundur i czapka, personel: koszula z krawatem). Własna postać ma
  jasny obrys.

**Render**: każde piętro to jeden sprite (widoczne tylko bieżące), postacie to
`Node2D._draw()` z `Label`em (y-sort). Kamera `Camera2D` z zoomem 3×, bez
wygładzania, z limitami mapy. Etykiety (strzałki schodów) mają skalę `1/zoom` i rozmiar
czcionki ekranowej, więc są ostre mimo zoomu. Nazw pokoi na mapie nie ma: są
na tabliczkach przy drzwiach — `MapData.plaque_at` (drzwi obok, pokój po
drugiej stronie), małe tabliczki rysuje `map_painter.gd`, dużą na środku
ekranu `ui/door_plaque.gd` (E, gdy podpowiedź to „Przeczytaj tabliczkę”).

**Podpowiedzi** na dole ekranu: „[E] Wezwij windę” / „Winda jedzie…” przy
drzwiach windy, „[E] Jedź na: …” w kabinie,
„[E] Porozmawiaj: Portier” przy NPC, informacja o wymaganej przepustce przed
bramką. **NPC** rysowane są w mundurze z czapką; wypowiedzi (`Say`) pokazują się
w dymku nad postacią (także gdy mówiący dopiero wejdzie w pole widzenia) i w
logu w lewym dolnym rogu.

**F3** (`debug_overlay.gd`): FPS, ping, tick serwera i czas renderu, piętro i pokój,
widoczni gracze, id/kafel, inputy w locie, liczba korekt, procent klatek z
pustym buforem interpolacji, transfer.
