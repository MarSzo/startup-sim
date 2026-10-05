# Wdrożenie na VPS

Serwer: Ubuntu (x86_64), dostęp `root` po SSH — **tylko z tailnetu**
(Tailscale, maszyna `startup-sim`). Z tego komputera (w tailnecie):

```bash
deploy/deploy.sh                 # pierwszy raz i każda aktualizacja: kod → VPS, build, restart
deploy/pull-cert.sh              # certyfikat logowania VPS-a do klienta (client/net/pins/)
deploy/pull-backups.sh           # dzienne kopie bazy z VPS-a do vps-backups/
deploy/pull-crashes.sh           # raporty awarii klientów do crash-reports/
```

Każde `deploy.sh` zapisuje się też na GitHubie jako wdrożenie środowiska
**serwer-testowy** (zakładka *Deployments* w repo i przy commitach): który
commit, kiedy, w toku / udane / nieudane. Potrzebne: zalogowane `gh` i
wypchnięty commit; bez tego (albo z `NO_GITHUB_DEPLOY=1`) wdrożenie idzie
normalnie, tylko bez wpisu. Adres serwera nie trafia na GitHuba. Lokalne,
niezacommitowane zmiany w `server/`, `client/maps/` albo `deploy/` są
zaznaczone w opisie („+ lokalne zmiany”).

SSH tylko z tailnetu (jednorazowo, na VPS jako root, póki SSH jest publiczny):

```bash
bash /opt/startup-sim/deploy/remote-tailscale.sh   # wypisze link logowania do Tailscale
# w panelu Tailscale: maszyna startup-sim → „Disable key expiry”
# z komputera sprawdź: ssh root@startup-sim
bash /opt/startup-sim/deploy/ssh-tailnet-only.sh   # zamyka publiczny port 22
```

Porty gry (7777/udp, 7778/tcp) zostają publiczne. Awaryjnie (tailnet nie
działa): konsola w panelu Hetznera. W Hetzner Cloud Firewall usuń regułę
22/tcp (ruch Tailscale idzie wychodząco, nie potrzebuje otwartych portów).

Na VPS:

- usługa `startup-sim` (systemd) jako użytkownik `startup-sim`: gra UDP `7777`,
  logowanie HTTPS TCP `7778`; `systemctl status|restart startup-sim`,
  logi: `journalctl -u startup-sim -f`;
- dane: `/var/lib/startup-sim` (`world.db`, `tls/`, `backups/`, `crashes/` —
  raporty awarii klientów, najnowsze 200, z adresem IP nadawcy);
- zapora `ufw`: SSH tylko na `tailscale0`, 7777/udp, 7778/tcp (w Hetzner Cloud Firewall tylko porty gry);
- administracja (jako `startup-sim`, w `/opt/startup-sim/src/server`):
  `sudo -u startup-sim /opt/startup-sim/bin/server --save /var/lib/startup-sim/world.db --list-accounts`
  (i `--reset-password <nick>`).

Zatrzymanie usługi zapisuje grę (SIGTERM), więc aktualizacja nic nie gubi.

Klient: lista serwerów w `client/net/servers.cfg` (poza repozytorium — wzór
w `servers.example.cfg`; gracz widzi tylko nazwę, np. „Serwer testowy”, adres
zostaje w środku). Bez tego pliku, a zawsze z edytora, dochodzi „Serwer
lokalny (dev)” (`127.0.0.1:7777`).
Certyfikat logowania z `deploy/pull-cert.sh <host:port>` leży w
`client/net/pins/` (też poza repozytorium) — klient ufa mu od pierwszego
połączenia. Profil eksportu ma już `net/servers.cfg` i `net/pins/*.pem` w
„Filters to export non-resource files”.
Po zmianie certyfikatu na serwerze (np. nowy `tls/`) trzeba go ponownie
ściągnąć i wydać nowego klienta.
