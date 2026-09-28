# Wdrożenie na VPS

Serwer: Ubuntu (x86_64), dostęp `root` po SSH. Z tego komputera:

```bash
deploy/deploy.sh                 # pierwszy raz i każda aktualizacja: kod → VPS, build, restart
deploy/pull-cert.sh              # certyfikat logowania VPS-a do klienta (client/net/pins/)
deploy/pull-backups.sh           # dzienne kopie bazy z VPS-a do vps-backups/
```

Na VPS:

- usługa `startup-sim` (systemd) jako użytkownik `startup-sim`: gra UDP `7777`,
  logowanie HTTPS TCP `7778`; `systemctl status|restart startup-sim`,
  logi: `journalctl -u startup-sim -f`;
- dane: `/var/lib/startup-sim` (`world.db`, `tls/`, `backups/`);
- zapora `ufw`: SSH, 7777/udp, 7778/tcp (w Hetzner Cloud Firewall te same porty);
- administracja (jako `startup-sim`, w `/opt/startup-sim/src/server`):
  `sudo -u startup-sim /opt/startup-sim/bin/server --save /var/lib/startup-sim/world.db --list-accounts`
  (i `--reset-password <nick>`).

Zatrzymanie usługi zapisuje grę (SIGTERM), więc aktualizacja nic nie gubi.
