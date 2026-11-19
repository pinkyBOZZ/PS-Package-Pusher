# Faza 0 — izvještaj spike-ova (PLAN 0.5)

Datum: 2026-11-19 · Mašina: dev laptop, Windows 11 (build 26300), AMD Radeon iGPU 60 Hz, skaliranje 125 %.

| Spike | Pitanje | Rezultat | Prijedlog odluke |
|---|---|---|---|
| S-1 | Splash → glavni prozor (preobražaj root viewporta) | Radi: 480×300 bez okvira i transparentan → 1100×700 sa okvirom, centrirano (`Transparent(false)`, `Decorations(true)`, `Resizable`, `MinInnerSize`, `InnerSize`, `OuterPosition`). Vlasnik vizuelno potvrdio: OK, bez treperenja. | Preobražaj (ne `show_viewport_deferred`), ako vlasnik ne vidi bljesak |
| S-1/S-7 | Renderer glow vs wgpu, fps | glow: ~25–27 fps, wgpu: ~30 fps uz stalni repaint na ovoj mašini (ekran 60 Hz). Isto i za transparentan i neprozirni prozor. Uzrok nepoznat (iGPU/frame pacing); **[PROVJERA]** na kućnom računaru. | **glow** (manji exe, razlika u fps zanemariva) |
| S-2 | Windows tema | `ctx.system_theme()` vraća `Dark` odmah na startu. Promjena teme uživo: vlasnik potvrdio da prozor reaguje. `SPI_GETCLIENTAREAANIMATION` čitanje radi (`true`). | `ThemePreference::System` + rezerva iz DESIGN 9.7 ako živa promjena ne stigne |
| S-3 | Repaint u mirovanju | **0 frame-ova** nakon završetka animacije (debug i release). | Potvrđeno, bez izmjena |
| S-4 | HTTP server | Vlastiti GET/HEAD server nad `std::net` (~150 linija): Range na 5 GB sparse fajlu tačan (prvi i zadnji bajtovi, suffix `-16`), više opsega i opseg van fajla → 416, 404/405, keep-alive (2 zahtjeva = 1 konekcija), prekid klijenta ne ruši server, 40 paralelnih → 32× 206 + 7× 503 + 1× reset, nakon opterećenja radi. Protok ~176 MB/s (loopback). IP filter može tiho zatvoriti konekciju (tiny_http ne može). | **Vlastiti server**, `tiny_http` se izbacuje. Ispraviti: prije 503 pročitati zahtjev da klijent ne dobije reset |
| S-5 | Firewall prompt i pravilo (9.8) | Windowsov prompt se **ne pojavljuje** na ovom računaru: politika ima `NotifyOnListen=False` za sve profile (mreža `DomainAuthenticated`). Naše pravilo preko UAC: prvi pokušaj pao zbog 8.3 putanje u `program=` ("application name could not be resolved"); sa kanonskom putanjom **uspjelo** — Enabled, Allow, Private, TCP 18095, LocalSubnet, vezano za exe. Brisanje preko UAC uspjelo (`delete` bez poklapanja vraća exit 1). Čitanje stanja preko PowerShella 1,8–3,2 s. | Mehanizam iz 9.8 potvrđen; DESIGN 9.8 dopunjen nalazima. Prompt na kućnom računaru → Sesija B (B2) |
| S-6 | PKG parser na stvarnim fajlovima | Stvarni paket (igra, `gd`, ~1,4 GB, ne-split): magic, `entry_count` @0x10, tabela @0x18 (0x2A80, nije 0x1000), Content ID @0x40, `pkg_size` @0x430 = veličina fajla, param.sfo (0x1000) i icon0.png (0x1200, PNG 512×512) nekriptovani — sve **potvrđeno**. param.sfo: `APP_VER=01.00`, `VERSION=01.01` za isti `gd` paket. Ime fajla završava na `_2.pkg`, a paket je samostalan — pravilo DESIGN 6.1/2 (samostalan `_N` bez `_0` = običan paket) je nužno. Split paket nije testiran. | Parser iz 9.5 potvrđen. Za prikaz verzije igre ostaje `APP_VER` (14.2/18), `VERSION` u tooltip. Split → Sesija B |
| S-7 | Veličina exe-a, ikone | Release exe (glow, bez fontova) **5,7 MB**; sa glow+wgpu 11,7 MB. `egui-phosphor` 0.14 traži egui 0.36; postoji fork `egui-pop-phosphor` 0.15 za 0.37 (nepoznat održavalac). | egui **0.37** + Phosphor TTF ugrađen direktno (MIT) — traži OK za preuzimanje fonta |
| S-8 | Narrator | Nije izvedeno — traži vlasnika. | Faza 5 |

## Ostalo urađeno u Fazi 0

- 0.1 git repo, `.gitignore`, `.gitattributes`, `LICENSE` (pinkyBOZZ), `rust-toolchain.toml`, git identitet pinkyBOZZ + noreply.
- 0.2 Cargo kostur po DESIGN 7.2 (lib `pspp` + `ps-package-pusher` + `mock-rpi`), build/clippy/fmt čisti.
- 0.4 `ci.yml` (nije još pokrenut — repo nije pushan).
- 0.6 Logo: `assets/logo/logo.svg`, `icon-32.svg` (24/32/48), `icon-16.svg` (pixel-aligned), PNG rendere u `assets/logo/png/` preko `examples/render_icons.rs` (resvg, dev-dependency).

## Grane

- `faza-0`: kostur, CI, logo, `pkg_dump`, ovaj izvještaj.
- `spike/window`: S-1/S-2/S-3/S-7 (`examples/spike_window.rs`).
- `spike/server`: S-4 (`examples/spike_server.rs`).

## Treba od vlasnika za CP-0

1. Pokrenuti `cargo run --release --example spike_window` (grana `spike/window`) i reći da li preobražaj trepće; dok radi, promijeniti Windows temu i vidjeti da li prozor reaguje.
2. S-5: zajedno izvesti firewall test (UAC klik).
3. S-6: putanja do nekoliko legalnih pkg fajlova (idealno i split).
4. OK za preuzimanje Phosphor TTF fonta (MIT).
5. OK za push na `pinkyBOZZ/PS-Package-Pusher` (pokreće CI).

## CP-0 odluke vlasnika (2026-11-19)

Prihvaćeno: glow renderer, vlastiti HTTP server (bez `tiny_http`), egui/eframe 0.37 sa direktno ugrađenim Phosphor 2.1.2 TTF (`assets/fonts/`, MIT), logo i ikone iz 0.6. Upisano u DESIGN 7.1, 9.4, 14.3/23.

S-1 splash: vlasnik potvrdio OK. Spike grane obrisane (rezultati u ovom izvještaju).

S-2 živa promjena teme: vlasnik potvrdio OK.

S-5 firewall: izveden sa vlasnikom, pravila uklonjena.

S-6 pkg parser: potvrđen na stvarnom paketu (ne-split).

**Faza 0 je kompletna — CP-0 čeka odobrenje vlasnika za start Faze 1.**
