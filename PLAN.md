# PS Package Pusher — PLAN izrade

Status: nacrt plana za v0.0.1 · Datum: 2026-11-19 · Jezik: bosanski (tehnički termini na engleskom)

Izvor istine za zahtjeve je `DESIGN.md`; ovaj plan ga ne ponavlja nego referencira sekcije ("DESIGN 6.2"). Plan opisuje put od praznog foldera do javnog GitHub releasea `0.0.1`.

Veličina zadataka: **S** (mali, jedna cjelina), **M** (nekoliko povezanih dijelova), **L** (veći modul ili više modula). Vremenske procjene se namjerno ne daju.

---

## 0. Pravila rada

1. **Ništa ne počinje bez izričitog OK-a vlasnika.** Prvi checkpoint je **CP-P: odobrenje ovog plana**. Tek nakon njega se radi Faza 0.
2. Svaka faza završava **checkpointom (CP-n)**: pokazuje se ono što je navedeno u fazi (demo, screenshotovi, test izvještaj), a vlasnik odobrava (a) rezultat faze i (b) početak sljedeće. Nema automatskog prelaska faza. Paralelni rad dvije faze je moguć samo ako vlasnik izričito odobri obje.
3. Mehanizam (odluka vlasnika 2026-11-19): radi se **direktno na `main`** i pusha na `main`; bez PR-ova i faza-grana. Checkpoint je pregled isporuka i izvještaja faze (`docs/faza-N-izvjestaj.md`) + zelen CI na `main`. Spike kod ostaje na granama `spike/*`.
4. Akcije koje uvijek traže vlasnika: `git init`, kreiranje GitHub repoa, push tagova, promjena vidljivosti repoa u public, objavljivanje releasea, klik "Allow" na Windows Firewall promptu, sve na konzoli, preuzimanje fajlova (fontovi, alati), izmjene odluka iz DESIGN-a.
5. Izmjena DESIGN-a tokom izrade: samo kad se nađe greška/kontradikcija ili kad provjera (spike, sesija na konzoli) da novi podatak; svaka izmjena se navodi u PR-u i odobrava na checkpointu.
6. Opseg je zaključan: sve iz DESIGN 1.1 je v1, DESIGN 1.2 ostaje van v1.

---

## 1. Pristup: rizik-prvo

Najrizičnije stvari su one koje (a) zavise od ponašanja koje ne kontrolišemo (eframe/winit/Windows, RPI/BGFT) i (b) bi kasno otkrivene promijenile arhitekturu. Zato:

- **Faza 0** je niz kratkih spike-ova koji odgovaraju na DESIGN 14.3 (prozor, tema, HTTP server, firewall, veličina exe-a) i na PKG dio 14.2, prije ijedne linije produkcionog koda.
- **Jezgro bez UI-ja (Faza 1)** se gradi i testira protiv **mock RPI-ja** (DESIGN 8), tako da kod nikad ne zavisi od nepotvrđenih detalja RPI-ja: tolerantni parseri, sva nepotvrđena imena polja na jednom mjestu ("RPI dialect", vidi 4.1), mock scenariji za obje varijante svakog nepotvrđenog detalja.
- **Konzola je nedostupna**, pa su sve [PROVJERA] stavke iz DESIGN 14.1 sabrane u dvije sesije koje vlasnik izvodi kod kuće (sekcija 7):
  - **Sesija A** — samo `curl.exe` sonde, ne treba nijedna linija našeg koda; može odmah nakon CP-0, **najkasnije prije početka Faze 4**.
  - **Sesija B** — end-to-end sa release-candidate buildom; **nakon CP-5, uslov za CP-6 i za release**.
- UI se gradi na tokenima iz DESIGN 2 od prvog dana (Faza 2), da se izgled ne "dorađuje" na kraju.

---

## 2. Preduslovi okruženja

Provjereno read-only na razvojnoj mašini 2026-11-19 (Windows 11 Pro, build 10.0.26300). Ništa nije instalirano.

| Stavka | Stanje na mašini | Potrebno | Akcija |
|---|---|---|---|
| Rust toolchain | `rustc 1.99.1`, `cargo 1.99.1`, `rustup 1.29.3`, toolchain `stable-x86_64-pc-windows-msvc` | ≥ 1.95 (MSRV eframe/egui 0.37) | ✓ ništa |
| Komponente | `clippy`, `rustfmt`, `rust-docs` | clippy, rustfmt | ✓ ništa |
| Target | `x86_64-pc-windows-msvc` | isti (DESIGN 7.1) | ✓ ništa |
| MSVC linker | Visual Studio Build Tools 2026 (18.10) sa VC x86/x64 | VC tools | ✓ ništa |
| Windows SDK | 10.0.26100 (i stariji) | bilo koji 10.0.x (winresource/rc.exe) | ✓ ništa |
| Git | 2.54.0 | ✓ | ništa |
| GitHub CLI | `gh` 2.103.3 | za kreiranje repoa/releasea | prijava (`gh auth status`) nije provjeravana — vlasnik provjerava na CP-0 |
| `curl.exe`, PowerShell | `C:\WINDOWS\system32\curl.exe`, Windows PowerShell 5.1 | za sesije na konzoli | ✓ (skripte u sekciji 6 su pisane za 5.1) |
| `cargo-about` | ne postoji | THIRD-PARTY-LICENSES (DESIGN 13.2) | instalacija u Fazi 7 (lokalno opciono; u CI-ju obavezno), uz OK |
| `cargo-deny` / `cargo-audit` | ne postoji | opciono (licence/advisory u CI-ju) | prijedlog za Fazu 7, uz OK |
| Fontovi | ne postoje | Inter Regular/SemiBold, Audiowide + OFL.txt (DESIGN 2.3) | preuzimanje u Fazi 2, uz OK (izvori: rsms/inter GitHub release, Google Fonts) |
| Logo | `assets/logo/logo.svg` ne postoji | vektor + ručni PNG 16/24/32 (DESIGN 2.5) | pravi Claude po konceptu 4 (Q4), zadatak 0.6 |
| Alat za `.ico` | ImageMagick/Inkscape ne postoje | pakovanje PNG → ICO | bez novog alata: `examples/make_ico.rs` sa `image` (feature `ico`) kao dev-dependency |
| Python 3.15, Node | postoje | nisu potrebni | — |
| Windows 10 okruženje | nema | — | odlučeno (Q7): službeno samo Windows 11; README: "Windows 10 22H2 nije testiran" |
| Git identitet | `user.name`/`user.email` nisu postavljeni (global) | pseudonim, bez pravog imena/emaila (Q6) | zadatak 0.1: lokalno za repo `user.name=pinkyBOZZ`, `user.email=<GitHub noreply>` |

Napomena o mašini (dev okruženje): firewall profili dozvoljavaju lokalna pravila (`AllowLocalFirewallRules=True`, provjereno read-only 2026-11-19), pa se S-5 i firewall pravilo (DESIGN 9.8) mogu testirati ovdje. Wi-Fi je na "Domain" profilu i postoji VPN interfejs `wt0`; scenarij sa konzolom (Private mreža kod kuće) se potvrđuje u Sesiji B (B2).

### 2.1 Verzije crate-ova (crates.io, 2026-11-19)

Pinuje se u `Cargo.lock` na početku Faze 1, nakon odluka iz Faze 0.

| Crate | Zadnja stabilna | Napomena za plan |
|---|---|---|
| `eframe` / `egui` | 0.37.0 | MSRV 1.95. Default renderer je sada **wgpu**; DESIGN 7.1 traži glow → `default-features = false`, feature-i `glow`, `accesskit`, `default_fonts`. API-ji koje DESIGN koristi postoje u 0.37: `ViewportCommand::{Decorations, InnerSize, Resizable, OuterPosition, Transparent, Visible, MinInnerSize, SetTheme}`, `emath::easing::{linear, cubic_*, back_out}`. |
| `egui-phosphor` | 0.14.0 | **zavisi od `egui ^0.36`** — ne radi sa egui 0.37. Odluka u S-7: (a) pin egui/eframe 0.36.x, ili (b) ugraditi Phosphor TTF (MIT) direktno sa konstantama za ~30 ikona koje se koriste. |
| `tiny_http` | 0.12.0 | zadnji release 10/2022; zavisi od `rustc-serialize`; nema API za socket timeoute; drop neodgovorenog `Request`-a šalje 500 (zato je DESIGN 9.4 dopunjen). Odluka u S-4. |
| `ureq` | 3.5.2 | `max_redirects(0)`, `proxy(None)`, `timeout_connect`, `timeout_recv_response`/`timeout_global`; **`http_status_as_error(false)`** je obavezno, inače se tijelo 403/404 gubi i klasifikacija DESIGN 9.1 ne radi. |
| `rfd` | 0.18.0 | — |
| `image` | 0.25.10 | samo `png` (i `ico` u dev-dependency za `make_ico`) |
| `windows-sys` | 0.61.2 | feature-i po DESIGN 7.1 (+ `FSCTL_SET_SPARSE` samo u testovima, vidi 5.2) |
| `winresource` | 0.1.34 | build-dependency |
| `serde`, `serde_json` | 1.x / 1.0.154 | — |
| `cargo-about` | 0.9.3 | alat, ne crate projekta |

`rust-toolchain.toml` pinuje `1.100.0` (bilo `1.99.1`, ažurirano 2026-11-20) + clippy/rustfmt, da CI i lokalni build koriste isti kompajler.

---

## 3. Rezultati desk-provjere PKG formata (DESIGN 9.5, 14.2)

psdevwiki.com je automatizovanom čitanju vratio HTTP 403, pa je provjera urađena prema tri nezavisne open-source implementacije (izvori ispod). Vlasnik može dodatno ručno pogledati https://www.psdevwiki.com/ps4/Package_Files i https://www.psdevwiki.com/ps4/Param.sfo.

| DESIGN tvrdnja | Nalaz | Izvor |
|---|---|---|
| Big-endian zaglavlje, magic `0x7F434E54` @0x00 | potvrđeno | orbis-pkg, LibOrbisPkg |
| 0x0C `file_count`, 0x10 `entry_count` (u32) | potvrđeno (0x14 `sc_entry_count` u16, 0x16 `entry_count_2` u16) | orbis-pkg `header.rs`, LibOrbisPkg `PkgReader.cs`, ps4-pkg-info `lib.rs` |
| 0x18 offset tabele entry-ja (u32) | potvrđeno; 0x1000 je kraj glavnog zaglavlja, ne nužno početak tabele — uvijek čitati 0x18 | isti |
| 0x40 Content ID, 36 B ASCII | potvrđeno (0x64–0x6F padding) | orbis-pkg, LibOrbisPkg |
| Entry 32 B: `id, filename_offset, flags1, flags2, data_offset, data_size, pad u64` | potvrđeno | LibOrbisPkg `Entry.cs` (MetaEntry), ps4-pkg-info |
| `id 0x1000` param.sfo, `id 0x1200` icon0.png | potvrđeno | ps4-pkg-info (`SFO_TABLE_ID`, `ICON0_TABLE_ID`) |
| Enkripcija = `flags1 & 0x80000000` (14.2/16) | potvrđeno (`Encrypted => (Flags1 & 0x80000000) != 0`; key index = `(flags2 & 0xF000) >> 12`, ne treba nam) | LibOrbisPkg `Entry.cs` |
| Ukupna veličina paketa (14.2/17) | `pkg_size` / `package_size`, **u64 @ 0x430** | orbis-pkg `header.rs`, LibOrbisPkg `PkgReader.cs` |
| param.sfo LE: `\0PSF`, 0x08 key table, 0x0C data table, 0x10 count, index 16 B od 0x14 (`key_offset u16, fmt u16, len u32, max_len u32, data_offset u32`) | potvrđeno; fmt `0x0204` utf8, `0x0004` utf8-special, `0x0404` u32 | ps4-pkg-info, playstationdev.wiki SFO |

Zaključak: **u DESIGN 9.5 nema greške**. 14.2/16 je potvrđeno desk-provjerom; 14.2/17 je lociran (0x430), ali ostaje provjeriti na stvarnom split paketu da `pkg_size` u `_0` sadrži ukupnu veličinu svih dijelova (S-6, lokalno, bez konzole). 14.2/18–19 ostaju za S-6 i Sesiju B. Odluka Q8: nalazi su **upisani** u DESIGN 9.5 i 14.2/16–17 (2026-11-19).

Izvori:
- https://docs.rs/orbis-pkg/0.1.0/src/orbis_pkg/header.rs.html
- https://docs.rs/ps4-pkg-info/0.1.2/src/ps4_pkg_info/lib.rs.html
- https://raw.githubusercontent.com/maxton/LibOrbisPkg/master/LibOrbisPkg/PKG/Entry.cs
- https://raw.githubusercontent.com/maxton/LibOrbisPkg/master/LibOrbisPkg/PKG/PkgReader.cs
- https://playstationdev.wiki/psvitadevwiki/index.php?title=SFO

---

## 4. Faze

Pregled:

| Faza | Naziv | Checkpoint | Interni tag |
|---|---|---|---|
| — | Odobrenje plana | CP-P | — |
| 0 | Repo, CI kostur, spike-ovi | CP-0 | — |
| 1 | Jezgro bez UI-ja (pkg, rpi, mock, server, engine) | CP-1 | `v0.0.1-alpha.1` |
| 2 | UI kostur, tema, tokeni, Postavke, Log | CP-2 | `v0.0.1-alpha.2` |
| 3 | Red čekanja end-to-end protiv mock-a | CP-3 | `v0.0.1-alpha.3` |
| 4 | Provjere, reinstall, deinstalacija, greške, log | CP-4 | `v0.0.1-alpha.4` |
| 5 | Splash, animacije, i18n, pristupačnost, brend | CP-5 | `v0.0.1-rc.1` |
| 6 | Sesija B na konzoli + popravke | CP-6 | `v0.0.1-rc.N` |
| 7 | Release: CI, README, licence, objava | CP-7 | `v0.0.1` |

Tagovi `-alpha.N`/`-rc.N` su SemVer pre-release oznake, ne troše `0.0.1`. `Cargo.toml` verzija se postavlja na isti string prije taga (DESIGN 13.2 provjera tag = Cargo verzija). `release.yml` za tag sa `-` pravi **draft pre-release** (build za testiranje), bez `-` draft release. Tagove pusha vlasnik ili agent na izričit OK.

### 4.1 Pravila otpornosti na nepotvrđene detalje RPI-ja (važe za sve faze)

- **Jedno mjesto za sve nepotvrđene detalje**: `rpi/types.rs` sadrži `RpiDialect` sa default vrijednostima prema DESIGN-u (imena polja u tijelu `uninstall_patch`/`uninstall_ac`, metoda `/api/log`, ime polja `exists`, signal završetka). Nije korisnička postavka (nije u DESIGN-u); mijenja se u kodu nakon sesije na konzoli.
- **Tolerantno čitanje**: serde `alias` za flatz i fork imena (DESIGN 9.2), brojevi kao broj/decimalni string/hex string, bool kao `true`/`"true"`/`1`, nepoznata polja ignorisana, nedostajuća = `None` (DESIGN 9.1).
- **Fallback umjesto pretpostavke**: `/api/log` POST pa GET (keš po sesiji); `uninstall_*` 404 se klasifikuje po tijelu (DESIGN 9.1 korak 3–4); ako `is_exists` ne vrati verziju, update/DLC "već instalirano" grana se jednostavno ne aktivira.
- **Mock pokriva obje varijante** svakog nepotvrđenog detalja (flagovi u Fazi 1, zadatak 1.9), a integracioni testovi se vrte nad obje.
- Sirovi odgovori iz sesija na konzoli postaju **golden fixture-i** (`tests/fixtures/rpi-3.58/`) i mock-ov default se prebacuje na stvarnu varijantu.

---

### Faza 0 — Repo, CI kostur, spike-ovi

**Cilj:** odgovoriti na najrizičnija tehnička pitanja (DESIGN 14.3, 14.2) prije produkcionog koda i postaviti repo/CI.

**Zadaci**

| ID | Zadatak | DESIGN | Vel. |
|---|---|---|---|
| 0.1 | `git init` u postojećem folderu (grana `main`), `.gitignore` (`/target`, `*.pdb`, `/dist`, `*.broken-*`, `local-golden/`), `.gitattributes` (`* text=auto`, binarni `*.ttf *.ico *.png *.pkg`), `LICENSE` (MIT, `Copyright (c) 2026 pinkyBOZZ`), `rust-toolchain.toml`; **prije prvog commita** lokalni git identitet: `git config user.name pinkyBOZZ` i `git config user.email <ID>+pinkyBOZZ@users.noreply.github.com` (adresu daje vlasnik iz GitHub Settings → Emails; vlasnik uključuje "Keep my email addresses private" i "Block command line pushes that expose my email"); prvi commit sa `DESIGN.md` + `PLAN.md` — **uz OK** | 1.3, 13.1 | S |
| 0.2 | Cargo kostur: jedan paket, `lib` + bin `ps-package-pusher` + bin `mock-rpi`, prazni moduli po 7.2; `.cargo/config.toml` sa `+crt-static` za msvc | 7.1, 7.2 | S |
| 0.3 | Privatni GitHub repo već postoji: `https://github.com/pinkyBOZZ/PS-Package-Pusher.git`. Provjeriti da je prazan i privatan, `git remote add origin …`, push grane `main` — **uz OK** (provjeriti i `gh auth status`) | 13 | S |
| 0.4 | `ci.yml` (fmt, clippy `-D warnings`, test, build release) — zelen na kosturu | 13.2 | S |
| S-1 | **Splash preobražaj** (14.3/20): minimalni eframe/glow prozor 480×300 borderless+transparentan sa zaobljenom scenom → `Decorations(true)`, `InnerSize`, `Resizable`, `OuterPosition`, `MinInnerSize`, fade sadržaja. Mjeriti: bljesak (snimak ekrana 60 fps), DWM zaobljenje, veličina na 100/150/200 %, ponašanje transparentne pozadine nakon preobražaja. Ista proba za rezervu (`show_viewport_deferred`, root skriven). Uporediti glow vs wgpu ako glow ne podržava transparentnost. | 4.1 | M |
| S-2 | **Tema uživo** (14.3/21): `ThemePreference::System`, promjena teme u Windows Settings dok app radi; da li `ctx.system_theme()` stiže i da li naslovna traka prati. Rezerva: čitanje `AppsUseLightTheme` + `ViewportCommand::SetTheme` ili `DwmSetWindowAttribute`. U istom spike-u: `SPI_GETCLIENTAREAANIMATION` (2.6). | 9.7, 2.6 | S |
| S-3 | **Repaint u mirovanju**: brojač frame-ova u spike-u; potvrditi 0 repaint-a u mirovanju i ponašanje `request_repaint` iz druge niti. | 2.6 | S |
| S-4 | **HTTP server** (14.3/23): `tiny_http` vs vlastiti mini server nad `std::net` (GET/HEAD). Test: Range nad > 4 GB sparse fajlom, 32 istovremene konekcije, klijent nestane usred odgovora (`curl.exe` prekinut), `set_write_timeout` na socketu, odbijanje ne-whitelist IP-a, keep-alive, 503 preko limita. Rezultat: izbor + DESIGN 7.1/9.4 ažuriranje. | 7.1, 9.4 | M |
| S-5 | **Firewall prompt i pravilo** (14.3/22, 9.8): nepotpisan exe sa VERSIONINFO (winresource) binduje se na LAN IP; da li dolazi prompt, pod kojim imenom, šta nakon "Cancel". Zatim elevirani `--firewall add/remove` kroz UAC i čitanje stanja preko `Get-NetFirewallRule` (trajanje, JSON oblik). Klik na "Allow" i UAC potvrdu **izvodi vlasnik**; pravila se na kraju spike-a uklanjaju. | 4.9, 6.6, 9.8 | S |
| S-6 | **PKG parser na stvarnim fajlovima** (14.2/17–19): `examples/pkg_dump.rs` (read-only: zaglavlje, entry tabela, SFO ključevi, nikad sadržaj) nad vlasnikovim legalnim pkg fajlovima (homebrew, vlastite kopije, split ako postoji). Ispis se dijeli bez imena fajlova ako vlasnik želi. | 9.5, 14.2 | S |
| S-7 | **Verzije i veličina exe-a** (14.3/25): eframe 0.37 vs 0.36 + Phosphor (crate ili ugrađen TTF); release exe sa fontovima, `lto`, `codegen-units=1`, `strip`, `panic` ostaje `unwind` (panic hook, DESIGN 11). Cilj < 15 MB. | 7.1, 2.5 | S |
| S-8 | **AccessKit/Narrator brza proba** (14.3/24): S-1 prozor sa 3 dugmeta i jednim inputom pod Narratorom. Puni test je u Fazi 5. | 12 | S |
| 0.6 | **Logo** (Q4, radi Claude): finalni `assets/logo/logo.svg` po konceptu 4, posebno crtane ikone 16/24/32 px (deblja strelica), 48/256 iz vektora; pregled na tamnoj i svijetloj pozadini i u taskbaru. `.ico` se pakuje u 5.3. Vlasnik odobrava na CP-0 | 2.5 | S |
| 0.5 | Spike izvještaj: tabela pitanje → rezultat → odluka → predložena DESIGN izmjena. Spike kod ostaje na granama `spike/*`, ne ulazi u `main`. | 14.3 | S |

**Isporuke:** repo sa kosturom i zelenim CI-jem; spike izvještaj; snimci S-1/S-2; `pkg_dump` ispis; lista odluka.

**Kriteriji prihvatanja**
- `cargo build` i CI prolaze na kosturu; repo je privatan.
- Za svaku stavku 14.3/20–25 postoji rezultat ili obrazloženo "prebačeno u Sesiju B / Fazu 5".
- Donesene (predložene) odluke: splash varijanta, renderer, egui verzija + način ugradnje ikona, HTTP server biblioteka.

**Testovi:** ručni (snimci, mjerenja); S-4 ima mali automatski test koji ostaje kao osnova za `server` testove.

**Rizici:** S-1 pokaže treperenje u obje varijante → prijedlog: splash bez transparentnosti (pravougaoni, bez zaobljenja) — odluka vlasnika. S-5 neizvodljiv na poslovnoj mašini → Sesija B.

**CP-0 (vlasnik):** pregled spike izvještaja i snimaka; odobrava odluke (splash varijanta, server, egui/Phosphor), upis PKG nalaza u DESIGN, i start Faze 1 (i opciono paralelni start Faze 2). Vlasnik od ovog trenutka može izvesti **Sesiju A** (sekcija 7.1).

---

### Faza 1 — Jezgro bez UI-ja

**Cilj:** kompletna logika (parsiranje, RPI klijent, mock, HTTP server, engine/red) testirana bez UI-ja i bez konzole.

**Zadaci**

| ID | Zadatak | DESIGN | Modul | Vel. |
|---|---|---|---|---|
| 1.1 | `storage.rs`: `%APPDATA%` putanje, atomski upis (`tmp → fsync → rename`), `.broken-<ts>` rukovanje | 7.4 | storage | S |
| 1.2 | `config.rs`: `Config` sa defaultima (6.6), `schema`, `#[serde(default)]`, validacija IP-a pri učitavanju | 7.5 | config | S |
| 1.3 | `logging.rs`: unos (nivo, izvor, Title ID, kod, polja), fajl dnevni + rotacija 14/10 MB, prsten 5000, anonimizacija `%USERPROFILE%`, skraćivanje tijela 2 KB, `console-<task>-<ts>.txt`, DEBUG samo uz `--debug` | 7.8 | logging | M |
| 1.4 | `i18n.rs` + `lang/en.json`, `lang/bs.json` (ključevi za greške i jezgro), plural bs/en, formatiranje brojeva i veličina (1024-bazno), fallback bs→en→ključ | 7.7 | i18n | M |
| 1.5 | `errors.rs`: katalog kodova (10) kao enum + `ErrorReport`; tabela 10.1 sa `verified: false`; regexi za `CE-…`/`0x8099…` bez novog crate-a (ručni skener) | 10, 10.1 | errors | M |
| 1.6 | `net.rs`: IPv4 validacija (4.5), PC IP UDP trik, /24 heuristika | 9.6, 4.5 | net | S |
| 1.7 | `pkg/header.rs`, `pkg/sfo.rs`: parser po 9.5 + sekciji 3 ovog plana (uključuje `pkg_size` @0x430 kao opcionalnu provjeru split grupe, aktivnu tek nakon S-6), limiti, bez panike, ikona dekodirana sa limitom dimenzija; PKG-03 politika po odgovoru na Q3 | 9.5 | pkg | M |
| 1.8 | `pkg/scan.rs`: ekspanzija foldera (symlink/junction zaštita, dubina 16), split grupisanje, duplikati, redoslijed nove grupe | 6.1 | pkg | M |
| 1.9 | `rpi/types.rs` + `rpi/client.rs`: endpointi, timeouti, klasifikacija 9.1, tolerantni tipovi, `RpiDialect` (4.1), ureq konfiguracija (2.1 tabela) | 9.1, 9.2 | rpi | M |
| 1.10 | `mock/` + `bin/mock_rpi.rs`: ponašanje 8.1, scenariji 8.2, plus flagovi za nepotvrđene varijante: `--exists-format bool\|string\|int`, `--log-method post\|get`, `--uninstall-fields design\|alt`, `--notfound-style json\|empty`, `--origin-check destructive\|all\|none`, `--num-format` | 8 | mock | L |
| 1.11 | `server/range.rs` (čista funkcija), `server/registry.rs`, `server/mod.rs` (bind na LAN IP, IP filter, 32 konekcije, 503, `/healthz`, SRV-03 detekcija promjene fajla, SRV-04 spor disk) — biblioteka po odluci S-4; DEBUG linija po zahtjevu: metoda, putanja, `Range`, udaljeni port, poslani bajtovi (potrebno za Sesiju B) | 9.4, 11 | server | L |
| 1.12 | `engine/queue.rs`: `Item`, `ItemState`, prelazi 5.2 kao čista funkcija (bez mreže, bez vremena) | 5 | engine | M |
| 1.13 | `engine/commands.rs` + `engine/mod.rs`: petlja `recv_timeout`, komande/događaji 7.3, tok 6.2 (uključujući korake 3–4 u osnovnom obliku), pauza/nastavak/otkaz, stall/FW-01 detekcija, nastavak 9.3, `queue.json` (debounce 500 ms); **sva vremena (1 s tick, 20 s FW-01, 60 s stall, 3×2 s, 10 min) u `EngineTimings`** da testovi rade u milisekundama | 6.2, 7.3, 9.2, 9.3 | engine | L |
| 1.14 | `winapi.rs`: `SetThreadExecutionState`, `SystemParametersInfoW`, `GetLocalTime` | 7.1 | winapi | S |
| 1.15 | `tests/support`: generator sintetičkih pkg-ova (8.3) + `examples/make_fixture.rs` (pravi lažne pkg-ove u folder za ručno testiranje UI-ja: igra/update/DLC, split 3 dijela, > 4 GB sparse, oštećeni) | 8.3 | tests | M |
| 1.16 | `examples/headless.rs`: dodaj fajlove → pokreni red protiv mock-a → ispisuj događaje (demo za CP-1, ne ide u release) | — | examples | S |

**Paralelno unutar faze:** {1.1–1.6}, {1.7–1.8}, {1.9–1.10}, {1.11} su nezavisni; 1.12–1.13 zavise od interfejsa 1.7, 1.9, 1.11 (dogovoriti tipove prvo, implementacije paralelno).

**Isporuke:** `lib` sa svim modulima osim `app/`; `mock-rpi.exe`; `pkg_dump`, `make_fixture`, `headless` primjeri; test izvještaj.

**Kriteriji prihvatanja**
- `cargo test` zelen u CI-ju; clippy bez upozorenja.
- Svi integracioni scenariji iz DESIGN 8.3 postoje i prolaze, svaki nad obje varijante `RpiDialect` gdje je relevantno.
- Parser: 10 000 nasumičnih i skraćenih ulaza bez panike (deterministički generator u testu, bez novog crate-a); svi sintetički fixture-i daju očekivane metapodatke.
- Range: unit testovi za sve oblike iz 9.4, uključujući offsete > `u32::MAX`; integracioni test sa sparse fajlom > 4 GB.
- i18n paritet test (7.7) zelen.
- `headless` demo prolazi: uspješna instalacija, split, stall → RPI-05, `--no-fetch` → FW-01, `--fail-task-at` → RPI-04 sa priloženim `/api/log`.

**Testovi:** unit (pkg, sfo, scan, range, queue prelazi, rpi types, net, i18n, config/storage atomski upis i `.broken`), integracioni (`tests/`, mock in-process na `127.0.0.1:0`).

**Rizici:** engine postane težak za testiranje → prelazi kao čista funkcija (1.12), I/O na rubovima. Sparse fajl ne radi na CI runneru → test se preskače uz jasnu poruku, range matematika ostaje pokrivena unit testovima.

**CP-1 (vlasnik):** test izvještaj (lista testova po DESIGN 8.3), snimak/transkript `headless` demo-a, lista otvorenih [PROVJERA] tačaka i kako ih kod trenutno tretira. Odobrava tag `v0.0.1-alpha.1` i start Faze 3 (Faza 2 je možda već u toku).

---

### Faza 2 — UI kostur, tema, tokeni, Postavke, Log

**Cilj:** prozor sa zaglavljem, tabovima, temom (dark/light + pretapanje), svim osnovnim widgetima, funkcionalnim Postavkama i Logom.

Može početi nakon CP-0 paralelno sa Fazom 1 samo uz izričit OK; tada koristi engine stub do CP-1.

| ID | Zadatak | DESIGN | Modul | Vel. |
|---|---|---|---|---|
| 2.1 | `main.rs` (`windows_subsystem`, `NativeOptions`, glow, persistence isključen), `app/mod.rs` ruter tabova, prečice `Ctrl+1/2/3`, `Ctrl+Tab`, prozor (default/min veličina, pozicija u `config.json`), panic hook | 3, 7.1, 11 | app | M |
| 2.2 | Fontovi (preuzimanje uz OK) + `fonts.rs` | 2.3 | app/fonts | S |
| 2.3 | `theme.rs`: `Palette` dark/light po tabelama 2.2, `lerp` u linearnom RGB, primjena na `egui::Style`, praćenje teme po rezultatu S-2 | 2.2, 9.7 | app/theme | M |
| 2.4 | `anim.rs`: `Anim` tip, easing tokeni 2.6, reduce motion, progress smoothing | 2.6 | app/anim | S |
| 2.5 | `widgets.rs`: dugmad (primarno/sekundarno/destruktivno), badge, status chip (ikona + tekst), progress bar (determinate/indeterminate), fokus prsten + glow pravila, banner | 2.2, 2.4, 2.5 | app/widgets | M |
| 2.6 | Zaglavlje: sunset traka, tabovi sa brojačima, indikator konzole, globalni banner | 3 | app | S |
| 2.7 | Postavke: polja, validacija u toku kucanja, Sačuvaj/Vrati, Test veze (6.5) protiv mock-a, jezik uživo, "Otvori folder" | 4.5, 6.5 | settings_view | M |
| 2.8 | Log tab: filteri, pretraga, proširivi unosi, Kopiraj/Kopiraj sve, Otvori folder | 4.6 | log_view | M |
| 2.9 | Debug-only "token galerija" (samo uz `--debug`): svi tokeni i widgeti u obje teme, za vizuelni pregled na checkpointima | 2 | app | S |

**Kriteriji prihvatanja**
- Unit test računa WCAG kontrast iz `Palette` konstanti i provjerava pragove iz 2.2 (tekst ≥ 4.5, UI ≥ 3, osim `text.disabled`).
- Promjena Windows teme dok app radi → pretapanje 300 ms bez restarta (ili dokumentovana rezerva iz S-2).
- U mirovanju 0 repaint-a (brojač u `--debug` overlay-u).
- Test veze protiv mock-a daje očekivane linije za: normalan rad, `--reject-all-403` (NET-03), `--legacy` (upozorenje za `/api/log`), ugašen mock (NET-01/02).
- Sve vidljive labele postoje u `en` i `bs`.

**Testovi:** unit (kontrast, validacija IP/porta, lerp); ručni (tema, DPI 100/150/200 %, tastatura po 12).

**CP-2 (vlasnik):** screenshotovi Postavki, Loga, praznog reda i token galerije u dark i light; kratak snimak promjene teme; vlasnik pregleda bosanske tekstove. Tag `v0.0.1-alpha.2`.

---

### Faza 3 — Red čekanja end-to-end protiv mock-a

**Cilj:** dodavanje, red, instalacija, pauza/nastavak/otkaz, progres i perzistencija rade od kraja do kraja protiv mock-a.

| ID | Zadatak | DESIGN | Modul | Vel. |
|---|---|---|---|---|
| 3.1 | Prazno stanje + drop zona + drag-over overlay (`hovered_files`), filtriranje ne-pkg nakon puštanja, info toast | 4.2, 4.4 | queue_view | M |
| 3.2 | "Dodaj fajlove/folder" (`rfd`), `Ctrl+O`/`Ctrl+Shift+O`, sve kroz `AddPaths` | 6.1 | queue_view | S |
| 3.3 | Lista: red 72 px, ikona (tekstura iz eventa), naziv/badge/verzija/veličina, drugi i treći red, akcije po stanju (5.1), selekcija, kontekstni meni (bez deinstalacije — Faza 4) | 4.3, 5.1 | queue_view | L |
| 3.4 | Reorder: `dnd_drag_source`/`dnd_drop_zone`, `Alt+↑/↓`, `Alt+Home/End`, kontekstni meni; aktivna stavka zaključana | 4.3, 12 | queue_view | M |
| 3.5 | Toolbar: Pokreni red (`Ctrl+Enter`), Zaustavi nakon trenutnog, Očisti završene, podnožje (broj, veličina, ukupni ETA) | 4.3 | queue_view | S |
| 3.6 | Progres, brzina, ETA u fiksnoj koloni; optimistički "Pauziram…" | 9.2, 7.3 | queue_view | S |
| 3.7 | `motion.item.in/out`, `motion.reorder` (layout sa animiranom visinom) | 2.6 | queue_view, anim | M |
| 3.8 | Predprovjera (6.1/6) i prikaz info u `Ready` | 6.1 | engine, queue_view | S |
| 3.9 | Perzistencija i restart: `interrupted` → RPI-07, "Pokušaj ponovo" sa nastavkom 9.3; PKG-07 pri učitavanju | 5.2, 9.3 | engine | S |
| 3.10 | `SetThreadExecutionState` tokom prenosa; dijalozi "Zatvaranje tokom prenosa" i "Uklanjanje aktivne stavke" | 6.2/9, 4.10 | engine, dialogs | S |

**Kriteriji prihvatanja** (ručni test-plan T3 nad fixture-ima iz `make_fixture` i mock-om):
- 3 paketa (igra, update, DLC) → red → sva tri `Done`; redoslijed nove grupe po 6.1/5.
- Split 3 dijela instaliran kao jedna stavka; nedostajući dio → `Invalid` (PKG-04).
- Duplikat odbačen sa PKG-05; isti Content ID druga verzija dozvoljen.
- Pauza/nastavak/otkaz rade; "Zaustavi nakon trenutnog" ne prekida aktivnu.
- Zatvaranje aplikacije usred prenosa → restart → RPI-07 → "Pokušaj ponovo" nastavlja (mock zadržava task).
- `--stall-after` → RPI-05; `--no-fetch` → FW-01; `--fail-task-at` → RPI-04; red nastavlja sa sljedećom stavkom (5.2 pravila).
- UI nit nikad ne blokira (nema "Not Responding" tokom parsiranja 20 fixture-a ili gašenja mock-a).
- U mirovanju 0 repaint-a; tokom prenosa repaint ~1×/s + smoothing.

**CP-3 (vlasnik):** živi demo ili snimak T3, test izvještaj. Tag `v0.0.1-alpha.3`. Na ovom checkpointu se pita za status **Sesije A** (potrebna prije starta Faze 4).

---

### Faza 4 — Provjere, reinstall, deinstalacija, greške, log

**Cilj:** sve grane provjere instaliranosti, destruktivne akcije sa potvrdom, kompletan katalog grešaka i prvo pokretanje.

**Ulaz:** rezultati Sesije A (ako postoje) ugrađeni na početku faze (zadatak 4.0). Bez Sesije A faza se radi sa DESIGN defaultima + obje mock varijante, a nepotvrđeni detalji ostaju na listi za Sesiju B.

| ID | Zadatak | DESIGN | Modul | Vel. |
|---|---|---|---|---|
| 4.0 | Ugradnja Sesije A: golden fixture-i, `RpiDialect` defaulti, mock default varijanta, DESIGN 14.1 oznake (sekcija 7.3) | 14.1 | rpi, mock, tests | S |
| 4.1 | Checking za `gp`/`ac` (baza, RPI-06), "Čeka osnovnu igru (#n)", nepoznata kategorija (PKG-06) | 6.2/3, 9.5 | engine | M |
| 4.2 | `AwaitingDecision` + dijalog 4.7 (default Preskoči, `Esc`), "primijeni na sve" (samo trenutne stavke), "Promijeni" | 4.7, 4.10, 6.3 | engine, dialogs | M |
| 4.3 | Reinstall: uninstall → ponovni `is_exists` (3×2 s) → install; RPI-08 | 6.2/4, 6.3 | engine | S |
| 4.4 | Update već instaliran: dijalog Preskoči / Instaliraj ponovo bez deinstalacije (Q2 = da, 14.4/28); aktivira se samo ako `is_exists` vraća verziju | 6.2/3 | engine, dialogs | S |
| 4.5 | Samostalna deinstalacija: kontekstni meni, `Shift+Delete`, dijalog 4.11 po tipu, `[⋯]` → 4.12 (validacija ID-a, Provjeri, CFG-03) | 6.7, 4.11, 4.12 | engine, dialogs, queue_view | M |
| 4.6 | Dijalog greške 4.8, `ErrorReport` za sve kodove, `/api/log` uz greške RPI-01/04/05/06/08, `Pokušaj ponovo` | 4.8, 6.4 | dialogs, errors | M |
| 4.7 | EN + BS tekstovi `error.<KOD>.what/why/action` za cijeli katalog 10 i tabelu 10.1 ("moguće značenje") | 10, 7.7 | lang | M |
| 4.8 | Prvo pokretanje 6.6 + firewall dijalog 4.9 + odgođeni bind; globalni banneri (port zauzet, PC IP nepoznat, konzola nije podešena) | 6.6, 4.9, 3 | app, engine | M |
| 4.9 | SRV-01 prijedlog slobodnog porta; SRV-02 promjena PC IP-a; SRV-03/04 | 9.4, 9.6 | server, engine | S |
| 4.10 | Firewall pravilo: `--firewall add|remove` (elevirani self, `netsh` bez shella), čitanje stanja u pozadini, red u Postavkama (aktivno / nema / drugi port / politika), dugme u 4.9, FW-02/03/04; test da elevirani put prima samo validan port | 9.8, 4.5, 4.9, 11 | firewall, main, app | M |

**Kriteriji prihvatanja**
- **Matrica grešaka**: za svaki kod iz DESIGN 10 postoji test (unit ili integracioni sa mock scenarijem) ili dokumentovan ručni korak; tabela kod → okidač → test ide u PR.
- Nijedna deinstalacija bez potvrde: test da engine šalje `uninstall_*` samo nakon odluke/potvrde, i da je default fokus nedestruktivan.
- `Esc` u svim dijalozima = nedestruktivna opcija.
- Prvo pokretanje na praznom `%APPDATA%` prati redoslijed 4.9/6.6 (ručni test sa privremeno preimenovanim folderom).
- Log fajl ne sadrži korisničko ime (test anonimizacije).

**Rizik:** tekst dijaloga 4.7/4.11 za save podatke ostaje nepotvrđen do Sesije B → privremeni neutralni tekst sa zvjezdicom, konačan u Fazi 6.

**CP-4 (vlasnik):** demo svih dijaloga, matrica grešaka, pregled BS tekstova grešaka. Tag `v0.0.1-alpha.4`.

---

### Faza 5 — Splash, animacije, i18n, pristupačnost, brend

**Cilj:** feature-complete build spreman za Sesiju B.

| ID | Zadatak | DESIGN | Modul | Vel. |
|---|---|---|---|---|
| 5.1 | Splash scena proceduralno po geometriji `logo.svg`, vremenska linija 4.1, koraci u pozadinskoj niti, preskakanje, timeouti, varijanta bez splasha, preobražaj po odluci S-1 | 4.1 | splash | L |
| 5.2 | Preostali motion tokeni: tab, dijalog in/out + overlay, pretapanje teme (provjera), indeterminate ≤ 30 fps | 2.6 | anim, app | M |
| 5.3 | Ikona aplikacije (`app.ico` iz PNG-ova iz 0.6, `make_ico`), `build.rs` sa winresource (ikona + VERSIONINFO) | 2.5, 13.1 | assets, build.rs | S |
| 5.4 | i18n kompletiranje: svi stringovi kroz `t()`, plural slučajevi, formati brojeva; vlasnik pregleda BS | 7.7 | lang | M |
| 5.5 | Tastatura po tabeli 12, fokus redoslijed, fokus u dijalog i nazad, fokus prsten svuda | 12 | app | M |
| 5.6 | AccessKit labele (ikonska dugmad, statusi), najava samo promjene stanja; Narrator prolaz | 12 | app | M |
| 5.7 | Test zabranjenih simbola: skenira `lang/*.json` i string literale u `src/` za `△ ○ × □ ✕ ◯ ☐` | 1.3 | tests | S |
| 5.8 | UI matrica (sekcija 6.4) + min. prozor 880×560 | 12 | — | M |
| 5.9 | `--debug` uputa za Sesiju B; RC zip kroz `release.yml` (draft pre-release); samo RC zip dodatno sadrži `pkg_dump.exe` (za B3) | 13.2 | — | S |

**Kriteriji prihvatanja**
- Splash ≤ 5 s u najgorem slučaju (koraci sa timeoutom), preskakanje radi; isključen "Show animations" → statična scena i trajanja 0 (osim progress smoothinga).
- UI matrica (5.4) prođena bez blokirajućih nalaza; screenshotovi spremljeni za README.
- Svi interaktivni elementi dostupni tastaturom; Narrator čita labele dugmadi i status aktivne stavke (ograničenja zapisana za README).
- Exe < 15 MB (ili obrazloženo odstupanje).
- Test zabranjenih simbola zelen.

**Zavisnost:** `logo.svg` + ručni PNG 16/24/32 iz zadatka 0.6.

**CP-5 (vlasnik):** screenshot matrica, snimak splasha i animacija, Narrator bilješke, RC build. Tag `v0.0.1-rc.1` (draft pre-release u privatnom repou). Vlasnik zakazuje **Sesiju B**.

---

### Faza 6 — Sesija B na konzoli + popravke

**Cilj:** potvrditi ili opovrgnuti sve preostale [PROVJERA] stavke na stvarnoj konzoli i popraviti razlike.

| ID | Zadatak | Vel. |
|---|---|---|
| 6.1 | Vlasnik izvodi Sesiju B (sekcija 7.2) i predaje paket rezultata | — |
| 6.2 | Analiza: za svaku 14.1 stavku status potvrđeno / drugačije / nije izvedeno | M |
| 6.3 | Ugradnja (sekcija 7.3): golden fixture-i, `RpiDialect`, mock defaulti, signal završetka (5.2 `Installing`), deregistracija 6.2/8, tekstovi 4.7/4.11, tabela 10.1 `verified: true` za reprodukovane kodove, DESIGN 14.1/14.2 ažuriran | M–L |
| 6.4 | Popravke bugova iz sesije; `v0.0.1-rc.2` | M |
| 6.5 | Kratka ponovna provjera na konzoli (B-ponovo, podskup koraka koje su popravke dotakle) — samo ako popravke mijenjaju ponašanje prema konzoli | S |

**Kriteriji prihvatanja**
- Svaka stavka DESIGN 14.1 i 14.2 ima status; nepotvrđene a neizvedene stavke vlasnik izričito prihvata kao poznata ograničenja (README).
- Mock po defaultu oponaša stvarno izmjerene odgovore; golden fixture testovi zeleni.
- Nema otvorenih bugova prioriteta P1 (gubitak podataka, neželjena deinstalacija, pad aplikacije, nemogućnost instalacije).

**CP-6 (vlasnik):** izvještaj sesije (tabela 14.1 → rezultat → izmjena), lista DESIGN izmjena, RC build za finalni ručni test. Odobrava ulazak u release fazu.

---

### Faza 7 — Release

Dio zadataka (CI, README kostur, CHANGELOG, issue template) može se raditi paralelno sa Fazama 5–6 uz OK.

| ID | Zadatak | DESIGN | Vel. |
|---|---|---|---|
| 7.1 | `release.yml`: tag `v0.0.*`, provjera verzije, `--release --bin ps-package-pusher` sa `+crt-static`, zip, `SHA256SUMS.txt`, draft (pre-)release | 13.2 | M |
| 7.2 | `THIRD-PARTY-LICENSES.txt` preko `cargo-about` u CI-ju (OFL fontovi, Phosphor MIT, crate licence) | 13.2 | S |
| 7.3 | `README.md` po 13.3 (screenshotovi iz Faze 5, disclaimer odmah ispod, SmartScreen "More info → Run anyway", SHA256 provjera `Get-FileHash`, troubleshooting po kodovima, poznata ograničenja iz Faze 6, zasluge) | 13.3, 1.3 | M |
| 7.4 | `CHANGELOG.md` (0.0.1), `.github/ISSUE_TEMPLATE/bug_report.md` | 13.1 | S |
| 7.5 | Release checklista (sekcija 8) | — | S |

**CP-7 (vlasnik):** go/no-go po checklisti; vlasnik prebacuje repo u public, odobrava push taga `v0.0.1`, provjerava draft release i objavljuje ga ručno.

---

## 5. Graf zavisnosti i paralelizam

```
CP-P ─► Faza 0 ─► CP-0 ─┬─► Faza 1 ─► CP-1 ─┐
                        │                   ├─► Faza 3 ─► CP-3 ─► Faza 4 ─► CP-4 ─► Faza 5 ─► CP-5 ─► Faza 6 ─► CP-6 ─► Faza 7 ─► CP-7 ─► v0.0.1
                        └─► Faza 2* ─► CP-2 ┘                       ▲                            │          ▲
                                                                    │                            ▼          │
                                                                Sesija A                     Sesija B ──────┘

  Sesija A (konzola, curl) : bilo kad nakon CP-0, najkasnije prije starta Faze 4
  Sesija B (konzola, RC)   : nakon CP-5, uslov za CP-6
  Faza 7 (CI dio)          : ci.yml od Faze 0; release.yml/README/licence paralelno sa Fazama 5–6 (uz OK)
  logo.svg + PNG ikone     : zadatak 0.6 (Faza 0), koriste ih 5.1 i 5.3
  * Faza 2 paralelno sa Fazom 1 samo uz izričit OK vlasnika
```

Šta se može raditi paralelno:
- Faza 1 iznutra: infrastruktura (1.1–1.6) ‖ pkg (1.7–1.8) ‖ rpi+mock (1.9–1.10) ‖ server (1.11); engine (1.12–1.13) nakon dogovorenih tipova.
- Faza 1 ‖ Faza 2 (uz OK).
- Sesija A ‖ Faze 1–3 (ne zavisi od koda).
- Faza 4: dijalozi (4.2, 4.5, 4.6) ‖ tekstovi grešaka (4.7) ‖ prvo pokretanje (4.8).
- Faza 5: splash (5.1) ‖ pristupačnost (5.5–5.6) ‖ i18n (5.4); README kostur ‖ Faza 5.

Šta blokira šta:
- Odluke CP-0 (server, egui verzija, splash varijanta) blokiraju 1.11, 2.1, 5.1.
- Q1–Q8 su odlučeni (sekcija 11); nijedan više ne blokira.
- Sesija A: mekano blokira 4.0/4.3/4.5 (radi se sa defaultima ako je nema).
- Sesija B: tvrdo blokira CP-6 i release.

---

## 6. Strategija testiranja

### 6.1 Nivoi

| Nivo | Šta | Gdje | Kada |
|---|---|---|---|
| Unit | pkg/sfo/scan, range, queue prelazi, rpi types (tolerancija + golden fixture-i), net, i18n paritet, config/storage, kontrast paleta, zabranjeni simboli, anonimizacija loga | `src/**` `#[cfg(test)]` | svaki commit, CI |
| Integracioni | engine + server + mock in-process, scenariji DESIGN 8.3 × varijante `RpiDialect` | `tests/` | svaki commit, CI |
| Lokalni golden (opciono) | `pkg_dump` nad vlasnikovim fajlovima, poređenje sa `local-golden/` (gitignored); `#[ignore]` test uz `PSPP_LOCAL_PKG_DIR` | lokalno | na zahtjev |
| Ručni UI | test-planovi T2–T5 iz faza + matrica 6.4 | razvojna mašina | na checkpointima |
| Konzola | Sesije A i B (sekcija 7) | kod vlasnika | 2× |

### 6.2 Testni pkg fajlovi — legalno

1. **Sintetički (primarno, u repou):** generator u `tests/support` pravi minimalan validan pkg (zaglavlje + entry tabela + param.sfo + mali PNG) i oštećene varijante (DESIGN 8.3). Nije instalabilan na konzoli i ne mora biti — služi parseru, serveru i mock-u. Split i > 4 GB varijante kao sparse fajlovi (`FSCTL_SET_SPARSE` preko `windows-sys`, samo u testovima); ako sparse nije dostupan, test se preskače sa porukom.
2. **Homebrew pkg-ovi** iz javnih open-source releasea (npr. sam RPI pkg ili druge homebrew aplikacije koje vlasnik već ima): samo lokalno za S-6 i Sesiju B; u repo ide najviše izvedeni JSON metapodataka, nikad pkg.
3. **Vlasnikove legalne kopije** (igra + update + DLC, split, > 4 GB): samo lokalno i na konzoli; ništa iz njih ne ide u repo ni u CI (DESIGN 8.3).

### 6.3 Mock kao ugovor

Svaki nepotvrđeni detalj ima mock flag sa obje varijante (1.10). Nakon sesija se default mock varijanta prebacuje na izmjerenu, a sirovi odgovori postaju fixture-i koje i parser i mock moraju reprodukovati (test "mock odgovara stvarnom": isti oblik JSON-a kao fixture).

### 6.4 UI matrica

Dimenzije: tema dark/light × skaliranje 100/150/200 % × jezik EN/BS (12 kombinacija).

- **Puni prolaz** (svi ekrani 4.1–4.12, sva stanja 5.1 kroz `make_fixture` + mock scenarije): dark-100-EN, light-150-BS, dark-200-BS, light-100-EN.
- **Smoke** (glavni prozor, jedan dijalog, Postavke) za ostalih 8.
- Dodatno: minimalni prozor 880×560, `Ctrl+=`/`Ctrl+-` zoom, isključen "Show animations", promjena teme uživo usred prenosa, Narrator na glavnim tokovima. Windows 10 se ne testira (Q7).
- Rezultat: tabela kombinacija × ekran sa ✓/nalaz + screenshotovi u `docs/screenshots/` (one za README).

---

## 7. Sesije provjere na konzoli

Obje sesije izvodi vlasnik kod kuće. Sve komande su za Windows PowerShell 5.1 na računaru u istoj mreži kao konzola. Komande ne mijenjaju ništa na računaru osim kreiranja foldera sa rezultatima.

### 7.0 Zajednička priprema (kopirati u PowerShell prozor na početku svake sesije)

```powershell
# ===== IZMIJENI OVE VRIJEDNOSTI =====
$ps4  = '192.168.1.50'                          # IP konzole (prikazan u RPI-ju)
$pc   = '192.168.1.10'                          # IP ovog računara (ipconfig, adapter prema ruteru)
$game = 'CUSA00000'                             # Title ID INSTALIRANE igre (idealno sa updateom i DLC-om)
$dlc  = 'UP0000-CUSA00000_00-0000000000000000'  # Content ID INSTALIRANOG DLC-a te igre (ako postoji)
$hb   = 'XXXX00000'                             # Title ID instalirane homebrew aplikacije
$sesija = 'A'                                   # 'A' ili 'B'
# =====================================
$base = "http://${ps4}:12801"
$out  = Join-Path $HOME ("Desktop\pspp-sesija-$sesija-" + (Get-Date -Format 'yyyyMMdd-HHmm'))
New-Item -ItemType Directory -Force -Path $out | Out-Null
Start-Transcript -Path (Join-Path $out 'transcript.txt') | Out-Null

function Invoke-Rpi {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Path,
        [string]$Body,
        [string]$Method = 'POST',
        [string[]]$Header = @()
    )
    $p = Join-Path $out $Name
    $a = @('-sS', '--max-time', '15', '-X', $Method,
           '-D', "$p.headers.txt", '-o', "$p.body.txt",
           '-w', '%{http_code} %{time_total}s', '--stderr', '-')
    foreach ($h in $Header) { $a += @('-H', $h) }
    if ($Body) {
        [IO.File]::WriteAllText("$p.request.json", $Body)   # UTF-8 bez BOM-a, tačni bajtovi
        $a += @('-H', 'Content-Type: application/json', '--data-binary', "@$p.request.json")
    }
    $a += "$base$Path"
    $r = & curl.exe @a
    "$Name | $Method $Path | $($Header -join '; ') | $r" | Tee-Object -FilePath (Join-Path $out 'summary.txt') -Append
    if (Test-Path "$p.body.txt") { Get-Content -Raw "$p.body.txt" }
}
function Body([string]$field, [string]$value) { '{{"{0}":"{1}"}}' -f $field, $value }
```

Tijela se pišu u fajl i šalju sa `--data-binary @fajl` jer Windows PowerShell 5.1 kvari navodnike u JSON-u kad se proslijede direktno `curl.exe`-u. Svaki poziv ostavlja `<ime>.headers.txt`, `<ime>.body.txt` (sirovi bajtovi) i liniju u `summary.txt`.

Zabilježiti u `biljeske.txt` u istom folderu: firmware konzole, verziju GoldHEN-a, verziju RPI-ja sa ekrana, korištene Title/Content ID-ove, i sve što se vidi na TV-u (notifikacije, lista preuzimanja).

### 7.1 Sesija A — API sonde (bez našeg koda)

**Kada:** bilo kad nakon CP-0; **najkasnije prije početka Faze 4**. Ako se ne stigne, koraci A se izvode kao prvi dio Sesije B.
**Sigurnost:** destruktivni endpointi se zovu **samo sa nepostojećim ID-ovima** (`CUSA00000`), pa nema šta da se obriše. `/api/exit` se ne zove. Korak A9 ostavlja zadatak u listi preuzimanja na konzoli; uklanja se `unregister_task`-om ili ručno (Notifications → Downloads).
**Trajanje:** kratko; ~40 poziva.

```powershell
# A1 — dostupnost (14.1 opšte)
Test-NetConnection $ps4 -Port 12801 | Out-File (Join-Path $out 'A1-tcp.txt')

# A2 — is_exists (14.1/1, /2)
Invoke-Rpi 'A2-exists-nepostojeci' '/api/is_exists' (Body 'title_id' 'CUSA00000')
Invoke-Rpi 'A2-exists-igra'        '/api/is_exists' (Body 'title_id' $game)
Invoke-Rpi 'A2-exists-homebrew'    '/api/is_exists' (Body 'title_id' $hb)
Invoke-Rpi 'A2-exists-dlc-titleid' '/api/is_exists' (Body 'title_id' $dlc)
Invoke-Rpi 'A2-exists-dlc-content' '/api/is_exists' (Body 'content_id' $dlc)

# A3 — /api/log metoda i format (14.1/6)
Invoke-Rpi 'A3-log-post' '/api/log' '{}'
Invoke-Rpi 'A3-log-get'  '/api/log' -Method GET

# A4 — nepoznat endpoint (14.1/7)
Invoke-Rpi 'A4-nepoznat' '/api/ne_postoji' '{}'

# A5 — GET na uninstall endpointima (14.1/7)
Invoke-Rpi 'A5-uninstall-game-GET'  '/api/uninstall_game'  -Method GET
Invoke-Rpi 'A5-uninstall-patch-GET' '/api/uninstall_patch' -Method GET
Invoke-Rpi 'A5-uninstall-ac-GET'    '/api/uninstall_ac'    -Method GET

# A6 — uninstall nepostojećeg + imena polja (14.1/7, /13a)
Invoke-Rpi 'A6-ugame-titleid'      '/api/uninstall_game'  (Body 'title_id'   'CUSA00000')
Invoke-Rpi 'A6-upatch-titleid'     '/api/uninstall_patch' (Body 'title_id'   'CUSA00000')
Invoke-Rpi 'A6-upatch-contentid'   '/api/uninstall_patch' (Body 'content_id' 'UP0000-CUSA00000_00-0000000000000000')
Invoke-Rpi 'A6-uac-contentid'      '/api/uninstall_ac'    (Body 'content_id' 'UP0000-CUSA00000_00-0000000000000000')
Invoke-Rpi 'A6-uac-titleid'        '/api/uninstall_ac'    (Body 'title_id'   'CUSA00000')
Invoke-Rpi 'A6-uac-prazno'         '/api/uninstall_ac'    '{}'

# A7 — Host / Origin / Sec-Fetch-Site (14.1/8)
Invoke-Rpi 'A7-exists-host-ime'    '/api/is_exists'      (Body 'title_id' 'CUSA00000') -Header 'Host: ps4.local'
Invoke-Rpi 'A7-exists-origin'      '/api/is_exists'      (Body 'title_id' 'CUSA00000') -Header 'Origin: http://example.invalid'
Invoke-Rpi 'A7-ugame-origin'       '/api/uninstall_game' (Body 'title_id' 'CUSA00000') -Header 'Origin: http://example.invalid'
Invoke-Rpi 'A7-ugame-secfetch'     '/api/uninstall_game' (Body 'title_id' 'CUSA00000') -Header 'Sec-Fetch-Site: cross-site'
Invoke-Rpi 'A7-stop-origin'        '/api/stop_task'       '{"task_id":999999}' -Header 'Origin: http://example.invalid'
Invoke-Rpi 'A7-unregister-origin'  '/api/unregister_task' '{"task_id":999999}' -Header 'Origin: http://example.invalid'
Invoke-Rpi 'A7-progress-origin'    '/api/get_task_progress' '{"task_id":999999}' -Header 'Origin: http://example.invalid'
Invoke-Rpi 'A7-install-origin'     '/api/install' ('{{"type":"direct","packages":["http://{0}:8095/pkg/0/a.pkg"]}}' -f $pc) -Header 'Origin: http://example.invalid'

# A8 — nepostojeći task (oblik greške)
Invoke-Rpi 'A8-progress-nepostojeci'   '/api/get_task_progress' '{"task_id":999999}'
Invoke-Rpi 'A8-unregister-nepostojeci' '/api/unregister_task'   '{"task_id":999999}'

# A9 — install na URL gdje ništa ne sluša: oblik task_id, progresa i greške (14.1/3, /5, /11)
Invoke-Rpi 'A9-install' '/api/install' ('{{"type":"direct","packages":["http://{0}:8095/pkg/0000000000000000/UP0000-CUSA00000_00-0000000000000000.pkg"]}}' -f $pc)
$task = Read-Host 'Upiši task_id TAČNO kako piše u JSON-u iznad (sa navodnicima ako ih ima)'
for ($i = 1; $i -le 45; $i++) {
    Invoke-Rpi ('A9-progress-{0:D2}' -f $i) '/api/get_task_progress' ('{{"task_id":{0}}}' -f $task) | Out-Null
    Start-Sleep -Seconds 1
}
Invoke-Rpi 'A9-log-nakon' '/api/log' '{}'
Invoke-Rpi 'A9-stop'       '/api/stop_task'       ('{{"task_id":{0}}}' -f $task)
Invoke-Rpi 'A9-unregister' '/api/unregister_task' ('{{"task_id":{0}}}' -f $task)
Invoke-Rpi 'A9-progress-nakon-unregister' '/api/get_task_progress' ('{{"task_id":{0}}}' -f $task)
# U biljeske.txt: da li je zadatak i dalje vidljiv u Downloads listi na konzoli nakon stop/unregister.

# A10 — validacija install URL-a (14.1/15); za svaki koji vrati task_id ponoviti A9-stop/unregister sa tim ID-om
Invoke-Rpi 'A10-hostname'  '/api/install' '{"type":"direct","packages":["http://pspp.invalid:8095/pkg/0/a.pkg"]}'
Invoke-Rpi 'A10-query'     '/api/install' ('{{"type":"direct","packages":["http://{0}:8095/pkg/0/a.pkg?x=1"]}}' -f $pc)
Invoke-Rpi 'A10-ne-ascii'  '/api/install' ('{{"type":"direct","packages":["http://{0}:8095/pkg/0/\u010d.pkg"]}}' -f $pc)
Invoke-Rpi 'A10-dugi-url'  '/api/install' ('{{"type":"direct","packages":["http://{0}:8095/pkg/0/{1}.pkg"]}}' -f $pc, ('a' * 1500))

Stop-Transcript
Compress-Archive -Path "$out\*" -DestinationPath "$out.zip"
"Gotovo: $out.zip"
```

Pokriva: 14.1/1, /2 (djelimično — bez verzije updatea ako `is_exists` nema to polje), /3 (oblik, ne puni tok), /5 (kod za nedostupan URL), /6, /7, /8, /11 (djelimično), /13a (imena polja), /15.

### 7.2 Sesija B — end-to-end sa RC buildom

**Kada:** nakon CP-5 (RC build); **uslov za CP-6 i release**.
**Potrebno:** RC zip, sadržaj sa liste ispod, isti računar kao za svakodnevnu upotrebu (zbog firewalla), kabl ili stabilan Wi-Fi.

Sadržaj (samo legalno, vlasnikov):
- **H1** mala homebrew aplikacija (`gd`) koja nije instalirana.
- **G** vlastita kopija igre sa **save podatkom koji se smije izgubiti**, njen **update** (`gp`) i jedan **DLC** (`ac`).
- **S** split paket (`_0…_n`), ako postoji. **X** bilo koji paket > 4 GB.
- Opciono: tema ili drugi tip (`CATEGORY` ≠ gd/gp/ac).

Pokretanje: prvo blok iz 7.0 sa `$sesija = 'B'`, pa:

```powershell
$exe = Join-Path $HOME 'Downloads\PS-Package-Pusher-0.0.1-rc.1-win-x64\ps-package-pusher.exe'
Get-FileHash $exe -Algorithm SHA256 | Out-File (Join-Path $out 'B0-hash.txt')
& $exe --debug
```

| Korak | Radnja | Šta zabilježiti | 14.x |
|---|---|---|---|
| B1 | Ako Sesija A nije urađena: izvesti A1–A10 | kao A | 1–8, 11, 13a, 15 |
| B2 | Prvo pokretanje na ovom računaru: firewall dijalog → Windows prompt → "Private" + "Allow" | screenshot prompta (ime aplikacije), šta je označeno | 14.3/22 |
| B3 | `pkg_dump.exe` iz RC zipa nad H1, G (+update, DLC), S, X: `& (Join-Path (Split-Path $exe) 'pkg_dump.exe') '<putanja>' > "$out\B3-<ime>.txt"` | ispis (bez sadržaja paketa) | 14.2/17–19 |
| B4 | Instalirati H1 iz reda | vrijeme do prvog zahtjeva konzole, da li dolazi HEAD, oblik Range-a, veličina komada, broj paralelnih konekcija (sve iz DEBUG loga servera); da li postoji faza "Installing"; da li konzola ponovi zadnji Range nakon završetka | 3, 4, 9 |
| B5 | `Invoke-Rpi 'B5-exists-h1' '/api/is_exists' (Body 'title_id' '<H1>')` | odgovor nakon instalacije | 1 |
| B6 | Dodati H1 ponovo → "Već instalirano" → Preskoči; pa "Pokušaj ponovo" → Reinstaliraj | da li reinstall prolazi; stanje taska nakon uspjeha (treba li `unregister_task`) | 11 |
| B7 | Igra G: provjeriti save u Settings → Application Saved Data; samostalna deinstalacija G (dijalog 4.11); ponovo provjeriti save, update i DLC | da li save ostaje; da li su update/DLC obrisani | 13 |
| B8 | Instalirati G, pa update, pa DLC; poslije svakog `is_exists` za `$game` i `$dlc` (`Invoke-Rpi`) | da li `is_exists` vraća verziju / vidi DLC | 2 |
| B9 | Ponovo instalirati isti update | ponašanje RPI/konzole | 2, Q2 |
| B10 | Tokom prenosa X: Pauziraj → 30 s → Nastavi; zatim Otkaži usred prenosa | Downloads lista na konzoli nakon otkaza | 11 |
| B11 | Tokom prenosa X: ugasiti PSPP preko Task Managera → 60 s → pokrenuti → "Pokušaj ponovo" | da li task postoji; početni Range offset nakon nastavka (DEBUG log) | 12 |
| B12 | Tokom prenosa X: deinstalirati H1 (samostalno) | da li oba uspiju | 13a |
| B13 | Tokom prenosa X: konzola u rest mode; nakon 2 min probuditi | šta javlja progres, kod greške | 14 |
| B14 | Split S | da li instalacija uspijeva sa našom konvencijom URL-ova | 10 |
| B15 | Namjerne greške: (a) kopija H1 skraćena na pola: `Copy-Item '<putanja do H1>' "$out\H1-skracen.pkg"; $s=[IO.File]::Open("$out\H1-skracen.pkg",'Open'); $s.SetLength([long]($s.Length/2)); $s.Close()`, pa instalirati kopiju; (b) izvući mrežni kabl PC-a 40 s usred prenosa; (c) pkg sa USB diska koji se izvadi usred prenosa | kod greške u progresu i u `/api/log` za svaki | 5 |
| B16 | Opciono: drugi tip paketa | kako se ponaša | 19 |

Kraj sesije:

```powershell
Compress-Archive -Path "$env:APPDATA\PS Package Pusher\logs\*" -DestinationPath (Join-Path $out 'pspp-logs.zip')
Copy-Item "$env:APPDATA\PS Package Pusher\queue.json" $out
Stop-Transcript
Compress-Archive -Path "$out\*" -DestinationPath "$out.zip"
```

Uz zip: `biljeske.txt` sa tabelom `korak | očekivano | dobijeno | napomena` i screenshotovi sa TV-a gdje je bitno (B2, B7, B10, B13).

Ako sesija postane preduga, B13–B16 se mogu odvojiti u kraći drugi dolazak; B1–B12 su minimum za release.

### 7.3 Kako se rezultati vraćaju u projekt

1. Zip se otpakuje u `tests/fixtures/rpi-3.58/sesija-<A|B>-<datum>/` (samo `*.body.txt`, `*.headers.txt`, `summary.txt`, `biljeske.txt`; logovi PSPP-a ostaju van repoa ako sadrže lokalne putanje).
2. **Parser**: svaki sačuvani odgovor postaje golden test u `rpi/types.rs` (mora se parsirati u očekivanu strukturu).
3. **Mock**: default varijante flagova (1.10) se prebacuju na izmjerene; nove varijante se dodaju ako stvarnost odstupa od obje pretpostavljene.
4. **`RpiDialect`**: imena polja, metoda `/api/log`, signal završetka, potreba za `unregister_task` nakon uspjeha.
5. **Katalog grešaka**: izmjereni kodovi u tabelu 10.1 sa `verified: true` + EN/BS tekstovi; README troubleshooting.
6. **DESIGN**: za svaku 14.1/14.2 stavku upisuje se rezultat i datum, a [PROVJERA] oznaka u odgovarajućoj sekciji se uklanja ili zamjenjuje stvarnim ponašanjem (npr. 5.2 `Installing`, 6.2/7–8, 4.7/4.11 tekst). Izmjene idu kroz PR i odobrava ih vlasnik na CP-4 (Sesija A) odnosno CP-6 (Sesija B).

### 7.4 Šta je blokirano bez sesija

| Bez | Blokirano | Kako kod radi do tada |
|---|---|---|
| Sesija A | konačni defaulti `RpiDialect`, klasifikacija 404 tijela, metoda `/api/log` | obje varijante podržane; mock testira obje |
| Sesija B | tekstovi 4.7/4.11 o save podacima; signal završetka i stanje `Installing`; deregistracija 6.2/8; nastavak 9.3; verifikovani BGFT/CE kodovi; firewall prompt (ako S-5 nije uspio); **release 0.0.1** | neutralan tekst sa zvjezdicom; `Done` po prvom pouzdanom signalu iz mock-a; kodovi sa napomenom "moguće značenje" |

---

## 8. Release checklista za 0.0.1

- [ ] CP-6 odobren; svi 14.1/14.2 statusi upisani u DESIGN.
- [ ] `Cargo.toml` verzija `0.0.1`; `CHANGELOG.md` sekcija 0.0.1.
- [ ] CI zelen na `main` (fmt, clippy `-D warnings`, test, build release).
- [ ] Ručna UI matrica (6.4) prođena na finalnom buildu; screenshotovi u `docs/screenshots/` aktuelni.
- [ ] `README.md` po DESIGN 13.3: screenshotovi dark/light, disclaimer odmah ispod (1.3), zahtjevi, brzi start + firewall, troubleshooting po kodovima, build iz izvora + mock, poznata ograničenja, zasluge (flatz, rawblue33), SmartScreen uputa, SHA256 provjera.
- [ ] `LICENSE` (MIT, `Copyright (c) 2026 pinkyBOZZ`); `git log --format='%an %ae'` sadrži samo pseudonim i noreply adresu; screenshotovi bez korisničkih putanja.
- [ ] `THIRD-PARTY-LICENSES.txt` (cargo-about) uključuje OFL za Inter i Audiowide i MIT za Phosphor.
- [ ] Nema Sony simbola ni zabranjenih znakova (test 5.7 zelen; ručni pregled ikone i README-a).
- [ ] Nema komercijalnih pkg-ova ni lokalnih putanja/korisničkih imena u repou i fixture-ima.
- [ ] `.github/ISSUE_TEMPLATE/bug_report.md` postoji.
- [ ] Repo prebačen u **public** (vlasnik).
- [ ] Tag `v0.0.1` pushan (vlasnik ili agent na izričit OK) → `release.yml` zelen.
- [ ] Draft release sadrži `PS-Package-Pusher-0.0.1-win-x64.zip` (exe + LICENSE + README + THIRD-PARTY-LICENSES) i `SHA256SUMS.txt`; hash se poklapa sa lokalno izračunatim.
- [ ] Vlasnik preuzme zip na čist Windows (ili Windows Sandbox), pokrene, prođe SmartScreen, dođe do glavnog prozora, Test veze sa konzolom.
- [ ] Vlasnik ručno objavljuje release.

---

## 9. Definicija "gotovo"

### 9.1 Zadatak

- Implementirano ponašanje iz referenciranih DESIGN sekcija; svako odstupanje je zapisano u PR-u i odobreno.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` čisti; CI zelen.
- Nova netrivijalna logika (parser, prelaz stanja, klasifikacija, Range, validacija) ima unit ili integracioni test.
- Nema `unwrap`/`expect` na podacima iz fajla, mreže ili korisničkog unosa; greške idu kroz katalog (10) i log.
- UI nit ne radi I/O ni mrežu (DESIGN 7.3).
- Svi novi korisnički stringovi u `en` i `bs` (paritet test zelen); bez zabranjenih simbola.
- Animacije ne traže repaint u mirovanju.
- Destruktivne akcije samo nakon potvrde, default nedestruktivan.

### 9.2 Release

- Sve stavke release checkliste (8) označene.
- Nema otvorenih P1 bugova; P2 su u README "poznata ograničenja" ili u issue-ima.
- Sve DESIGN 1.1 stavke implementirane i pokazane na nekom checkpointu.
- Sesija B izvedena i ugrađena.
- Vlasnik dao go na CP-7.

---

## 10. Registar rizika

| ID | Rizik | Vjer. | Uticaj | Mitigacija | Gdje |
|---|---|---|---|---|---|
| R1 | Preobražaj splash → glavni prozor treperi ili se loše skalira | sred. | sred. | S-1 rano; rezerva `show_viewport_deferred`; krajnja rezerva splash bez transparentnosti (odluka vlasnika) | F0, F5 |
| R2 | eframe ne javlja promjenu teme ili naslovna traka ne prati | niska | niska | S-2; rezerva registar + `SetTheme`/DWM | F0, F2 |
| R3 | `tiny_http` neodržavan, bez timeouta, drop šalje 500 | visoka | sred. | S-4; vlastiti GET/HEAD server nad `std::net` je predviđena rezerva (DESIGN 7.1) | F0, F1 |
| R4 | `egui-phosphor` kasni za egui verzijom | potvrđeno | niska | pin egui 0.36 ili ugrađen Phosphor TTF (S-7) | F0 |
| R5 | RPI 3.58 API se razlikuje od pretpostavki | sred. | visok | tolerantni parseri, `RpiDialect`, mock varijante, rana Sesija A | F1, F4 |
| R6 | Konzola dugo nedostupna → release blokiran | sred. | visok | Sesija A je kratka i bez koda; B podijeljena na minimum (B1–B12) i dodatak | F6 |
| R7 | BGFT koristi oblike Range-a/HEAD/paralelizam koje server ne očekuje | niska | visok | server podržava sve oblike iz 9.4 + HEAD; DEBUG log svakog zahtjeva; B4 | F1, F6 |
| R8 | Firewall: korisnik klikne "Cancel" na Windowsov prompt ili politika blokira lokalna pravila | sred. | sred. | dugme "Dozvoli u firewallu" (DESIGN 9.8), detekcija politike (FW-04), S-5 na dev mašini, B2 kod kuće | F0, F4, F6 |
| R20 | Elevirani firewall korak kao sigurnosna površina | niska | visok | fiksni argumenti, samo validiran port, bez shella, samo na klik + UAC (DESIGN 11) | F4 |
| R21 | Pravo ime/email procuri kroz git historiju ili screenshotove | sred. | sred. | lokalni git identitet prije prvog commita (0.1), provjera u release checklisti | F0, F7 |
| R9 | > 4 GB test zahtijeva sparse fajl na CI-ju | sred. | niska | `FSCTL_SET_SPARSE`; preskakanje uz poruku; range unit testovi | F1 |
| R10 | SmartScreen/antivirus lažno označi nepotpisan exe koji otvara server | sred. | sred. | README uputa + SHA256; bez code signinga u 0.0.x (DESIGN 13.2) | F7 |
| R11 | Pravni/brend: Sony simboli, komercijalni sadržaj u repou | niska | visok | test 5.7, ručni pregled, samo sintetički fixture-i, disclaimer | F5, F7 |
| R12 | Neželjena deinstalacija | niska | visok | uvijek dijalog, default/`Esc` nedestruktivan, test u Fazi 4 | F4 |
| R13 | Gubitak reda/postavki (oštećen JSON) | niska | sred. | atomski upis, `.broken` kopija, testovi | F1 |
| R14 | Exe > 15 MB | niska | niska | glow umjesto wgpu, LTO, strip (S-7) | F0, F5 |
| R15 | AccessKit/Narrator ograničen | sred. | niska | S-8 rano, puni test F5, ograničenja u README | F5 |
| R16 | Nema Windows 10 za testiranje | potvrđeno | niska | službeno samo Windows 11; README "Windows 10 nije testiran" (Q7) | F7 |
| R17 | Logo/ikone kasne | niska | sred. | rano u Fazi 0 (0.6) | F0 |
| R18 | RPI fork nema javnog izvornog koda (Codeberg repo sadrži samo README za 3.55) | potvrđeno | sred. | ponašanje se utvrđuje isključivo mjerenjem (Sesije A/B) | F6 |
| R19 | Rast opsega | sred. | sred. | DESIGN 1.2 zaključan; nove ideje idu u issue "v2" | sve |

---

## 11. Otvorena pitanja za vlasnika

Sva pitanja su odlučena 2026-11-19:

| # | Pitanje | Odluka |
|---|---|---|
| Q1 | Default port lokalnog servera | **8095**, plus opciono firewall pravilo kroz UAC (DESIGN 9.8, zadatak 4.10) |
| Q2 | "Instaliraj ponovo" za update | **da, u v1** (zadatak 4.4) |
| Q3 | PKG-03 politika | **dozvoliti** ako Content ID daje validan Title ID, inače `Invalid` |
| Q4 | Logo i ikone | pravi **Claude** po konceptu 4 (zadatak 0.6) |
| Q5 | GitHub repo | `https://github.com/pinkyBOZZ/PS-Package-Pusher.git`, **privatan** do daljnjeg (zadatak 0.3) |
| Q6 | Nosilac prava za MIT | pseudonim **pinkyBOZZ**; git commitovi sa GitHub noreply adresom (zadatak 0.1) |
| Q7 | Windows 10 | nema okruženja → službeno **samo Windows 11** |
| Q8 | Upis PKG nalaza u DESIGN | **da**, upisano (DESIGN 9.5, 14.2) |

Preostaje samo **CP-P**: odobrenje ovog plana i početak Faze 0.
