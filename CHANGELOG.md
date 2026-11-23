# Changelog

All notable changes to PS Package Pusher. Versions follow Semantic Versioning.

## 1.0.2

### Changed

- A finished install is now removed from the PS4 Downloads list (Notifications → Downloads) about a minute after it completes, so the list no longer fills up.
- The app version and a link to the GitHub releases are shown at the bottom of Settings.
- Cancelling an install is now written to the log.

### Fixed

- In a narrow window the Pause / Cancel buttons no longer overlap the progress numbers; the ETA (and if needed the speed) is hidden when there is no room.

## 1.0.1

### Fixed

- Opening Settings (or the firewall notice) no longer shows an empty PowerShell window for a few seconds while the firewall rule is checked.

## 1.0.0

First public release.

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
