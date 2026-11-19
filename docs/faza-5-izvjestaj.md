# Faza 5 — izvještaj (splash, animacije, i18n, pristupačnost, ikona)

Datum: 2026-11-19 · Grana: `main` · Testovi: 58 prolazi, clippy `-D warnings` i rustfmt čisti.

## Urađeno po PLAN zadacima

| ID | Šta |
|---|---|
| 5.1 | **Splash** po DESIGN 4.1: scena se crta painterom (sunce sa prugama i gradijentom izlazi, horizont bljesne, mreža se pali od horizonta, kutija sa `back_out` skalom, strelica proleti uz speed linije), naziv u Audiowide, verzija, tekst koraka i progress linija. Koraci u pozadinskoj niti (postavke, red čekanja, IP računara, port). Traje min. 1,2 s, max. 5 s; klik / Esc / Enter / Space preskače. Na kraju se root prozor preobrazi u glavni (kao u spike-u S-1) i sadržaj se pojavi fade-om. Isključiv u Postavkama. |
| 5.2 | `motion.dialog.in`: dijalozi i pozadina se pojavljuju fade-om (180 ms); tab, red, reorder i tema već iz Faza 2–3. "Show animations" isključen → bez animacija, splash statičan. |
| 5.3 | `assets/icon/app.ico` (16/24/32/48/256, iz logo PNG-ova, `examples/make_ico.rs`) + `build.rs` (winresource): ikona exe-a i VERSIONINFO (ProductName, FileDescription, verzija iz Cargo.toml, `Copyright (c) 2026 pinkyBOZZ (MIT)`). |
| 5.4 | Svi korisnički tekstovi idu kroz `t()`; paritet EN/BS test zelen. Izuzetak: debug galerija tokena (samo `--debug`). |
| 5.5 | Tastatura: `Ctrl+1/2/3`, `Ctrl+Tab`, `Ctrl+O`, `Ctrl+Shift+O`, `Ctrl+Enter`, `↑/↓`, `Alt+↑/↓`, `Alt+Home/End`, `Delete`, `Shift+Delete`, `Space`, `Enter`, `F5`, `Esc`; zoom `Ctrl+=` / `Ctrl+-` je ugrađen u egui. |
| 5.6 | AccessKit: vlastita dugmad i redovi reda čekanja imaju ime za čitače ekrana ("naslov, status"). Prolaz sa Narratorom **nije** urađen (traži tebe). |
| 5.7 | Test zabranjenih simbola (`tests/forbidden_symbols.rs`): skenira string literale u `src/` i sve prevode. |
| 5.8 | UI matrica (screenshotovi u `docs/faza-5/`): red dark/EN/100 %, red light/BS/150 %, Postavke dark/BS, Log light/EN, splash. **Nalaz:** na 150 % indikator konzole se preklapao sa tabovima i toolbar je izlazio iz prozora → indikator se sužava na ikonu + IP (puni tekst u tooltipu), toolbar se prelama u drugi red. |
| 5.9 | RC paket (lokalno, `dist/`, nije u gitu): `PS-Package-Pusher-0.0.1-rc.1-win-x64.zip` = `ps-package-pusher.exe` (9,8 MB, ispod cilja 15 MB), `mock-rpi.exe`, `pkg_dump.exe`, LICENSE + licence fontova. Release exe provjeren na čistom pokretanju (splash → glavni prozor → firewall uputa). `release.yml` dolazi u Fazi 7. |

## Kriteriji prihvatanja

- Splash ≤ 5 s, preskakanje radi; reduce motion → statična scena. ✓
- Exe < 15 MB (9,8 MB). ✓
- Test zabranjenih simbola zelen. ✓
- UI matrica bez blokirajućih nalaza (nalaz na 150 % popravljen). ✓
- Narrator prolaz — **otvoreno** (ručno).

## Nije provjereno

- Narrator / čitač ekrana uživo.
- Stvarno Windows skaliranje 150 % / 200 % (matrica je rađena egui zoom-om, što je strože jer prozor ostaje iste fizičke veličine).
- Splash na drugom monitoru / različitom DPI-ju.

## Za CP-5

1. Pokreni RC iz `dist/PS-Package-Pusher-0.0.1-rc.1-win-x64/ps-package-pusher.exe` (ili `cargo run --release`) i pogledaj splash, tamnu/svijetlu temu i dijaloge.
2. Odobrenje taga `v0.0.1-rc.1`.
3. **Sesija B na konzoli** (PLAN 7.2) — sljedeći korak; sa RC buildom kod kuće. Uslov za Fazu 6 i release.
