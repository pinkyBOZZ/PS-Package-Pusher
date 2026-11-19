# Faza 0 — izvještaj spike-ova (PLAN 0.5)

Datum: 2026-11-19 · Mašina: dev laptop, Windows 11 (build 26300), AMD Radeon iGPU 60 Hz, skaliranje 125 %.

| Spike | Pitanje | Rezultat | Prijedlog odluke |
|---|---|---|---|
| S-1 | Splash → glavni prozor (preobražaj root viewporta) | Radi: 480×300 bez okvira i transparentan → 1100×700 sa okvirom, centrirano (`Transparent(false)`, `Decorations(true)`, `Resizable`, `MinInnerSize`, `InnerSize`, `OuterPosition`). Treperenje **nije ocijenjeno okom** — čeka vlasnika. | Preobražaj (ne `show_viewport_deferred`), ako vlasnik ne vidi bljesak |
| S-1/S-7 | Renderer glow vs wgpu, fps | glow: ~25–27 fps, wgpu: ~30 fps uz stalni repaint na ovoj mašini (ekran 60 Hz). Isto i za transparentan i neprozirni prozor. Uzrok nepoznat (iGPU/frame pacing); **[PROVJERA]** na kućnom računaru. | **glow** (manji exe, razlika u fps zanemariva) |
| S-2 | Windows tema | `ctx.system_theme()` vraća `Dark` odmah na startu. Promjena teme uživo **nije testirana** (traži promjenu sistemske postavke) — čeka vlasnika. `SPI_GETCLIENTAREAANIMATION` čitanje radi (`true`). | `ThemePreference::System` + rezerva iz DESIGN 9.7 ako živa promjena ne stigne |
| S-3 | Repaint u mirovanju | **0 frame-ova** nakon završetka animacije (debug i release). | Potvrđeno, bez izmjena |
| S-4 | HTTP server | Vlastiti GET/HEAD server nad `std::net` (~150 linija): Range na 5 GB sparse fajlu tačan (prvi i zadnji bajtovi, suffix `-16`), više opsega i opseg van fajla → 416, 404/405, keep-alive (2 zahtjeva = 1 konekcija), prekid klijenta ne ruši server, 40 paralelnih → 32× 206 + 7× 503 + 1× reset, nakon opterećenja radi. Protok ~176 MB/s (loopback). IP filter može tiho zatvoriti konekciju (tiny_http ne može). | **Vlastiti server**, `tiny_http` se izbacuje. Ispraviti: prije 503 pročitati zahtjev da klijent ne dobije reset |
| S-5 | Firewall prompt i pravilo (9.8) | Nije izvedeno: traži "Allow" i UAC klik vlasnika. Lokalna pravila su dozvoljena na ovoj mašini. | Izvesti zajedno sa vlasnikom |
| S-6 | PKG parser na stvarnim fajlovima | `examples/pkg_dump.rs` napravljen i provjeren na sintetičkom paketu (header, entry tabela, enkripcija, param.sfo, `pkg_size` @0x430). Stvarni paketi **nisu testirani** — trebaju vlasnikovi legalni fajlovi (homebrew, vlastite kopije, split). | Čeka fajlove |
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

Otvoreno do zatvaranja CP-0: vizuelna provjera splasha i žive promjene teme (S-1/S-2), firewall test (S-5), stvarni pkg fajlovi (S-6).
