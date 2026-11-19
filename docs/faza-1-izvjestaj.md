# Faza 1 — izvještaj (jezgro bez UI-ja)

Datum: 2026-11-19 · Grana: `main` · Testovi: **51 prolaze** (32 unit + 19 integracionih), 3 uzastopna puna prolaza bez slučajnih padova · clippy `-D warnings` i rustfmt čisti.

## Urađeno po PLAN zadacima

| ID | Modul | Šta radi |
|---|---|---|
| 1.1 | `storage` | `%APPDATA%\PS Package Pusher`, atomski upis (tmp → fsync → rename), oštećen JSON → `*.broken-<ts>.json` |
| 1.2 | `config` | defaulti 6.6, `#[serde(default)]`, nevalidan IP konzole se briše uz upozorenje, nepoznat jezik → `en` |
| 1.3 | `logging` | format linije 7.8, dnevni fajl + `.N` preko 10 MB, čuva 14 fajlova, prsten 5000 za UI, `%USERPROFILE%` anonimizacija (i u konzolinom logu), tijela skraćena na 2 KB, DEBUG u fajl samo uz `debug` |
| 1.4 | `i18n` + `lang/*.json` | EN/BS, fallback bs → en → ključ, plural (bs one/few/other), veličine 1024-bazno (`61,3 GB`); svi kodovi grešaka imaju `what` + `action` u oba jezika |
| 1.5 | `errors` | katalog kodova iz DESIGN 10 (31 kod), `ErrorReport`, tabela 10.1 (`verified: false`), skener `CE-xxxxx-x` / `0x8099xxxx` bez regex crate-a |
| 1.6 | `net` | IPv4 validacija (hostname, 0.0.0.0, broadcast, multicast), PC IP preko UDP rute, /24 heuristika |
| 1.7 | `pkg::header`, `pkg::sfo` | parser po 9.5 + PLAN 3, PKG-03 politika (Q3), ikona dekodirana uz limit 1024×1024 |
| 1.8 | `pkg::scan` | rekurzija bez praćenja symlink/junction, dubina 16, split grupisanje, samostalan `_N` bez `_0` = običan paket (nalaz S-6), redoslijed igra → update po verziji → DLC |
| 1.9 | `rpi` | klijent (ureq 3, bez proxyja, bez redirecta, `http_status_as_error(false)`), klasifikacija 9.1, tolerantni brojevi/bool/`error`, `RpiDialect` za sve nepotvrđene detalje, `/api/log` POST↔GET fallback sa kešom |
| 1.10 | `mock` + `mock-rpi.exe` | ponašanje 8.1, scenariji 8.2, plus flagovi za obje varijante: `--exists-format`, `--log-method`, `--uninstall-fields`, `--notfound-style`, `--origin-check`, `--num-format`; povlači pakete Range zahtjevima kao BGFT, ponavlja dok PC nestane do 30 s |
| 1.11 | `server` | vlastiti GET/HEAD server (odluka S-4): Range > 4 GB, 416, 404/405, `/healthz`, 32 konekcije → 503 (nakon čitanja zahtjeva), tihi drop ne-whitelist IP-a, SRV-03/SRV-04 događaji, DEBUG linija po zahtjevu |
| 1.12 | `engine::queue` | stanja 5.1, prelazi 5.2 kao čista funkcija, perzistencija 7.6, EMA brzina/ETA 9.2 |
| 1.13 | `engine` | nit sa `recv_timeout`, komande/događaji 7.3, tok 6.2 (provjera → odluka → deinstalacija → install → prenos → potvrda), pauza/nastavak/otkaz, FW-01 i stall detekcija, globalni stop + banner, nastavak taska nakon restarta (9.3), `queue.json` sa debounce, `keep_awake` tokom prenosa, sva vremena u `EngineTimings` |
| 1.14 | `winapi` | `SetThreadExecutionState`, "Show animations", lokalno vrijeme |
| 1.15 | `pkg::fixture`, `examples/make_fixture.rs` | sintetički pkg (header + entry tabela + param.sfo + PNG), split, > 4 GB sparse, oštećeni; ručni folder za UI |
| 1.16 | `examples/headless.rs` | demo za CP-1 (transkript ispod) |

Dodatno: CI koristi `actions/checkout@v7` i `actions/cache@v6` (upozorenje o Node 20 riješeno) i ima **provjeru privatnosti** — build pada ako se u repou pojavi stvarna putanja korisničkog profila.

## Testovi po DESIGN 8.3

| Scenarij 8.3 | Test |
|---|---|
| uspješna instalacija + split (3 dijela) + redoslijed + `queue.json` | `engine::success_split_order_and_persistence` |
| već instalirano: Preskoči i Reinstaliraj | `engine::already_installed_skip_and_reinstall` |
| update bez baze → RPI-06 | `engine::update_without_base_fails_rpi06` |
| stall → RPI-05, kod greške → RPI-04, bez preuzimanja → FW-01 | `engine::stall_fail_code_and_fw01` |
| pauza (ne računa se kao stall) / nastavak / otkaz | `engine::pause_resume_cancel` |
| restart sa `interrupted` stavkom → nastavak istog taska | `engine::restart_resumes_interrupted_task` |
| neispravni ulazi (PKG-01, PKG-04, duplikat putanje) | `engine::invalid_inputs` |
| > 4 GB Range (offseti iznad `u32::MAX`, sparse 5 GB) | `server::range_over_4gb_and_errors`, `pkg::scan::split_fixture_over_4gb` |
| obje varijante nepotvrđenih detalja | `rpi_mock::*` (exists format × brojevi, uninstall polja, `/api/log` metoda, legacy) |
| parser bez panike na 10 000 nasumičnih/skraćenih ulaza | `pkg::header::no_panic_fuzz` |

## Headless demo (skraćeno)

```
=== scenario: ok ===
  0.01s Test Game CUSA10001      Checking
  0.27s Test Game CUSA10001      Transferring
  1.06s Test Game CUSA10001      Transferring  81.0%
  1.32s Test Game CUSA10001      Installing 100.0% 14.4 MB/s ETA 0s
  1.58s Test Game CUSA10001      Done 100.0% 14.4 MB/s ETA 0s
=== scenario: split ===            (3 dijela, 24 MB) → Done za 2,1 s
=== scenario: stall ===            6 % pa ništa → 3.71s Failed RPI-05
=== scenario: nofetch ===          konzola ne traži fajl → 3.47s Failed FW-01
=== scenario: failcode ===         25 % → 0.80s Failed RPI-04 0x80990085
```

Pokretanje: `cargo run --example headless` (ili sa imenom scenarija).

## Kako kod trenutno tretira nepotvrđene [PROVJERA] tačke

| Tačka (DESIGN 14.1) | Trenutno ponašanje |
|---|---|
| JSON `is_exists` / progresa | prima bool/string/int, broj/decimalni/hex string, flatz i fork imena polja |
| `/api/log` metoda | POST pa GET, pobjednik se kešira |
| polja `uninstall_patch`/`uninstall_ac` | `RpiDialect` (default po DESIGN-u, alternativa testirana) |
| 404 tijela | JSON sa `status` = RPI greška, inače "endpoint ne postoji" |
| signal završetka | prenos 100 % → `Installing`; igra se potvrđuje `is_exists` (svake 2 s, max 10 min), update/DLC odmah `Done` |
| "već instalirano" za update | aktivno samo ako `is_exists` vrati verziju |
| task nakon restarta | `get_task_progress` → `resume_task`; ako task ne postoji → `unregister_task` + puni tok |
| BGFT kod kad PC zastane | mock koristi izmišljeni `0x80990088` (samo za test) |

## Odstupanja i šta je pomjereno

- **Predprovjera pri dodavanju** (6.1/6: info "Nije instalirano"/"Čeka osnovnu igru") nije u Fazi 1 — ide u Fazu 3 zajedno sa listom. Obavezna provjera prije instalacije (6.2/3) radi.
- **Komande `TestConnection`, `Uninstall`, `CheckInstalled`** dolaze sa svojim ekranima (Faza 2 Postavke, Faza 4 deinstalacija).
- **`--debug` flag** se čita u `main.rs` u Fazi 2; logger ga već podržava.
- **> 4 GB kroz cijeli lanac engine → mock** nije pokrenut (bio bi stvarni prenos 5 GB); Range iznad 4 GB je pokriven na nivou servera i parsera.
- Na Windowsu odbijena TCP konekcija traje ~2 s (sistem ponavlja SYN), pa "konzola ne odgovara" stiže nakon connect timeouta.

## Za CP-1

1. Pregled ovog izvještaja i demo-a (`cargo run --example headless`).
2. Odobrenje taga `v0.0.1-alpha.1`.
3. Odobrenje starta Faze 2 (UI kostur, tema, Postavke, Log) — Faza 3 zavisi od nje.
