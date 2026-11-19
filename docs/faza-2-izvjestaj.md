# Faza 2 — izvještaj (UI kostur, tema, Postavke, Log)

Datum: 2026-11-19 · Grana: `main` · Svi testovi prolaze, clippy `-D warnings` i rustfmt čisti.

## Urađeno po PLAN zadacima

| ID | Šta |
|---|---|
| 2.1 | `main.rs`: glow renderer, bez eframe persistence, prozor 1100×720 (min 880×560), pozicija i veličina se pamte u `config.json`, ikona prozora iz logoa, panic hook upisuje backtrace u log. Ruter tabova, `Ctrl+1/2/3`, `Ctrl+Tab`. |
| 2.2 | Fontovi Inter 4.1 (Regular, SemiBold) i Audiowide, OFL licence i SHA256 u `assets/fonts/SOURCES.txt`. |
| 2.3 | `theme.rs`: `Palette` dark/light tačno po DESIGN 2.2, pretapanje u linearnom RGB (300 ms, `cubic_in_out`), mapiranje na egui vizuale. |
| 2.4 | `anim.rs`: tokeni trajanja, poštuje Windows "Show animations", smoothing progressa (τ = 250 ms). |
| 2.5 | `widgets.rs`: dugmad (primarno / sekundarno / destruktivno / ghost, hover i press animacija, fokus prsten), badge, status (ikona + tekst), progress (gradijent i indeterminate ≤ 30 fps), banner, kartica, sunset traka. |
| 2.6 | Zaglavlje: sunset traka, logo + naziv (Audiowide), tabovi sa brojačima i animiranom podvlakom, indikator konzole (klik → Postavke), globalni banner. |
| 2.7 | Postavke: validacija u toku kucanja (IPv4, hostname poruka, portovi), Sačuvaj / Vrati podrazumijevano, jezik odmah, Test veze (F5) u pozadinskoj niti po DESIGN 6.5, Otvori folder. |
| 2.8 | Log: filteri po nivou, pretraga, proširivi unosi grešaka (Šta / Šta uraditi / detalji), Kopiraj, Kopiraj sve, Otvori folder. |
| 2.9 | Galerija tokena (samo uz `--debug`). |

Debug pomoćni argumenti (ne mijenjaju ponašanje aplikacije): `--debug` (DEBUG u log, brojač frame-ova, galerija), `--tab settings|log|gallery`, `--theme light|dark` (samo uz `--debug`, za screenshotove).

## Kriteriji prihvatanja

| Kriterij | Rezultat |
|---|---|
| WCAG kontrast iz `Palette` konstanti | test `app::theme::wcag_thresholds` zelen za obje teme (tekst ≥ 4.5, UI ≥ 3, badge, banneri, dugmad) |
| Promjena Windows teme uživo | radi (vlasnik potvrdio na S-2); pretapanje 300 ms implementirano |
| 0 repaint-a u mirovanju | brojač stoji na 7 na Queue / Settings / Log ekranu |
| Test veze protiv mock-a | test `connection_test_against_mock`: normalno bez grešaka, `--reject-all-403` → NET-03, `--legacy` → upozorenje za `/api/log`, ugašen mock → NET-01/02 |
| Sve labele u `en` i `bs` | test pariteta zelen |

## Nalaz: ikone i Inter

Inter ima vlastite glifove u private-use opsegu (U+E000…), na istim kodovima kao Phosphor, pa su se ikone prikazivale kao slova (`Ŝ`). Obrnut redoslijed nije rješenje: Phosphor mapira ASCII slova na ligature i tekst nestane. Rješenje: ikone u **zasebnoj font familiji**, ikona + tekst crtaju se kao jedan `LayoutJob` sa dva fonta. Upisano u DESIGN 2.5.

## Screenshotovi

`docs/faza-2/`: `queue`, `settings`, `log`, `gallery` × `dark`, `light`.

## Šta nije u Fazi 2 (po planu kasnije)

- Red čekanja (lista, dodavanje, drag & drop) → Faza 3; dugmad "Dodaj fajlove/folder" su sada prikazana ali onemogućena.
- Firewall red u Postavkama (DESIGN 9.8) → Faza 4 (4.10).
- Splash → Faza 5.

## Za CP-2

1. Pregled screenshotova; ako želiš, pokreni aplikaciju sam: `cargo run --release` (podaci idu u `%APPDATA%\PS Package Pusher`).
2. **Pregled bosanskih tekstova**: Postavke → Jezik → Bosanski (primjenjuje se odmah).
3. Odobrenje taga `v0.0.1-alpha.2` i starta Faze 3.
