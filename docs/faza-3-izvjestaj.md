# Faza 3 — izvještaj (red čekanja end-to-end protiv mock-a)

Datum: 2026-11-19 · Grana: `main` · Testovi: 51 prolazi (36 unit + 15 integracionih), clippy `-D warnings` i rustfmt čisti.

## Urađeno po PLAN zadacima

| ID | Šta |
|---|---|
| 3.1 | Prazno stanje sa isprekidanom drop zonom; drag-over overlay (`hovered_files`) na bilo kojem tabu prebacuje na Red čekanja; ne-`.pkg` fajlovi se odbacuju u engine-u uz info toast. |
| 3.2 | "Dodaj fajlove" (filter `.pkg`) i "Dodaj folder" preko `rfd` na zasebnoj niti (UI ne blokira); `Ctrl+O` / `Ctrl+Shift+O`. |
| 3.3 | Red 72 px: `#n` ili ikona statusa, ikona igre iz `icon0.png` (48×48), naziv, badge tipa, verzija, veličina; drugi red Title ID + kontekst (greška sa detaljem, predprovjera, split, Content ID); treći red status, progress, %, brzina, ETA, akcije po stanju. Selekcija, kontekstni meni (pomjeri, otvori lokaciju fajla, kopiraj Title ID, ukloni). |
| 3.4 | Reorder: ručka za prevlačenje (aktivna stavka zaključana), `Alt+↑/↓`, `Alt+Home/End`, kontekstni meni. |
| 3.5 | Toolbar: Pokreni red (`Ctrl+Enter`), Zaustavi nakon trenutnog, Očisti završene; podnožje: broj paketa, preostala veličina, ukupni ETA. |
| 3.6 | Brojevi u fiksnim kolonama (mono font), optimističko "Pauziram…". |
| 3.7 | `motion.item.in` (visina 0→72 + alfa), `motion.item.out` (alfa pa visina, ostali redovi klize), `motion.reorder` (premješten red klizi sa stare pozicije); poštuje "Show animations". |
| 3.8 | Predprovjera nakon dodavanja (`is_exists` u pozadini): "Nije instalirano", "Već instalirano na konzoli", "Čeka osnovnu igru (#n)", "Osnovna igra nije instalirana". |
| 3.9 | Perzistencija i nastavak nakon restarta (iz Faze 1, sada vidljivo u listi: RPI-07 + "Pokušaj ponovo"). |
| 3.10 | Sprječavanje sleepa tokom prenosa (Faza 1); dijalozi **"Prenos je u toku"** pri zatvaranju prozora (Ostani = default/Esc) i **"Otkazati instalaciju i ukloniti iz reda?"** za aktivnu stavku. |

Dodatno: jednostavan dijalog **Detalji** greške (kod, šta, šta uraditi, detalji) — puni dijalog 4.8 sa konzolinim logom dolazi u Fazi 4. Tekstovi grešaka sada popunjavaju `{placeholder}` stvarnim detaljem (npr. "Nedostaju split dijelovi: _1").

## Test T3 protiv mock-a (uživo, izolovani `%APPDATA%`)

Folder iz `make_fixture` dodan odjednom, red pokrenut, mock sa `--installed CUSA00005`:

| Stavka | Rezultat |
|---|---|
| Split igra (3 dijela, 3 GB) | jedna stavka "split: 3 dijela" → **Završeno** |
| Igra | **Završeno** |
| Update za igru u redu | **Završeno** (poslije igre) |
| DLC | **Završeno** |
| Update bez osnovne igre | **Greška RPI-06** "Osnovna igra CUSA00777 nije instalirana", red nastavio |
| Igra već instalirana na konzoli | **Već instalirano** → Preskoči / Reinstaliraj |
| Split bez `_1` | **Neispravan PKG-04** "Nedostaju split dijelovi: _1" |
| Zip preimenovan u `.pkg` | **Neispravan PKG-01** |

Redoslijed nove grupe: igre → updatei po verziji → DLC → ostalo. U mirovanju se ekran ne crta (brojač 933 → 935 za 8 s, samo od snimanja ekrana). Screenshotovi: `docs/faza-3/queue-transferring.png`, `docs/faza-3/queue-finished.png`.

Pauza / nastavak / otkaz / stall (RPI-05) / bez preuzimanja (FW-01) / kod greške (RPI-04) / restart → nastavak su pokriveni automatskim testovima engine-a iz Faze 1 i rade kroz iste komande iz UI-ja.

## Pojednostavljenja (zamjenjuje ih Faza 4)

- "Već instalirano": umjesto dijaloga 4.7 su dugmad Preskoči/Reinstaliraj u samom redu (bez "primijeni na sve").
- Detalji greške: jednostavan dijalog bez konzolinog loga.

## Nije provjereno

- Ručno klikanje (drag & drop iz Explorera, ručka za prevlačenje, kontekstni meni, dijalog pri zatvaranju) — testirano samo kroz build i ekrane; molim da probaš.
- Indikator konzole u zaglavlju ostaje "nije provjereno" dok se ne pokrene Test veze, iako red uspješno radi sa konzolom.

## Za CP-3

1. Probaj sam: `cargo run --release -- --debug`, pa prevuci folder sa `.pkg` fajlovima (ili `cargo run --example make_fixture -- <folder>` za lažne pakete) i pokreni `mock-rpi` (`cargo run --bin mock-rpi -- --speed 20MB`), u Postavkama IP `127.0.0.1`, port mock-a `12801`.
2. Odobrenje taga `v0.0.1-alpha.3` i starta Faze 4.
3. **Sesija A na konzoli** (PLAN 7.1) — po planu potrebna prije Faze 4; bez nje Faza 4 ide sa defaultima i obje varijante u mock-u.
