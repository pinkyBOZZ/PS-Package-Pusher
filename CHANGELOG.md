# Changelog

All notable changes to PS Package Pusher. Versions follow `0.0.x` until the first stable release.

## 0.0.1

First public release (not released yet — waiting for testing on a real console).

- Drag & drop of `.pkg` files and folders, Add files / Add folder; split packages joined automatically.
- Package details read locally: name, Title ID, version, type (game / update / DLC), game icon.
- Check before every install: already installed game → Skip / Reinstall (with "apply to all"); update/DLC need their game.
- Queue: reorder, pause / resume / cancel, progress, speed, ETA; kept across restarts, interrupted transfers resume.
- Uninstall from the console from the queue or by Title ID / Content ID, always confirmed.
- Error dialog with what / why / what to do, console error-code explanations and the console log; Log tab with filters and search.
- Settings with connection test; Windows Firewall rule helper (UAC); local HTTP server with Range support (> 4 GB).
- Windows light/dark theme, animations (respects "Show animations"), splash screen, English and Bosnian.

### Fixed

- Installs failed with RPI-04 when the console requested the package file with a query string in the URL (rc.4).
- Installs were refused with RPI-01 (0x80990015) after an earlier failed install of the same package. A failed install now removes its leftover download from the console; connection-lost errors keep it so Retry can resume.
- Queue: an ETA of 100 hours or more is hidden (early estimates are unreliable), and the ETA column is wider so it no longer overlaps the speed and percent.
- The console indicator in the header no longer stays at "Console: not checked" until Test connection is pressed. It is checked at startup and after Settings are saved, and updates when the console becomes reachable or unreachable.
