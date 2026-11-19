# Faza 4 — izvještaj (provjere, reinstall, deinstalacija, greške, firewall)

Datum: 2026-11-19 · Grana: `main` · Testovi: 56 prolazi (39 unit + 17 integracionih), clippy `-D warnings` i rustfmt čisti.

## Urađeno po PLAN zadacima

| ID | Šta |
|---|---|
| 4.0 | Sesija A na konzoli nije izvedena → Faza 4 radi sa DESIGN defaultima; obje varijante nepotvrđenih detalja ostaju podržane (`RpiDialect`, mock flagovi). |
| 4.1 | Provjera za update/DLC (baza mora postojati, RPI-06), "Čeka osnovnu igru (#n)", nepoznata kategorija (PKG-06) — iz Faza 1/3, sada sa punim tekstovima. |
| 4.2 | **Dijalog 4.7 "Igra je već instalirana"**: ikona i podaci paketa, objašnjenje, "Primijeni na sve preostale pakete u redu", Preskoči je podrazumijevano i fokusirano, `Esc` = Preskoči. Otvara se automatski kad engine pita; u redu ostaje dugme "Odaberi…". |
| 4.3 / 4.4 | Reinstaliraj za igru (deinstalacija → provjera → instalacija) i "Instaliraj ponovo" za update (bez deinstalacije). |
| 4.5 | **Samostalna deinstalacija**: desni klik → "Deinstaliraj sa konzole…" i `Shift+Delete` (onemogućeno dok se isti naslov instalira), prvo provjera da li je instalirano, zatim **potvrda 4.11** (tekst po tipu, Odustani podrazumijevano); toolbar `⋯` → **"Deinstaliraj po ID…" (4.12)** sa validacijom Title ID / Content ID (CFG-03), dugme Provjeri, pa potvrda. Engine radi deinstalaciju u pozadini, igru provjerava ponovo (3×), 404 = "već nije bilo instalirano". |
| 4.6 | **Dijalog greške 4.8**: Šta / Zašto / Šta uraditi, prevod CE/BGFT kodova, tehnički detalji, konzolin log (zadnjih 200 linija), Kopiraj detalje, Otvori Log, Pokušaj ponovo. |
| 4.7 | EN + BS tekstovi "zašto" za svih 31 kod greške; tekstovi popunjavaju `{placeholder}` stvarnim detaljem. |
| 4.8 | **Prvo pokretanje**: firewall uputa 4.9 se otvara dok se ne označi "Ne prikazuj ponovo"; server se i dalje binduje tek kad red krene. |
| 4.9 | Banner SRV-01 nudi dugme **"Koristi port N"** (prvi slobodan port), koje odmah mijenja postavku. |
| 4.10 | **Firewall pravilo (DESIGN 9.8)**: `--firewall add <port> \| remove` (isti exe, elevirano kroz UAC, `netsh` bez shella, port se validira kao broj ≥ 1024), čitanje stanja preko PowerShella u pozadini; red u Postavkama: aktivno / pravilo za drugi port ("Ažuriraj pravilo") / nema pravila / politika (FW-04); poruke FW-02 (UAC odbijen) i FW-03. |

## Novi automatski testovi

- `engine::standalone_uninstall_item_and_by_id` — igra iz reda (provjera → deinstalacija → stavka nazad u Spreman), DLC po Content ID-u, naslov koji nije instaliran (404 → "već nije bilo").
- `engine::decision_apply_to_all` — jedna odluka sa "primijeni na sve" preskače i sljedeću instaliranu igru bez novog pitanja.
- `firewall::*` — elevirani korak prima samo broj porta (`8095 & calc` se odbija), parsiranje stanja.
- `dialogs::id_formats` — Title ID i Content ID formati.

## Test uživo (izolovani `%APPDATA%`, mock)

- Prvo pokretanje → firewall uputa sa stanjem "Nema pravila" i dugmetom "Dozvoli u firewallu" (`docs/faza-4/firewall-first-run.png`).
- Red stigne do igre koja je već na konzoli → dijalog 4.7, Preskoči fokusiran (`docs/faza-4/already-installed.png`).

## Nije provjereno / ograničenja

- Klik na "Dozvoli u firewallu" iz aplikacije (UAC) nisam pokretao — mehanizam je isti kao u spike-u S-5 koji je prošao sa tobom; probaj kad ti odgovara.
- Promjena IP adrese računara **usred prenosa** (SRV-02) i dalje se otkriva tek pri sljedećem pokretanju servera, ne odmah.
- Tekstovi o save podacima u 4.7/4.11 kažu "još nije potvrđeno" do Sesije B.
- Sesija A nije urađena; nepotvrđeni detalji RPI-ja su i dalje u `RpiDialect` sa obje varijante.

## Za CP-4

1. Pregled dijaloga (screenshotovi ili `cargo run --release`).
2. Pregled bosanskih tekstova grešaka (Log → klik na grešku, ili dijalog Detalji).
3. Odobrenje taga `v0.0.1-alpha.4` i starta Faze 5 (splash, animacije, i18n kompletiranje, pristupačnost, ikona exe-a).
