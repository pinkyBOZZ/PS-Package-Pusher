# PS Package Pusher — DESIGN

Status: nacrt dizajna za v0.0.1 · Datum: 2026-11-19 · Jezik dokumenta: bosanski (tehnički termini na engleskom)

Oznaka **[PROVJERA]** znači pretpostavku koju treba potvrditi na stvarnoj konzoli (RPI 3.58) ili prototipom, prije nego što se kod oslone na nju. Sve [PROVJERA] stavke su sabrane u sekciji 14.

---

## 1. Pregled

PS Package Pusher (u daljem tekstu PSPP) je Windows desktop aplikacija (Rust, egui/eframe) koja instalira PS4 `.pkg` fajlove na jailbreakovanu konzolu preko homebrew aplikacije **Remote PKG Installer (RPI) v3.58** (fork flatz-ovog RPI-ja, https://codeberg.org/rawblue33/Remote-PKG-Installer-v3.53).

Princip rada:

1. PSPP čita metapodatke iz `.pkg` lokalno (naziv, Title ID, verzija, tip, ikona).
2. PSPP podiže lokalni HTTP server koji servira samo pakete iz reda.
3. PSPP šalje RPI-ju (`http://<IP_konzole>:12801/api/...`) zahtjev za instalaciju sa URL-ovima ka svom serveru.
4. Konzola (BGFT servis) sama povlači fajl HTTP Range zahtjevima; PSPP prati napredak preko `/api/get_task_progress`.

Distribucija: javni GitHub repo, MIT licenca, verzije `0.0.x` (prvi release `0.0.1`), jedan `.exe` za Windows 11 x64 (službeno podržano i testirano; Windows 10 22H2 "trebalo bi raditi, nije testirano").

### 1.1 Ciljevi (v1)

- Dodavanje paketa: drag & drop fajlova/foldera, "Dodaj fajlove", "Dodaj folder" — jedna zajednička logika (parsiranje, spajanje split pkg-ova, odbacivanje duplikata).
- Red čekanja: instalacija jedan po jedan, promjena redoslijeda, pauza/nastavak/otkaz, red se čuva između pokretanja.
- Obavezna provjera instaliranosti prije svake instalacije; dijalog Preskoči/Reinstaliraj; provjera osnovne igre za Update/DLC.
- Deinstalacija sa konzole kao zasebna funkcija: za stavku iz liste (igra, update ili DLC) i ručno po Title ID / Content ID, uvijek uz potvrdu (6.7).
- Progress bar, brzina, ETA.
- Detaljan error log (šta / zašto / šta uraditi), log fajl, konzolin `/api/log` uz grešku, prevod poznatih kodova.
- Postavke: IP konzole (samo IPv4), port, test veze, port lokalnog servera, automatski PC IP, jezik, splash.
- Uputa za Windows Firewall pri prvom pokretanju + dugme "Dozvoli u firewallu" koje uz UAC potvrdu kreira usko pravilo za port lokalnog servera, i "Ukloni pravilo" (9.8).
- Jezici: engleski (default) i bosanski.
- Praćenje Windows teme (light/dark), uključujući živu promjenu sa pretapanjem boja.
- Fluidne animacije bez CPU troška u mirovanju.
- Splash screen sa animiranim logom.
- Mock RPI server (zaseban binary) za razvoj i CI bez konzole.
- GitHub Actions build, README sa screenshotima i disclaimerom.

### 1.2 Ne-ciljevi (v2 ili nikad)

| Stavka | Status |
|---|---|
| Skeniranje mreže / auto-otkrivanje PS4 | v2 |
| Windows toast obavještenja | v2 |
| Provjera novih verzija (auto-update) | v2 |
| Profil za originalni flatz RPI (port 12800, bez 3.57/3.58 pravila) | v2 |
| Lista svih instaliranih naslova sa konzole (RPI nema takav endpoint) | nije moguće |
| Ručni izbor teme (override Sistem/Tamna/Svijetla) | v2 |
| Više konzola, HTTPS, FTP, 32-bit Windows, Linux/macOS | nije planirano |
| Preuzimanje sadržaja s interneta, baze naslova, cover art | nikad (sve se čita lokalno iz pkg-a) |

### 1.3 Pravni i brend okvir

- README i About sadrže disclaimer: aplikacija nije povezana sa Sony Interactive Entertainment; namijenjena isključivo vlastitim, legalno napravljenim sigurnosnim kopijama i homebrew sadržaju.
- U UI-ju, logu, ikoni i README-u se ne koriste Sonyjevi simboli (trougao, krug, iks, kvadrat u stilu kontrolera), PS logo ni Sonyjeve boje dugmadi. Unicode znakovi `△ ○ × □` (i varijante `✕ ◯ ☐`) su zabranjeni u stringovima i logu; za "otkaži/ukloni" koriste se ikone (smeće, zabrana) i tekst.
- Riječ "PS4" se koristi samo deskriptivno ("PS4 konzola", "PS4 PKG"). Naziv aplikacije je odlučen: **PS Package Pusher** (rizik žiga prihvaćen uz disclaimer).
- Nosilac autorskog prava i autor u repou je pseudonim **pinkyBOZZ** (`Copyright (c) 2026 pinkyBOZZ`); pravo ime se ne pojavljuje u kodu, LICENSE-u, README-u, git historiji (commitovi sa GitHub "noreply" adresom) ni screenshotovima.

---

## 2. Vizuelni sistem

Polazište je logo "koncept 4": vaporwave scena (sunce sa prugama, neon mreža, izometrijska kutija, cyan strelica kroz kutiju). UI uzima pozadinu i neone iz loga; neon se koristi štedljivo — za akcente, fokus i aktivni progress, nikad kao glow na tekstu.

### 2.1 Paleta loga (izvor, ne mijenja se)

| Element | Hex |
|---|---|
| Pozadina | `#0D0221` |
| Pod | `#1A0638` |
| Sunce gradijent | `#FFD23D` → `#FF9E3D` → `#FF2E97` |
| Mreža | linije magenta `#FF2E97`, horizont cyan `#00F0FF` |
| Kutija top / lijeva / desna | `#7B2FF7` / `#2A0A5E` / `#4A1597` |
| Kutija obrub / traka | `#00F0FF` / `#FF2E97` |
| Strelica / speed linije | `#00F0FF` / `#FF2E97` |

### 2.2 Design tokeni — kontrast

Kontrast je izračunat po WCAG 2.x formuli relativne luminanse (sRGB linearizacija, `(L1+0.05)/(L2+0.05)`), ne procijenjen. Prag: tekst ≥ 4.5:1, UI elementi i grafika (borderi kontrola, progress fill, badge oblik, fokus) ≥ 3:1. Neprozirni overlay i alfa-tintovi su zamijenjeni punim hex vrijednostima da bi kontrast bio tačan.

Kolone kontrasta: **bg** = `bg.base`, **surf** = `bg.surface`, **s2** = `bg.surface2`, **sel** = `bg.selected`.

#### Dark tema

| Token | Hex | bg | surf | s2 | sel | Upotreba |
|---|---|---|---|---|---|---|
| `bg.base` | `#0D0221` | — | — | — | — | pozadina prozora, splash |
| `bg.surface` | `#1A0638` | — | — | — | — | paneli, redovi liste, dijalozi |
| `bg.surface2` | `#261052` | — | — | — | — | inputi, toolbar, progress track |
| `bg.hover` | `#33166B` | — | — | — | — | hover reda/dugmeta |
| `bg.selected` | `#2E1260` | — | — | — | — | selektovan red (+ lijeva traka 3 px `accent.primary`) |
| `bg.overlay` | `#0D0221` @ 60 % | — | — | — | — | pozadina iza modala |
| `text.primary` | `#F2EAFF` | 17.17 | 15.96 | 14.07 | 13.12 | tekst (na hover 12.26) |
| `text.secondary` | `#B8A8D9` | 9.20 | 8.55 | 7.54 | 7.03 | sekundarni tekst (na hover 6.57) |
| `text.disabled` | `#7A6A9E` | 4.18 | 3.89 | 3.43 | — | onemogućene kontrole (WCAG izuzetak, ipak ≥ 3:1) |
| `accent.primary` (cyan) | `#00F0FF` | 14.23 | 13.23 | 11.66 | 10.87 | linkovi, aktivni tab, primarno dugme (fill), fokus (na hover 10.16) |
| `accent.primary.on` | `#0D0221` na `#00F0FF` | 14.23 | | | | tekst na primarnom dugmetu |
| `accent.secondary` (magenta) | `#FF2E97` | 5.81 | 5.40 | 4.76 | 4.44 | samo fill/grafika: progress kraj, speed linije, sekundarni indikatori |
| `accent.secondary.text` | `#FF5CAD` | 7.05 | 6.55 | 5.78 | 5.39 | magenta tekst (na hover 5.03) |
| `purple` | `#7B2FF7` | 3.43 | 3.19 | — | — | samo dekoracija (velike površine, splash), nikad tekst |
| `purple.text` | `#B98CFF` | 7.88 | 7.33 | 6.46 | 6.02 | ljubičasti tekst, DLC badge fill |
| `success` | `#3DFFA8` | 15.34 | 14.26 | 12.57 | 11.71 | Završeno |
| `warning` | `#FFD23D` | 13.88 | 12.90 | 11.37 | 10.60 | Već instalirano, upozorenja |
| `danger` | `#FF5C7A` | 6.74 | 6.27 | 5.53 | 5.15 | greške (na hover 4.81) |
| `info` | `#5CC8FF` | 10.65 | 9.90 | 8.73 | 8.14 | Parsiranje, Provjera, info |
| `border.subtle` | `#3A1F6B` | — | 1.41 | — | — | samo dekorativni separatori (nije nosilac informacije) |
| `border.strong` | `#8B74C2` | 5.13 | 4.77 | 4.20 | — | obrub inputa, sekundarnih dugmadi, checkboxa |
| `focus.ring` | `#00F0FF` | 14.23 | 13.23 | 11.66 | 10.87 | 2 px prsten + 2 px razmak u boji pozadine + glow |
| `progress.fill` | `#00F0FF` → `#FF2E97` | | | 11.66 / 4.76 | | gradijent fill; kontrast prema tracku `bg.surface2` |
| `progress.track` | `#261052` | | | | | |

Dugmad (dark): primarno `#00F0FF` + tekst `#0D0221` = 14.23; sekundarno `bg.surface2` + `border.strong` + `text.primary` = 14.07; destruktivno `#FF5C7A` + tekst `#0D0221` = 6.74.

Badge-evi tipa (dark) — pun fill, tamni tekst:

| Badge | Fill | Tekst | Tekst/fill | Fill/surf |
|---|---|---|---|---|
| IGRA | `#00F0FF` | `#0D0221` | 14.23 | 13.23 |
| UPDATE | `#FF9E3D` | `#0D0221` | 9.74 | 9.06 |
| DLC | `#B98CFF` | `#0D0221` | 7.88 | 7.33 |

Banneri (dark), pozadina / naslov / tekst:

| Banner | Pozadina | Naslov (boja) | `text.primary` | `text.secondary` |
|---|---|---|---|---|
| warning | `#2E2208` | `#FFD23D` 10.78 | 13.34 | 7.15 |
| danger | `#3A0A1E` | `#FF5C7A` 5.69 | 14.48 | 7.76 |
| info | `#08283A` | `#5CC8FF` 8.12 | 13.08 | 7.01 |
| success | `#082E22` | `#3DFFA8` 11.28 | 12.63 | 6.77 |

#### Light tema

Svijetla tema zadržava ljubičasti ton pozadine i neon dekoracije (traka sunca na vrhu, gradijent progressa), a za tekst i kontrole koristi tamnije varijante neona.

| Token | Hex | bg | surf | s2 | sel | Upotreba |
|---|---|---|---|---|---|---|
| `bg.base` | `#F7F3FF` | — | — | — | — | pozadina prozora |
| `bg.surface` | `#FFFFFF` | — | — | — | — | paneli, redovi, dijalozi |
| `bg.surface2` | `#EFE7FF` | — | — | — | — | inputi, toolbar |
| `bg.hover` | `#F1EBFF` | — | — | — | — | hover |
| `bg.selected` | `#EAE0FF` | — | — | — | — | selektovan red (+ lijeva traka 3 px `accent.primary`) |
| `bg.overlay` | `#1A0638` @ 40 % | — | — | — | — | iza modala |
| `text.primary` | `#1A0638` | 17.05 | 18.63 | 15.57 | 14.73 | tekst (na hover 16.03) |
| `text.secondary` | `#584878` | 7.35 | 8.04 | 6.71 | 6.35 | sekundarni tekst (na hover 6.91) |
| `text.disabled` | `#9A8DB8` | 2.80 | 3.05 | 2.55 | — | onemogućene kontrole (WCAG izuzetak; uz to uvijek ikona/oblik onemogućenosti) |
| `accent.primary` (tamni cyan) | `#006A73` | 5.81 | 6.35 | 5.30 | 5.02 | linkovi, aktivni tab, primarno dugme (na hover 5.46) |
| `accent.primary.on` | `#FFFFFF` na `#006A73` | 6.35 | | | | tekst na primarnom dugmetu |
| `accent.secondary` (tamna magenta) | `#B3005F` | 6.24 | 6.81 | 5.69 | 5.39 | magenta tekst i fill, fokus (na hover 5.86) |
| `purple` | `#6A1FE0` | 6.66 | 7.28 | 6.08 | 5.76 | ljubičasti tekst, DLC badge |
| `success` | `#0A6640` | 6.43 | 7.03 | 5.87 | 5.56 | |
| `warning` | `#8A5300` | 5.79 | 6.33 | 5.29 | 5.00 | |
| `danger` | `#B0142F` | 6.43 | 7.03 | 5.87 | 5.56 | |
| `info` | `#005FA8` | 6.00 | 6.56 | 5.48 | 5.19 | |
| `border.subtle` | `#DCD0F5` | — | 1.46 | — | — | samo dekorativni separatori |
| `border.strong` | `#7F6CA8` | 4.17 | 4.55 | 3.80 | 3.60 | obrubi kontrola (na hover 3.92) |
| `focus.ring` | `#B3005F` | 6.24 | 6.81 | 5.69 | 5.39 | 2 px prsten + 2 px razmak; magenta da se razlikuje od cyan primarnog dugmeta |
| `progress.fill` | `#006A73` → `#B3005F` | | 6.35 / 6.81 | | | prema `bg.surface`; prema tracku 4.63 / 4.97 |
| `progress.track` | `#E3D6FF` | | | | | |
| `decor.sunset` | `#FFD23D` → `#FF9E3D` → `#FF2E97` | | | | | traka 2 px na vrhu prozora, nikad ispod teksta |

Dugmad (light): primarno `#006A73` + `#FFFFFF` = 6.35; sekundarno bijelo + `border.strong` (4.55) + `text.primary` 18.63; destruktivno `#B0142F` + `#FFFFFF` = 7.03.

Badge-evi tipa (light) — pun fill, bijeli tekst:

| Badge | Fill | Tekst | Tekst/fill | Fill/surf |
|---|---|---|---|---|
| IGRA | `#006A73` | `#FFFFFF` | 6.35 | 6.35 |
| UPDATE | `#8A5300` | `#FFFFFF` | 6.33 | 6.33 |
| DLC | `#6A1FE0` | `#FFFFFF` | 7.28 | 7.28 |

Banneri (light):

| Banner | Pozadina | Naslov (boja) | `text.primary` | `text.secondary` |
|---|---|---|---|---|
| warning | `#FFF4D6` | `#8A5300` 5.78 | 17.00 | 7.33 |
| danger | `#FFE3E8` | `#B0142F` 5.83 | 15.44 | 6.66 |
| info | `#E0F0FF` | `#005FA8` 5.65 | 16.04 | 6.92 |
| success | `#DDF7EA` | `#0A6640` 6.21 | 16.47 | 7.11 |

#### Statusi → token + ikona

Status se uvijek prikazuje kao **ikona + tekst** (nikad samo boja):

| Status | Token boje | Ikona (Phosphor) |
|---|---|---|
| Parsiranje | `info` | `spinner` (rotira samo dok traje) |
| Neispravan | `danger` | `file-x` |
| Spreman | `text.secondary` | `package` |
| U redu (#n) | `text.secondary` | `hourglass` |
| Provjera | `info` | `magnifying-glass` |
| Već instalirano | `warning` | `warning` |
| Deinstalira | `warning` | `arrows-clockwise` |
| Pokreće se / Šalje se / Instalira | `accent.primary` | `upload-simple` / `download-simple` |
| Pauzirano | `warning` | `pause` |
| Završeno | `success` | `check` |
| Greška | `danger` | `warning-octagon` |
| Otkazano | `text.secondary` | `prohibit` |
| Preskočeno | `text.secondary` | `skip-forward` |

#### Neon glow pravila

- Glow = 1–2 sloja zaobljenog pravougaonika iste boje sa alfom 25 % i 10 %, proširenje 2 i 4 px. Crta se painterom ispod elementa.
- Dozvoljen na: fokus prstenu, aktivnom tabu (podvlaka), progress fillu aktivne stavke, drop zoni u drag-over stanju, splash sceni.
- Zabranjen na: tekstu, ikonama u listi, neaktivnim elementima, light temi osim fokusa (light tema bez glowa, samo pun prsten).

#### Pretapanje teme

Paleta je struct `Palette` sa svim tokenima; aktivna paleta = `lerp(stara, nova, t)` po kanalima u linearnom RGB prostoru (`egui::Rgba`), `t` animiran tokenom `motion.theme` (sekcija 2.6). Splash je uvijek dark (brend), bez obzira na Windows temu.

### 2.3 Tipografija

| Uloga | Font | Veličina (pt) | Težina | Upotreba |
|---|---|---|---|---|
| `type.display` | Audiowide (OFL) | 26 | Regular | naziv na splashu, naslov u About |
| `type.title` | Inter (OFL) | 20 | SemiBold | naslov taba, naslov dijaloga |
| `type.heading` | Inter | 16 | SemiBold | naziv paketa u redu, sekcije Postavki |
| `type.body` | Inter | 14 | Regular | sav ostali tekst |
| `type.caption` | Inter | 12 | Regular | Title ID, Content ID, ETA, brzina, pomoćni tekst |
| `type.badge` | Inter | 11 | SemiBold, VERZAL, razmak +0.5 | badge-evi tipa |
| `type.mono` | egui default mono (Hack) | 13 | Regular | Log, tehnički detalji grešaka |

- Monoton se ne koristi: nečitljiv ispod ~40 px. Audiowide samo za ASCII naslove (naziv aplikacije), nikad za bosanske stringove sa dijakritikom.
- Ugrađivanje: `include_bytes!` TTF fajlova u `assets/fonts/`, registracija preko `egui::FontDefinitions` (`FontFamily::Proportional` = Inter Regular, `FontFamily::Name("inter-semibold")`, `FontFamily::Name("display")` = Audiowide). egui ne podržava variable font ose — koriste se statični `Inter-Regular.ttf` i `Inter-SemiBold.ttf`. Inter pokriva č ć đ š ž. Licence (`OFL.txt`) idu u `assets/fonts/` i u release zip.
- Brojevi u progressu/brzini/ETA: Inter sa tabular figures nije dostupan kroz egui; da brojevi ne "plešu", vrijednosti se renderuju u fiksno širokoj koloni (desno poravnato).

### 2.4 Spacing, radijusi, dimenzije

| Token | Vrijednost |
|---|---|
| `space.1..6` | 4, 8, 12, 16, 24, 32 px |
| `radius.sm` | 4 px (checkbox, mali indikatori) |
| `radius.md` | 8 px (dugmad, inputi, ikona paketa 6 px) |
| `radius.lg` | 12 px (kartice, dijalozi, drop zona) |
| `radius.splash` | 16 px |
| `radius.pill` | visina/2 (badge-evi) |
| Red liste | visina 72 px, ikona 48×48 px, padding 12 px |
| Minimalna ciljna površina | 32×32 px za sva dugmad/ikone |
| Glavni prozor | default 1100×720, minimum 880×560 |
| Splash | 480×300 px |
| Dijalog | širina 480 px (greška sa detaljima 640 px) |
| Fokus prsten | 2 px + 2 px razmak |

### 2.5 Ikone

- UI ikone: **Phosphor** (MIT) preko crate-a `egui-phosphor` (icon font, regular težina). Nema Sony simbola; za "otkaži" `prohibit`, za "ukloni" `trash`, za "zatvori dijalog" tekstualno dugme "Zatvori".
- Ikona aplikacije: `assets/icon/app.ico` sa 16, 24, 32, 48, 64, 128, 256 px.
  - 256/128/64/48: pun logo (sunce, mreža, kutija, strelica) na `#0D0221` kvadratu sa radijusom 18 %.
  - 32/24: bez mreže; polukrug sunca (2 pruge), kutija, strelica debljine ≥ 3 px.
  - 16: samo kutija (2 lica) + cyan strelica debljine 2 px preko nje, bez sunca i speed linija; pikselski poravnato (ručno crtan PNG, ne skaliran).
- Izvor loga: `assets/logo/logo.svg` (vektor, koncept 4; pravi ga Claude, PLAN 0.6). Splash scena se **ne** crta iz rastera nego proceduralno egui painterom po istoj geometriji, da bi se mogla animirati i bila oštra na svakom DPI-ju.

### 2.6 Animacijski tokeni

Easing funkcije su iz `egui::emath::easing`.

| Token | Trajanje | Easing | Upotreba |
|---|---|---|---|
| `motion.press` | 60 ms | `linear` | potamnjenje pri kliku |
| `motion.hover` | 100 ms | `cubic_out` | promjena boje pri hoveru |
| `motion.tab` | 180 ms | `cubic_out` | novi tab: alfa 0→1, pomak 12 px → 0 (smjer po indeksu taba) |
| `motion.dialog.in` | 180 ms | `cubic_out` | dijalog: alfa 0→1, skala 0.96→1; overlay alfa 0→ciljna u 150 ms |
| `motion.dialog.out` | 120 ms | `cubic_in` | obrnuto |
| `motion.item.in` | 200 ms | `cubic_out` | novi red: visina 0→72, alfa 0→1 |
| `motion.item.out` | 120 + 180 ms | `cubic_in`, pa `cubic_in_out` | uklanjanje: prvo alfa 1→0, zatim visina 72→0; ostali redovi klize jer layout koristi animiranu visinu |
| `motion.reorder` | 180 ms | `cubic_in_out` | premješteni red: y offset (stara_y − nova_y) → 0 |
| `motion.theme` | 300 ms | `cubic_in_out` | pretapanje palete |
| `motion.progress` | τ = 250 ms | eksponencijalno | prikazana vrijednost: `v += (cilj − v)·(1 − e^(−dt/τ))`, stop kad je razlika < 0.1 % |
| `motion.indeterminate` | 1200 ms petlja | `linear` | segment koji klizi u Provjera/Pokreće se; max 30 fps |
| `motion.splash.*` | vidi 4.1 | | |

Pravila performansi:

- egui crta samo na input ili `request_repaint`. Animacija traži repaint samo dok je aktivna (`ctx.animate_*` / vlastiti `Anim` tip koji vraća "još traje").
- Worker niti zovu `ctx.request_repaint()` samo kad pošalju događaj UI-ju. Progress se puni 1×/s, nakon čega smoothing radi ~250 ms → u aktivnom prenosu ~15 repaint-a/s u prosjeku, u mirovanju 0.
- Nema beskonačnih "pulse" efekata; glow je statičan.
- Ako Windows ima isključeno "Show animations in Windows" (`SPI_GETCLIENTAREAANIMATION` = false), sva trajanja su 0 (osim progress smoothinga, koji ostaje jer prenosi informaciju), a splash animacija se preskače i prikazuje se statična scena.

---

## 3. Navigacija i struktura prozora

Glavni prozor ima zaglavlje i tri taba: **Red čekanja** (Queue), **Postavke** (Settings), **Log**. Tab se mijenja klikom, `Ctrl+1/2/3` ili `Ctrl+Tab`. Prelaz: novi sadržaj ulazi sa `motion.tab` (egui je immediate-mode, stari tab se ne crta tokom prelaza — samo novi ulazi).

Zaglavlje (uvijek vidljivo):

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ decor.sunset traka 2 px ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀│
│ [logo 24] PS Package Pusher    Red čekanja (3)   Postavke   Log (1)                  │
│                                ═══════════                    ● Konzola 192.168.1.50 │
│                                                               (povezano, 18 ms)      │
└────────────────────────────────────────────────────────────────────────────────────┘
```

- Brojač uz "Red čekanja" = broj nezavršenih stavki; uz "Log" = broj nepročitanih grešaka (badge `danger`).
- Indikator konzole: ikona + tekst — "Povezano", "Nedostupna", "Nije podešeno", "Provjeravam…". Ažurira se pri startu, nakon "Test veze" i iz rezultata svakog API poziva; nema periodičnog pinganja. Klik vodi na Postavke.
- Globalni banner (ispod zaglavlja, iznad sadržaja taba) za stanja koja blokiraju rad: port zauzet, PC IP nepoznat, konzola nije podešena. Banner ima dugme za akciju ("Otvori Postavke").

---

## 4. Ekrani (wireframe-ovi)

Wireframe-ovi prikazuju bosanske stringove; engleski su ekvivalenti po istim i18n ključevima.

### 4.1 Splash

Mali prozor bez okvira, centriran na primarnom monitoru, 480×300, zaobljen `radius.splash`, uvijek dark.

```
╭──────────────────────────────────────────────╮
│                                              │
│              ▄▄▄▄▄▄▄▄▄▄▄▄▄                   │   sunce (pruge, gradijent)
│            ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄                 │
│      ═══ ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄ ═══           │
│ ─────────────────[ ◇ kutija ]──────────────  │   cyan horizont
│   ╲    ╲    ╲  ===>>>>[==]===>    ╱    ╱     │   strelica kroz kutiju + speed linije
│     ╲    ╲    ╲    │    ╱    ╱    ╱          │   magenta mreža u perspektivi
│                                              │
│              PS Package Pusher               │   type.display, text.primary
│                   v0.0.1                     │   type.caption, text.secondary
│                                              │
│  Pokrećem lokalni server…                    │   type.caption, text.secondary
│  ━━━━━━━━━━━━━━━━━━━━━━━━──────────────────  │   linija 2 px, progress.fill
╰──────────────────────────────────────────────╯
```

Vremenska linija animacije (ms od prikaza prozora):

| Od–do | Element | Animacija | Easing |
|---|---|---|---|
| 0–150 | cijela scena | alfa 0→1 | `cubic_out` |
| 100–700 | sunce | izlazi: y +40 px → 0, pruge se otkrivaju odozdo | `cubic_out` |
| 300–360 | horizont | cyan linija bljesne (alfa 0→1, glow 1→0.4) | `linear` |
| 300–900 | mreža | linije se pale od horizonta naniže, razmak 40 ms po liniji, alfa 0→1 | `cubic_out` |
| 500–800 | kutija | skala 0.9→1, alfa 0→1 | `back_out` |
| 750–1150 | strelica | prolijeće s lijeva na desno kroz kutiju; magenta speed linije iza nje blijede 200 ms | `cubic_in_out` |
| ≥ 1150 | scena | statična (bez repaint-a), mijenja se samo progress linija i tekst koraka | — |
| kraj | prozor | fade out 200 ms, zatim glavni prozor fade in 200 ms | `cubic_in` / `cubic_out` |

Koraci (izvršavaju se u pozadinskoj niti, ne u UI niti):

1. "Učitavam postavke…" — `config.json`.
2. "Učitavam red čekanja…" — `queue.json`, provjera postojanja fajlova.
3. "Pokrećem lokalni server…" — određivanje PC IP-a, bind servera.
4. "Provjeravam port…" — loopback zahtjev na vlastiti server (HEAD `/healthz`).

Progress linija = broj završenih koraka / 4, animirano `motion.progress`.

Pravila:

- Splash traje `max(1200 ms, trajanje koraka)`, a najviše 5 s (koraci imaju timeoute). Greška u koraku ne zaustavlja splash — prikazuje se kao banner u glavnom prozoru.
- Klik bilo gdje, `Esc`, `Enter` ili `Space` preskače animaciju; prozor se zatvara čim su koraci gotovi.
- Provjera veze sa konzolom se pokreće paralelno i **ne** blokira splash.
- Postavka "Prikaži splash pri pokretanju" = isključeno → koraci se izvrše prije prvog frame-a glavnog prozora (glavni prozor ne prikazuje sadržaj dok koraci ne završe, max 5 s; tipično < 200 ms).

Implementacija prozora: winit dozvoljava jedan event loop po procesu, pa se splash i glavni prozor ne mogu pokrenuti kao dva `run_native`. Odluka: **root viewport se "preobražava"** — počinje kao borderless, transparentan, 480×300; na kraju splasha šalju se `ViewportCommand::Decorations(true)`, `InnerSize`, `Resizable(true)`, `OuterPosition` (centrirano ili sačuvana pozicija), uz fade sadržaja. Rezervna varijanta ako preobražaj treperi: glavni prozor kao `show_viewport_deferred`, root se skriva. **[PROVJERA]** prototipom na Windows 11 (treperenje, DWM zaobljenje, transparentnost, DPI).

### 4.2 Red čekanja — prazno stanje

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ [+ Dodaj fajlove]  [+ Dodaj folder]                                                  │
├────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│     ╭ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─╮        │
│     │                                                                       │        │
│     │                        [ikona: kutija + strelica]                     │        │
│     │                                                                       │        │
│     │                 Prevuci .pkg fajlove ili foldere ovdje                │        │
│     │        ili koristi "Dodaj fajlove" / "Dodaj folder" (Ctrl+O)          │        │
│     │                                                                       │        │
│     │     Split paketi (_0, _1, …) se spajaju automatski.                   │        │
│     ╰ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─╯        │
│                                                                                      │
└────────────────────────────────────────────────────────────────────────────────────┘
```

Drop zona: isprekidan obrub 2 px `border.strong`, `radius.lg`, tekst `text.secondary`.

### 4.3 Red čekanja — puna lista

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ [+ Dodaj fajlove] [+ Dodaj folder] │ [▶ Pokreni red] [Zaustavi nakon trenutnog] │ [⋯]  │
│                                     │ [Očisti završene]        4 paketa · 61,3 GB  │
├────────────────────────────────────────────────────────────────────────────────────┤
│ ⠿ #1 ┌────┐ Neki Naslov Igre                    [IGRA]   v01.00          42,1 GB   │
│      │ICON│ CUSA12345 · UP0000-CUSA12345_00-…                                       │
│      └────┘ ⬇ Šalje se  ███████████████▒▒▒▒▒▒▒▒▒▒  61 %  38,5 MB/s  ~7 min          │
│                                                     [Pauziraj] [Otkaži]             │
├────────────────────────────────────────────────────────────────────────────────────┤
│ ⠿ #2 ┌────┐ Neki Naslov Igre                    [UPDATE] v01.05           3,2 GB   │
│      │ICON│ CUSA12345 · čeka osnovnu igru (#1)                                       │
│      └────┘ ⌛ U redu                                                    [Ukloni]    │
├────────────────────────────────────────────────────────────────────────────────────┤
│ ⠿ #3 ┌────┐ Druga Igra                          [IGRA]   v01.00          15,9 GB   │
│      │ICON│ CUSA54321 · split: 3 dijela                                             │
│      └────┘ ⚠ Već instalirano — odluka: Reinstaliraj               [Promijeni] [Ukloni]│
├────────────────────────────────────────────────────────────────────────────────────┤
│   ✓  ┌────┐ Treća Igra — Dodatak                [DLC]    v01.00           0,1 GB   │
│      │ICON│ CUSA11111 · završeno 14:02                                              │
│      └────┘ ✓ Završeno                                                  [Ukloni]    │
└────────────────────────────────────────────────────────────────────────────────────┘
```

Napomena: znakovi `▶ ⬇ ⌛ ⚠ ✓ ⠿` u wireframe-u predstavljaju Phosphor ikone iz 2.2, ne Unicode znakove u aplikaciji.

Kolone/elementi reda:

- **⠿** ručka za prevlačenje (vidljiva na hover/fokus); **#n** = pozicija u redu (samo za nezavršene stavke; završene/preskočene/otkazane imaju ikonu statusa umjesto broja).
- Ikona 48×48 iz `icon0.png`; ako je nema — generička ikona kutije na `bg.surface2`.
- Naziv (`type.heading`, skraćen sa "…" i tooltipom), badge tipa, verzija, veličina (desno).
- Drugi red: Title ID · Content ID (skraćen) ili kontekstualna poruka (split, čeka osnovnu igru, razlog greške u jednoj rečenici).
- Treći red: status (ikona + tekst), progress bar (samo u aktivnim stanjima), procenat, brzina, ETA, dugmad za akcije po stanju (tabela u sekciji 5).
- Klik na red ga selektuje; dvoklik na stavku sa greškom otvara dijalog greške; desni klik otvara kontekstni meni (Pomjeri na vrh / gore / dolje / na dno, Otvori lokaciju fajla, Kopiraj Title ID, Deinstaliraj sa konzole…, Ukloni). "Deinstaliraj sa konzole…" je dostupno u stanjima iz 6.7 i vodi kroz potvrdu 4.11; "Ukloni" briše samo stavku iz liste, nikad sa konzole.
- Meni `[⋯]` u toolbaru: "Deinstaliraj po ID…" (dijalog 4.12), za naslove koji nisu u listi.
- Promjena redoslijeda: drag & drop ručkom (egui `dnd_drag_source`/`dnd_drop_zone`), `Alt+↑/↓`, ili kontekstni meni. Aktivna stavka se ne može pomjerati.
- Podnožje toolbara: broj paketa, ukupna veličina preostalih, ukupni ETA (zbir preostalih bajtova / izglađena brzina; prikazuje se tek kad postoji izmjerena brzina).

### 4.4 Red čekanja — drag-over stanje

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓ │
│ ┃ (lista ostaje vidljiva ispod overlay-a bg.overlay 60 % / 40 %)                   ┃ │
│ ┃                                                                                  ┃ │
│ ┃                    [ikona: strelica u kutiju, accent.primary]                    ┃ │
│ ┃                         Pusti za dodavanje u red                                 ┃ │
│ ┃                      3 stavke (fajlovi i folderi)                                ┃ │
│ ┃                                                                                  ┃ │
│ ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛ │
└────────────────────────────────────────────────────────────────────────────────────┘
```

- Aktivira se na `hovered_files` (egui `RawInput`), bez obzira na trenutni tab (automatski prebacuje na Red čekanja nakon puštanja).
- Obrub 2 px `accent.primary` + glow (dark), `radius.lg`; fade in/out 120 ms.
- Windows ne javlja ekstenzije prije puštanja pouzdano, pa se ne-pkg fajlovi filtriraju tek nakon puštanja (javlja se "Preskočeno N fajlova koji nisu .pkg" kao info toast unutar aplikacije, 4 s).

### 4.5 Postavke

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ Postavke                                                                             │
│                                                                                      │
│ KONZOLA                                                                              │
│   IP adresa konzole   [192.168.1.50      ]   samo IPv4, npr. 192.168.1.50            │
│   Port RPI-ja         [12801 ]               default 12801 (RPI 3.58)                │
│   [Test veze  F5]                                                                    │
│   ╭──────────────────────────────────────────────────────────────────────────────╮  │
│   │ ✓ RPI odgovara na 192.168.1.50:12801 (18 ms)                                  │  │
│   │ ✓ /api/log dostupan — RPI 3.57 ili noviji                                     │  │
│   │ ✓ Lokalni server radi na 192.168.1.10:8095                                    │  │
│   │ ⓘ Firewall se može potvrditi tek pri prvom prenosu.                           │  │
│   ╰──────────────────────────────────────────────────────────────────────────────╯  │
│                                                                                      │
│ OVAJ RAČUNAR                                                                         │
│   IP ovog računara    (•) Automatski: 192.168.1.10 (interfejs prema konzoli)         │
│                       ( ) Ručno [               ]                                    │
│   Port lokalnog servera [8095 ]                                                      │
│   Windows Firewall    ✓ Pravilo aktivno (TCP 8095, Private)  [Ukloni pravilo] [Uputa]│
│                       (ili: ⚠ Nema pravila  [Dozvoli u firewallu] [Uputa])           │
│                                                                                      │
│ APLIKACIJA                                                                           │
│   Jezik               [English        ▾]                                             │
│   [x] Prikaži splash pri pokretanju                                                  │
│   Folder sa podacima  %APPDATA%\PS Package Pusher   [Otvori folder]                  │
│                                                                                      │
│                                                     [Vrati podrazumijevano] [Sačuvaj]│
└────────────────────────────────────────────────────────────────────────────────────┘
```

- Validacija u toku kucanja: IP mora biti IPv4 literal (`std::net::Ipv4Addr::from_str`), ne `0.0.0.0`, ne `255.255.255.255`, ne multicast. Hostname daje inline grešku: "Unesi IP adresu, ne ime (RPI 3.58 odbija imena zbog zaštite od DNS rebindinga)." Port 1–65535; lokalni port ≠ port konzole nije uslov (različite mašine), ali < 1024 daje upozorenje.
- "Sačuvaj" je aktivno samo kad ima promjena i sve je validno. Promjena IP-a/porta lokalnog servera restartuje server (nije dozvoljeno dok traje prenos — dugme onemogućeno uz objašnjenje).
- Jezik se primjenjuje odmah (bez restarta).
- Rezultat testa veze je lista provjera, svaka sa ikonom + tekstom (format u 6.5).
- Red "Windows Firewall" prikazuje stanje pravila (9.8): aktivno / nema pravila / pravilo za drugi port ("Ažuriraj pravilo") / lokalna pravila blokirana politikom (bez dugmadi, samo objašnjenje). Stanje se čita u pozadini pri otvaranju Postavki i nakon svake promjene.

### 4.6 Log

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│ Log   [Sve ▾] [x] Greške [x] Upozorenja [x] Info [ ] Debug   [Traži…        ]       │
│                                         [Kopiraj] [Kopiraj sve] [Otvori folder]      │
├────────────────────────────────────────────────────────────────────────────────────┤
│ 14:01:12  INFO   Dodano 3 paketa (1 split, 1 duplikat odbačen)                       │
│ 14:01:40  INFO   [CUSA12345] Provjera instaliranosti: nije instalirano               │
│ 14:01:41  INFO   [CUSA12345] Instalacija pokrenuta, task_id=7                        │
│▸14:09:03  GREŠKA [CUSA54321] Konzola je prijavila grešku pri preuzimanju             │
│ ╭────────────────────────────────────────────────────────────────────────────────╮  │
│ │ Šta:   Instalacija nije uspjela (BGFT 0x80990xxx).                              │  │
│ │ Zašto: <prevod koda ili "nepoznat kod">                                         │  │
│ │ Uradi: <preporučena akcija>                                                     │  │
│ │ Detalji (mono): HTTP 200 {"status":"fail","error":…} · task_id=8 · URL …       │  │
│ │ Konzolin log (zadnjih 200 linija) [prikaži ▾]                                   │  │
│ ╰────────────────────────────────────────────────────────────────────────────────╯  │
└────────────────────────────────────────────────────────────────────────────────────┘
```

- U UI-ju se drži prsten od zadnjih 5000 unosa; puni log je u fajlu.
- Filter po nivou, tekstualna pretraga, filter "Sve / samo izabrani paket".
- Unos greške je proširiv (klik ili `Enter`) i prikazuje strukturu Šta / Zašto / Šta uraditi / tehnički detalji / konzolin log.
- "Kopiraj" kopira selektovani unos (sa detaljima), "Kopiraj sve" vidljive (filtrirane) unose, kao običan tekst pogodan za GitHub issue. "Otvori folder" otvara `%APPDATA%\PS Package Pusher\logs` u Exploreru.

### 4.7 Dijalog "Već instalirano"

```
╭──────────────────────────────────────────────────────────────╮
│ ⚠  Igra je već instalirana                                    │
│                                                              │
│  ┌────┐  Druga Igra                                          │
│  │ICON│  CUSA54321 · v01.00 · 15,9 GB                        │
│  └────┘                                                      │
│                                                              │
│  Na konzoli već postoji CUSA54321. Instalacija preko         │
│  postojeće može izazvati konflikt.                           │
│                                                              │
│  Reinstaliraj = deinstalira postojeću igru (sa konzole se    │
│  briše igra; sačuvane pozicije ostaju*), pa instalira ovaj   │
│  paket.                                                      │
│                                                              │
│  [ ] Primijeni na sve preostale pakete u redu                │
│                                                              │
│                       [Preskoči]   [Reinstaliraj]            │
╰──────────────────────────────────────────────────────────────╯
```

- Modal; red čekanja čeka odluku (ne prelazi na sljedeću stavku).
- Default fokus na **Preskoči** (nedestruktivna opcija); `Esc` = Preskoči. Reinstaliraj je destruktivno dugme (`danger`).
- *Da li `uninstall_game` čuva save podatke: **[PROVJERA]**; tekst se prilagođava rezultatu provjere.
- Ako se dijalog pojavi tokom predprovjere (stavka još nije na redu), odluka se pamti u stavci (`decision`) i ponovo potvrđuje provjerom neposredno prije instalacije.

### 4.8 Dijalog greške sa detaljima

```
╭──────────────────────────────────────────────────────────────────────────────╮
│ ⛔ Instalacija nije uspjela — Druga Igra (CUSA54321)                          │
│                                                                              │
│ Šta se desilo                                                                │
│   Konzola je prekinula preuzimanje: računar nije slao podatke 30 s.          │
│ Zašto                                                                        │
│   Najčešće: računar je otišao u stanje mirovanja, fajl je na sporom/         │
│   odspojenom disku, ili je mrežna veza pala.                                 │
│ Šta uraditi                                                                  │
│   1. Provjeri da je disk sa fajlom dostupan.                                 │
│   2. Klikni "Pokušaj ponovo".                                                │
│                                                                              │
│ ▸ Tehnički detalji                                                           │
│ ▸ Konzolin log (preuzet 14:09:05)                                            │
│                                                                              │
│ [Kopiraj detalje]  [Otvori Log]                  [Zatvori]  [Pokušaj ponovo] │
╰──────────────────────────────────────────────────────────────────────────────╯
```

### 4.9 Dijalog "Windows Firewall" (prvo pokretanje i na zahtjev)

```
╭──────────────────────────────────────────────────────────────────────────────╮
│ ⓘ Dozvoli konzoli pristup ovom računaru                                      │
│                                                                              │
│ Konzola preuzima pakete direktno sa ovog računara (port 8095). Windows       │
│ Firewall to po defaultu blokira.                                             │
│                                                                              │
│ 1. Kad Windows prikaže "Windows Security Alert" za PS Package Pusher,        │
│    označi "Private networks" i klikni "Allow access".                        │
│ 2. Mreža na kojoj su računar i konzola mora biti postavljena kao             │
│    "Private" (Settings → Network & internet → <mreža> → Private network).    │
│ 3. Ako si ranije kliknuo "Cancel": Control Panel → Windows Defender          │
│    Firewall → Allow an app through firewall → PS Package Pusher → Private.   │
│ 4. VPN i "AP/client isolation" na ruteru također mogu blokirati vezu.        │
│                                                                              │
│ Ili prepusti aplikaciji: Windows će tražiti potvrdu (UAC), a pravilo važi    │
│ samo za port 8095, Private mrežu i lokalnu podmrežu.                         │
│                                                                              │
│ [ ] Ne prikazuj ponovo   [Kopiraj upute]  [Razumijem]  [Dozvoli u firewallu] │
╰──────────────────────────────────────────────────────────────────────────────╯
```

- Prikazuje se pri prvom pokretanju, nakon splasha, **prije** prvog binda servera na LAN IP (da korisnik zna šta znači Windowsov prompt koji slijedi). Redoslijed: splash → glavni prozor → ovaj dijalog → bind (ako je pri prvom pokretanju bind odgođen).
- PSPP nikad ne mijenja firewall bez korisnika: "Dozvoli u firewallu" pokreće postupak 9.8 koji uvijek prolazi kroz Windowsov UAC prozor. Glavni proces aplikacije radi bez admin prava.
- Ponovo dostupan iz Postavki i iz greške FW-01.

### 4.10 Ostali dijalozi (kratko)

- **Zatvaranje tokom prenosa**: "Prenos je u toku. Ako zatvoriš aplikaciju, konzola će prekinuti preuzimanje za ~30 s." [Ostani] (default) [Zatvori].
- **Uklanjanje aktivne stavke**: "Otkazati instalaciju i ukloniti iz reda?" [Ne] (default) [Otkaži i ukloni].
- **Promjena odluke** (Već instalirano → "Promijeni"): isti dijalog kao 4.7.

### 4.11 Dijalog "Deinstaliraj sa konzole"

```
╭──────────────────────────────────────────────────────────────╮
│ ⛔ Deinstalirati sa konzole?                                  │
│                                                              │
│  ┌────┐  Druga Igra                              [IGRA]      │
│  │ICON│  CUSA54321 · v01.00                                  │
│  └────┘                                                      │
│                                                              │
│  Igra će biti obrisana sa konzole. Sačuvane pozicije         │
│  ostaju*. Ovo se ne može poništiti.                          │
│                                                              │
│                         [Odustani]   [Deinstaliraj]          │
╰──────────────────────────────────────────────────────────────╯
```

- Tekst zavisi od tipa: **Igra** → "Igra će biti obrisana…" (+ "instalirani update i DLC se također brišu*" ako se to potvrdi); **Update** → "Update će biti obrisan; igra ostaje na osnovnoj verziji."; **DLC** → "Dodatak će biti obrisan; igra ostaje."
- *Ponašanje za save podatke i za update/DLC uz brisanje igre: **[PROVJERA]**; tekst se prilagođava rezultatu.
- Default fokus i `Esc` = **Odustani**; Deinstaliraj je destruktivno dugme (`danger`). Nema "primijeni na sve": svaka deinstalacija se potvrđuje posebno.
- Ako engine prije slanja utvrdi (`is_exists`) da naslov ne postoji, dijalog se ne otvara; prikazuje se info "Nije instalirano na konzoli".

### 4.12 Dijalog "Deinstaliraj po ID"

```
╭──────────────────────────────────────────────────────────────╮
│ Deinstaliraj po ID                                           │
│                                                              │
│  Tip      (•) Igra   ( ) Update   ( ) DLC                    │
│  Title ID [CUSA54321           ]   ✓ Instalirano             │
│                                                              │
│                [Odustani]   [Provjeri]   [Deinstaliraj…]     │
╰──────────────────────────────────────────────────────────────╯
```

- Igra i Update traže **Title ID** (`^[A-Z]{4}\d{5}$`, velika slova se nameću pri unosu); DLC traži **Content ID** (`^[A-Z]{2}\d{4}-[A-Z]{4}\d{5}_\d{2}-[A-Z0-9]{16}$`). Neispravan unos → poruka ispod polja (CFG-03), dugmad onemogućena.
- "Provjeri" radi `is_exists` i prikazuje rezultat uz polje (ikona + tekst). "Deinstaliraj…" je omogućeno tek kad provjera kaže "Instalirano" i otvara potvrdu 4.11 (bez naziva i ikone, samo ID i tip).
- Naziv i ikona nisu dostupni (RPI ih ne vraća); ako je isti ID u listi, preuzimaju se iz te stavke.

---

## 5. Stanja paketa (state machine)

### 5.1 Stanja

| Stanje | Opis | UI (status / progress / akcije) | Persistira se kao |
|---|---|---|---|
| `Parsing` | čitanje zaglavlja, param.sfo, ikone | "Parsiranje…", indeterminate; Ukloni | (ne čuva se; pri učitavanju ponovo) |
| `Invalid` | nije validan PS4 pkg / nedostaje split dio | "Neispravan: <razlog>"; Ukloni | `invalid` |
| `Ready` | validan, red nije pokrenut | "Spreman" + info predprovjere ("Nije instalirano" / "Čeka osnovnu igru #n"); Ukloni, pomjeri | `ready` |
| `Queued` | red je pokrenut, čeka svoj red | "U redu" + #n; Ukloni, pomjeri | `ready` |
| `Checking` | `is_exists` neposredno prije instalacije (ili predprovjera) | "Provjera…", indeterminate | `ready` |
| `AwaitingDecision` | igra postoji, nema odluke | "Već instalirano" + dijalog; Promijeni, Ukloni | `ready` + `installed_hint` |
| `Uninstalling` | `uninstall_*` u toku (Reinstaliraj ili samostalna deinstalacija, 6.7) | "Deinstalira…", indeterminate | `ready` (samostalna: prethodno stanje) |
| `Starting` | poslan `install`, čeka `task_id` / prve bajtove | "Pokreće se…", indeterminate | `ready` |
| `Transferring` | konzola povlači fajl | "Šalje se", %, brzina, ETA; Pauziraj, Otkaži | `interrupted` |
| `Installing` | prenos 100 %, konzola instalira (ako je detektabilno, **[PROVJERA]**) | "Instalira…", indeterminate; Otkaži | `interrupted` |
| `Paused` | `pause_task` uspio | "Pauzirano", zamrznut progress; Nastavi, Otkaži | `interrupted` |
| `Done` | završeno (i potvrđeno `is_exists` za igre) | "Završeno <vrijeme>"; Ukloni | `done` |
| `Failed` | greška | "Greška: <jedna rečenica>"; Detalji, Pokušaj ponovo, Ukloni | `failed` + `error` |
| `Cancelled` | korisnik otkazao | "Otkazano"; Pokušaj ponovo, Ukloni | `cancelled` |
| `Skipped` | odluka Preskoči | "Preskočeno (već instalirano)"; Pokušaj ponovo, Ukloni | `skipped` |

### 5.2 Prelazi

```
            add
             │
          Parsing ──(greška parsiranja)──► Invalid
             │ ok
             ▼
           Ready ◄──────────────(Zaustavi red)────────── Queued
             │ (predprovjera, ako je konzola dostupna;      ▲
             │  rezultat samo kao info u Ready)             │ Pokreni red
             └──────────────────────────────────────────────┘
                                                            │ stavka na redu
                                                            ▼
                                                        Checking
              ┌───────────────────────┬────────────────────┼────────────────────────┐
     igra postoji,               igra postoji,        ne postoji / Update-DLC     baza ne postoji
     nema odluke                 odluka postoji       sa postojećom bazom          (Update/DLC) ili
              │                       │                    │                       greška mreže
              ▼                       │                    │                           │
      AwaitingDecision ──Preskoči──► Skipped              │                           ▼
              │ Reinstaliraj          │ Reinstaliraj       │                         Failed
              ▼                       ▼                    │
         Uninstalling ◄───────────────┘                    │
              │ ok (ili 404 = nije instalirano)            │
              └──────────────────────────► Starting ◄──────┘
                                              │ task_id
                                              ▼
                            Pause ┌──── Transferring ────┐ error / stall / konzola nestala
                       Paused ◄───┤          │           ├──────────────────────► Failed
                         │ Resume │          │ 100 %     │
                         └───────►┘          ▼           │
                                         Installing ─────┘
                                              │ potvrda
                                              ▼
                                             Done

  Otkaži iz {Starting, Transferring, Installing, Paused} ──► Cancelled
  Pokušaj ponovo iz {Failed, Cancelled, Skipped} ──► Ready (odluka se briše; Failed sa važećim
                                                     task_id prvo pokušava resume_task, 9.3)
  Deinstaliraj (6.7) iz {Ready, Done, Skipped, Failed, Cancelled} ──► Uninstalling
                                                     ──ok──► Ready ("Nije instalirano")
                                                     ──greška──► prethodno stanje + RPI-08
```

Pravila:

- Istovremeno je najviše jedna stavka u stanjima `Checking…Installing/Paused` (jedan po jedan).
- `Paused` drži red: sljedeća stavka ne kreće dok aktivna nije završena/otkazana.
- `Failed` ne zaustavlja red: red nastavlja sa sljedećom stavkom; red se zaustavlja samo ako je greška globalna (konzola nedostupna, server pao, port zauzet) — tada sve `Queued` vraća u `Ready` i prikazuje banner.
- Nakon pokretanja aplikacije stanja `Transferring/Installing/Paused` (persistirana kao `interrupted`) postaju `Failed` sa greškom RPI-07 "Prekinuto zatvaranjem aplikacije"; "Pokušaj ponovo" prvo provjerava sačuvani `task_id` (9.3).
- Fajl nestao/promijenjen (veličina ili mtime) pri učitavanju reda ili prije slanja → `Failed` (PKG-07).

---

## 6. Tokovi

### 6.1 Dodavanje (drop, "Dodaj fajlove", "Dodaj folder")

Sva tri ulaza proizvode listu putanja i šalju jednu komandu `AddPaths(Vec<PathBuf>)` engine-u.

1. **Ekspanzija**: folderi rekurzivno (bez praćenja symlink/junction petlji: `symlink_metadata`, set posjećenih kanonskih putanja, max dubina 16). Zadržavaju se samo fajlovi sa ekstenzijom `.pkg` (case-insensitive). Ostalo se broji i javlja jednom info porukom.
2. **Grupisanje split paketa**: ime koje odgovara `^(.*)_(\d+)\.pkg$` grupiše se po (folder, prefiks). Grupa je validna ako postoji `_0` i indeksi su neprekinuti 0..n. Nedostaje dio → `Invalid` (PKG-04) sa listom nedostajućih. Fajl `X_0.pkg` bez `X_1.pkg` je običan (ne-split) paket. Fajl čije ime završava na `_N` ali je samostalan validan pkg (ima magic, a `_0` ne postoji) tretira se kao običan paket. **[PROVJERA]** konvencija imenovanja split pkg-ova koju RPI 3.58 očekuje i da li samo `_0` nosi zaglavlje.
3. **Parsiranje** (pool od najviše 4 niti): zaglavlje iz prvog dijela, param.sfo, icon0 (sekcija 9.5). Stavka se odmah pojavljuje u listi kao `Parsing` (animacija `motion.item.in`).
4. **Duplikati** (provjera nakon parsiranja):
   - ista kanonska putanja (`std::fs::canonicalize`) prvog dijela kao postojeća stavka → odbačeno tiho, uz info u logu;
   - isti `CONTENT_ID` + ista verzija + ista ukupna veličina → odbačeno, info poruka "Duplikat: <naziv> je već u redu (#n)";
   - isti `CONTENT_ID`, različita verzija → dozvoljeno (npr. dva updatea), upozorenje u logu.
   - Stavke u stanju `Done/Skipped/Cancelled/Failed` se takođe računaju (korisnik ih može ukloniti pa ponovo dodati).
5. **Redoslijed nove grupe**: stavke dodane jednom akcijom se sortiraju: igre (`gd`), pa updatei (`gp`) po verziji rastuće, pa DLC (`ac`), unutar tipa po nazivu. Dodaju se na kraj reda.
6. **Predprovjera**: ako je konzola podešena i dostupna, engine u pozadini radi `is_exists` za nove stavke (sekvencijalno, timeout 3 s) i postavlja info ("Nije instalirano", "Već instalirano" → dijalog 4.7, "Čeka osnovnu igru (#n)" ako je baza u redu ispred, "Osnovna igra nije instalirana" kao upozorenje). Predprovjera nikad ne zamjenjuje obaveznu provjeru u 6.2.
7. `queue.json` se snima (debounce 500 ms).

### 6.2 Instalacija iz reda

Korisnik klikne "Pokreni red" (`Ctrl+Enter`). Za svaku stavku redom:

1. **Preduslovi**: server radi i sluša na PC IP-u; PC IP je u istoj podmreži kao konzola (ako nije — upozorenje, ne blokada, jer rutirane mreže su moguće); fajlovi postoje i nepromijenjeni su.
2. **Registracija**: stavka se registruje u serveru (whitelist) → URL-ovi `http://<PC_IP>:<port>/pkg/<item_id>/<safe-name>` za svaki dio, redom.
3. **Checking** (obavezno, uvijek):
   - Igra (`gd`): `is_exists(TITLE_ID)`. Postoji → odluka iz stavke ili dijalog 4.7. Ne postoji → dalje.
   - Update (`gp`) / DLC (`ac`): `is_exists(TITLE_ID baze)` (TITLE_ID iz param.sfo; ako fali, iz Content ID-a, znakovi 7–15). Ne postoji → `Failed` (RPI-06). Postoji → dalje. Da li `is_exists` prepoznaje već instaliran update/DLC i vraća verziju: **[PROVJERA]**; ako vraća verziju, ista ili novija verzija updatea tretira se kao "Već instalirano" (dijalog sa opcijama Preskoči / Instaliraj ponovo bez deinstalacije).
4. **Uninstalling** (samo Reinstaliraj za igru): `POST /api/uninstall_game {"title_id":…}`. 200/success → dalje; 404 sa RPI JSON tijelom = "nije instalirano" → dalje (log info); ostalo → `Failed`. Zatim ponovni `is_exists` mora vratiti "ne postoji" (max 3 pokušaja, razmak 2 s).
5. **Starting**: `POST /api/install {"type":"direct","packages":[url_0, url_1, …]}` → `task_id`. Odgovor `fail` → `Failed` (RPI-01). Engine pamti `task_id` u stavci i snima red.
6. **Transferring**: polling `get_task_progress` svake 1 s (9.2). Paralelno server bilježi zadnju aktivnost za tu stavku.
   - Nijedan HTTP zahtjev konzole ka serveru u prvih 20 s nakon `task_id` → `Failed` (FW-01).
   - `error ≠ 0` u progresu → `Failed` (RPI-04) + preuzimanje `/api/log`.
   - Napredak ne raste 60 s, a nije `Paused` → `Failed` (RPI-05).
7. **Installing / Done**: kad `transferred_total ≥ length_total > 0` (ili fork `progress_percent == 100`) — stanje `Installing` dok RPI ne javi završetak; za igre `is_exists` potvrđuje instalaciju (polling 2 s, max 10 min) → `Done`. Za update/DLC `Done` čim RPI javi završetak. Tačan signal završetka: **[PROVJERA]**.
8. Server deregistruje stavku 60 s nakon `Done` (BGFT ponekad ponovi zadnji Range, **[PROVJERA]**), odmah nakon `Cancelled/Failed`.
9. Sljedeća stavka. Dok je bilo koja stavka u prenosu, PSPP drži `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)` da računar ne ode u mirovanje (otpušta se kad red stane).

Pauza: `POST /api/pause_task {"task_id":N}` → `Paused`; "Nastavi" → `resume_task` → `Transferring`. Otkaži: `stop_task` pa `unregister_task` → `Cancelled`; greška u otkazivanju se loguje, stavka ipak postaje `Cancelled` uz upozorenje "konzola možda i dalje ima zadatak u listi preuzimanja". "Zaustavi nakon trenutnog" ne prekida aktivnu stavku. Semantika `stop_task` vs `unregister_task`: **[PROVJERA]**.

### 6.3 Reinstall

Pokreće se samo kroz odluku "Reinstaliraj" (dijalog 4.7, sa ili bez "primijeni na sve"). Tok: 6.2 korak 4 → 5. Ako uninstall ne uspije, stavka je `Failed` (RPI-08) i ništa se ne instalira. Reinstall se nikad ne izvršava bez odluke korisnika, uključujući "primijeni na sve" koje važi samo za stavke koje su u trenutku odluke u redu (ne za kasnije dodane).

### 6.4 Greška

1. Engine klasifikuje grešku u kod iz kataloga (sekcija 10) i pravi `ErrorReport { code, what, why, action, technical, console_log }`.
2. Za greške u kojima je konzola učestvovala (RPI-01, -04, -05, -06, -08) engine povlači `/api/log` (timeout 5 s, zadnjih 200 linija) i prilaže ga. Neuspjeh povlačenja je samo napomena u izvještaju.
3. Log unos nivoa ERROR, brojač na tabu Log, status reda "Greška: <what>".
4. Dijalog greške se **ne** otvara automatski dok red radi (ne prekida rad); otvara se klikom/dvoklikom ili iz Loga. Izuzetak: globalne greške (port zauzet, konzola nestala pri pokretanju reda) — banner + zaustavljen red.

### 6.5 Test veze

Pokreće se dugmetom (`F5` u Postavkama), na pozadinskoj niti; dugme prikazuje spinner i ne može se pokrenuti dvaput. Koraci (svaki daje jednu liniju rezultata):

| # | Provjera | Uspjeh | Neuspjeh → poruka |
|---|---|---|---|
| 1 | IP validan IPv4 | — | CFG-01 |
| 2 | TCP connect `IP:port`, timeout 3 s | "Konzola dostupna (X ms)" | timeout → NET-01; refused → NET-02 |
| 3 | `POST /api/is_exists {"title_id":"CUSA00000"}` | validan RPI JSON (`status` polje) | 403 → NET-03; ne-JSON → NET-04; 404 → NET-05 |
| 4 | `/api/log` | "RPI 3.57 ili noviji" | 404 → upozorenje "Stariji RPI — nema konzolinog loga; preporučen 3.58" (ne greška) |
| 5 | PC IP preko UDP "connect" trika (9.6) | "Ovaj računar: 192.168.1.10" | NET-06 |
| 6 | Loopback `HEAD http://<PC_IP>:<port>/healthz` na vlastiti server | "Lokalni server radi" | SRV-01/02 |
| 7 | Info | "Firewall se potvrđuje pri prvom prenosu" | — |

Indikator konzole u zaglavlju se ažurira rezultatom koraka 2–3.

### 6.6 Prvo pokretanje

1. `%APPDATA%\PS Package Pusher\` ne postoji → kreira se; `config.json` sa defaultima (jezik `en`, port 12801, server 8095, splash uključen, `firewall_notice_shown=false`).
2. Splash (koraci 1–2; korak 3 se preskače jer konzola nije podešena i firewall dijalog još nije prikazan).
3. Glavni prozor: prazan red (4.2) + banner "Konzola nije podešena — [Otvori Postavke]".
4. Firewall dijalog (4.9), sa opcijom "Dozvoli u firewallu" (9.8).
5. Nakon što korisnik unese IP i sačuva (ili klikne Test veze): određuje se PC IP, server se binduje → Windows prikazuje svoj firewall prompt.
6. Ako PC ima jedan LAN IP a konzola još nije podešena, server se ne binduje dok se IP konzole ne unese (PC IP zavisi od rute ka konzoli).

### 6.7 Deinstalacija (samostalna)

Ulazi: kontekstni meni stavke "Deinstaliraj sa konzole…" ili `[⋯]` → "Deinstaliraj po ID…" (4.12). Komanda engine-u: `Uninstall { target: Item(id) | Manual { kind, id } }`.

1. **Dozvoljeno** za stavke u stanjima `Ready, Done, Skipped, Failed, Cancelled` (ne `Invalid`, ne dok je stavka u redu ili aktivna). Ne izvršava se za naslov koji se upravo instalira (aktivna stavka sa istim Title ID-om) — dugme onemogućeno uz tooltip.
2. **Provjera**: `is_exists` za ciljni naslov. Ne postoji → info "Nije instalirano na konzoli", kraj. Greška → RPI-02.
3. **Potvrda** dijalogom 4.11 (uvijek, bez izuzetka).
4. **Poziv** (sve `POST`, JSON):

| Tip | Endpoint | Tijelo |
|---|---|---|
| Igra (`gd`) | `/api/uninstall_game` | `{"title_id":"CUSAxxxxx"}` |
| Update (`gp`) | `/api/uninstall_patch` | `{"title_id":"CUSAxxxxx"}` |
| DLC (`ac`) | `/api/uninstall_ac` | `{"content_id":"…"}` |

   Imena polja u tijelu za `uninstall_patch`/`uninstall_ac`: **[PROVJERA]**. Teme (`uninstall_theme`) nisu podržane u v1.
5. **Rezultat**: 200/success → za igru ponovni `is_exists` mora vratiti "ne postoji" (max 3 pokušaja, razmak 2 s); stavka → `Ready` sa info "Nije instalirano"; log INFO "Deinstalirano: <naziv> (<ID>)". 404 sa RPI JSON tijelom → info "Već nije bilo instalirano". Ostalo → RPI-08 (stavka se vraća u prethodno stanje, dijalog greške dostupan) + preuzimanje `/api/log`.
6. Ako je red pokrenut, deinstalacija se izvršava između dva ticka engine-a (RPI 3.57+ serijalizuje BGFT/AppInstUtil pozive); aktivni prenos se ne prekida. **[PROVJERA]** da istovremena deinstalacija i prenos drugog naslova ne smetaju jedno drugom.

---

## 7. Tehnička arhitektura

### 7.1 Odluke

| Tema | Odluka | Razlog |
|---|---|---|
| Struktura | jedan Cargo paket: `lib` + dva binary-ja (`ps-package-pusher`, `mock-rpi`) | dovoljno za ovu veličinu; mock dijeli tipove sa klijentom; workspace nije potreban |
| Target | samo `x86_64-pc-windows-msvc`, `+crt-static` | `usize` = 64 bita (Range/Content-Length > 4 GB bez posebnog koda), nema zavisnosti od VC++ runtime-a |
| Async | **nema** tokio/async; std niti + `std::sync::mpsc` | malo istovremenih operacija; jednostavnije, manji binary |
| HTTP klijent | `ureq` (blocking), proxy isključen, eksplicitni timeouti | minimalan, bez async runtime-a |
| HTTP server | `tiny_http` (sync) + vlastita Range logika | mali, zreo; koristi ga i mock. Rizik: kontrola socket timeouta **[PROVJERA]**; rezerva je vlastiti mini server nad `std::net` (samo GET/HEAD) |
| UI | `eframe`/`egui` (glow renderer, AccessKit uključen), eframe persistence isključen | prozor/stanje se čuvaju u našem `config.json` |
| Dijalozi | `rfd` | native Windows file/folder picker |
| Slike | `image` sa samo `png` featureom | dekodiranje `icon0.png` |
| Serijalizacija | `serde`, `serde_json` | config, queue, RPI API |
| Ikone | `egui-phosphor` | icon font, MIT |
| Windows API | `windows-sys` (feature-i samo za: `SetThreadExecutionState`, `SystemParametersInfoW`, `GetLocalTime`, `ShellExecuteExW` + `WaitForSingleObject`/`GetExitCodeProcess`) | sprječavanje sleepa, reduce motion, lokalno vrijeme za log, elevirani firewall korak (9.8) |
| Resursi exe-a | `winresource` (build-dependency) | ikona i VERSIONINFO u `.exe` |
| Logging | vlastiti modul (bez `log`/`tracing`) | treba nam strukturisan unos za UI + fajl; ~150 linija |
| i18n | vlastiti modul, JSON tabele ugrađene `include_str!` | dva jezika; bez fluent/gettext |
| Otvaranje foldera | `std::process::Command::new("explorer")` | bez dodatnog crate-a |
| Clipboard | `egui::Context::copy_text` | ugrađeno |

Tačne verzije crate-ova se pinuju u `Cargo.toml`/`Cargo.lock` pri izradi (zadnje stabilne u tom trenutku).

### 7.2 Moduli

```
src/
  main.rs              # #![windows_subsystem = "windows"]; NativeOptions; pokreće App; `--firewall add|remove` → firewall.rs bez UI-ja (9.8)
  lib.rs
  app/                 # UI sloj (samo UI nit)
    mod.rs             # App: eframe::App, ruter tabova, primanje događaja engine-a
    splash.rs          # splash scena (painter), koraci, preobražaj prozora
    queue_view.rs      # lista, toolbar, drop zona, drag-over, reorder
    settings_view.rs
    log_view.rs
    dialogs.rs         # Već instalirano, greška, firewall, potvrde
    widgets.rs         # dugmad, badge, progress bar, status chip, fokus prsten
    theme.rs           # Palette (dark/light), tokeni, lerp, primjena na egui::Style
    anim.rs            # Anim (vrijeme, easing), smoothing progressa, reduce-motion
    fonts.rs
  engine/              # radna nit: vlasnik stanja reda
    mod.rs             # petlja: recv_timeout(1 s) → komande, tick progressa
    queue.rs           # Item, ItemState, prelazi (čista logika, testabilna bez mreže)
    commands.rs        # Command (UI→engine), Event (engine→UI)
  pkg/
    header.rs          # PS4 PKG zaglavlje + tabela entry-ja
    sfo.rs             # param.sfo
    scan.rs            # ekspanzija foldera, split grupisanje, duplikati
  rpi/
    client.rs          # endpointi, timeouti, klasifikacija odgovora
    types.rs           # tolerantni JSON tipovi (broj | "123" | "0x7B")
  server/
    mod.rs             # bind, accept, limit konekcija, IP filter
    range.rs           # parsiranje Range zaglavlja (čista funkcija, testovi)
    registry.rs        # whitelist (item_id, ime) → putanje
  net.rs               # PC IP detekcija, validacija IPv4, ista podmreža
  config.rs            # Config, učitavanje/snimanje, migracija sheme
  storage.rs           # %APPDATA% putanje, atomski upis
  logging.rs           # unosi, fajl, rotacija, prsten za UI, anonimizacija putanja
  errors.rs            # katalog grešaka (kod → ključevi poruka + akcija), BGFT/CE tabela
  i18n.rs              # t(key), t_args(key, args), plural za bs
  winapi.rs            # tanki omotači oko windows-sys poziva
  firewall.rs          # stanje pravila (čitanje), elevirani add/remove preko `--firewall` (9.8)
  mock/                # lažni RPI (koristi ga bin/mock_rpi.rs i integracioni testovi)
    mod.rs
src/bin/mock_rpi.rs
lang/en.json
lang/bs.json
assets/fonts/ (Inter-Regular.ttf, Inter-SemiBold.ttf, Audiowide-Regular.ttf, OFL.txt)
assets/icon/app.ico, assets/logo/logo.svg, assets/logo/logo-512.png
tests/ (integracioni testovi sa mock serverom)
```

### 7.3 Model niti

```
 UI nit (eframe)  ── Command (mpsc) ──►  Engine nit ──► RPI pozivi (ureq, blocking, kratki timeouti)
      ▲                                     │  ▲
      │ Event (mpsc) + ctx.request_repaint  │  │ ParseResult (mpsc)
      └─────────────────────────────────────┘  │
                                  Parse pool (≤ 4 niti, po potrebi)
 Server: accept nit ─► nit po konekciji (≤ 32 istovremeno) ─► ServerEvent (mpsc → engine)
 Startup/Test veze: kratkotrajne niti, rezultat kao Event
 Logger: Mutex<BufWriter<File>> + prsten; poziva se iz svih niti
```

- **UI nit nikad ne blokira**: ne radi I/O, mrežu, parsiranje ni dekodiranje slika. Na početku svakog frame-a prazni `Event` kanal (`try_recv` petlja) i ažurira svoj view-model (kopija stanja stavki). Dekodirana ikona dolazi kao `ColorImage` u eventu; UI samo pravi teksturu (`ctx.load_texture`).
- **Engine** je jedini vlasnik stanja reda i jedini koji piše `queue.json`. Petlja: `recv_timeout(1 s)` → obradi komandu ili tick (polling progressa aktivne stavke, stall detekcija). RPI pozivi imaju timeout connect 2 s / read 5 s, pa komanda (npr. Pauziraj) čeka najviše ~7 s u najgorem slučaju; UI odmah prikazuje "Pauziram…" (optimistički pending status na dugmetu).
- Svaki `Event` koji mijenja prikaz prati `ctx.request_repaint()`.
- Komande: `AddPaths`, `Remove(id)`, `Move(id, to)`, `StartQueue`, `StopAfterCurrent`, `Pause(id)`, `Resume(id)`, `Cancel(id)`, `Decide(id, Skip|Reinstall, apply_all)`, `Retry(id)`, `CheckInstalled(kind, id)`, `Uninstall(target)`, `TestConnection`, `ApplySettings(Config)`, `ClearFinished`, `Shutdown`.
- Događaji: `ItemUpsert(ItemView)`, `ItemRemoved(id)`, `Order(Vec<id>)`, `AskDecision(id)`, `Log(LogEntry)`, `ConsoleStatus`, `ServerStatus`, `TestResult(Vec<CheckLine>)`, `InstalledResult(kind, id, bool)`, `AskUninstallConfirm(target)`, `UninstallResult(target, Result)`, `SplashStep(n, key)`, `Banner(Option<BannerKind>)`.

### 7.4 Gdje se šta čuva

Sve u `%APPDATA%\PS Package Pusher\` (`std::env::var("APPDATA")`):

```
config.json
queue.json
logs\pspp-YYYY-MM-DD.log        # dnevni fajl; čuva se zadnjih 14 fajlova, max 10 MB po fajlu
```

Upis je atomski: `*.tmp` → `fsync` → `rename`. Neispravan JSON pri čitanju → original se preimenuje u `*.broken-<timestamp>.json`, koriste se defaulti (config) ili prazan red (queue), i loguje se upozorenje (CFG-02).

Ikone se ne keširaju na disku: pri učitavanju reda čita se samo icon0 entry iz pkg-a (nekoliko desetina KB).

### 7.5 Format `config.json`

```json
{
  "schema": 1,
  "console_ip": "192.168.1.50",
  "console_port": 12801,
  "server_port": 8095,
  "pc_ip_override": null,
  "language": "en",
  "splash_enabled": true,
  "firewall_notice_shown": true,
  "window": { "x": 120, "y": 80, "w": 1100, "h": 720, "maximized": false }
}
```

- `console_ip`: `null` ili IPv4 string. Pri učitavanju se validira; neispravno → `null` + upozorenje.
- `schema` omogućava migracije; nepoznata polja se ignorišu, nedostajuća dobijaju default (`#[serde(default)]`).

### 7.6 Format `queue.json`

```json
{
  "schema": 1,
  "items": [
    {
      "id": "9f2c41d07ab3e5c1",
      "parts": [
        { "path": "D:\\PKG\\Igra_0.pkg", "size": 4294967296, "mtime": 1763550000 },
        { "path": "D:\\PKG\\Igra_1.pkg", "size": 1234567890, "mtime": 1763550000 }
      ],
      "content_id": "UP0000-CUSA12345_00-0000000000000000",
      "title_id": "CUSA12345",
      "title": "Neki Naslov Igre",
      "version": "01.00",
      "category": "gd",
      "state": "ready",
      "decision": null,
      "task_id": null,
      "error": null,
      "finished_at": null
    }
  ]
}
```

- `id`: 64-bitni nasumičan hex (iz `std::collections::hash_map::RandomState` + vrijeme + brojač; ne treba kriptografska jačina jer server ima i IP filter).
- `state`: `ready | invalid | done | failed | cancelled | skipped | interrupted` (5.1).
- `decision`: `null | "skip" | "reinstall"`.
- `error`: `{ "code": "RPI-04", "detail": "…" }`.
- Redoslijed niza = redoslijed u redu čekanja.

### 7.7 i18n

- `lang/en.json` i `lang/bs.json`: ravna mapa `"queue.add_files": "Add files"`. Argumenti `{name}`; plural ključevi sa sufiksima `.one`, `.few`, `.other` (bs pravilo: n%10==1 && n%100!=11 → one; n%10 u 2..4 && n%100 nije 12..14 → few; inače other; en: one/other).
- Nedostajući ključ u `bs` → `en` vrijednost; nedostajući u `en` → sam ključ (vidljivo u razvoju).
- Unit test: obje datoteke imaju isti skup ključeva i iste `{argumente}`.
- Default jezik je uvijek engleski (ne prati jezik Windowsa); izbor se pamti.
- Formatiranje brojeva: `en` → `61.3 GB`, `bs` → `61,3 GB`. Veličine po Windows Explorer konvenciji (1024-bazne, oznake KB/MB/GB), da se poklapaju sa Explorerom. Brzina u MB/s istom bazom.

### 7.8 Logging

- Format linije u fajlu: `2026-11-19 14:09:03.512 ERROR [engine] [CUSA54321] RPI-04 msg="…" task_id=8 http=200 body={…}` (lokalno vrijeme, UTF-8).
- Nivoi: ERROR, WARN, INFO, DEBUG (DEBUG u fajl samo ako je pokrenuto sa `--debug`).
- Anonimizacija: putanja korisničkog profila (`%USERPROFILE%`) se pri upisu zamjenjuje stringom `%USERPROFILE%`, da kopirani log za GitHub issue ne otkriva korisničko ime. Ostale putanje ostaju (potrebne za dijagnozu).
- RPI tijela odgovora se loguju skraćena na 2 KB.
- Konzolin `/api/log` se ne piše u glavni log nego u `logs\console-<task_id>-<vrijeme>.txt` i referencira iz unosa.

---

## 8. Mock RPI server (`mock-rpi.exe`)

Zaseban binary, ne ide u release zip. Uloga: razvoj bez konzole, ručno testiranje UI-ja i automatski integracioni testovi u CI.

### 8.1 Ponašanje (oponaša RPI 3.58)

| Pravilo | Mock ponašanje |
|---|---|
| Port | default `127.0.0.1:12801`; `--bind <ip:port>` |
| Host zaglavlje | ako `Host` nije IP literal → 403 `{"status":"fail","error":"forbidden"}` |
| Origin / Sec-Fetch-Site | na destruktivnim endpointima (`uninstall_*`, `stop_task`, `unregister_task`, `exit`, `install`) prisustvo `Origin` ili `Sec-Fetch-Site: cross-site` → 403 **[PROVJERA]** tačna lista endpointa |
| Metode | `/api/uninstall_*` samo POST; GET → 405 **[PROVJERA]** stvarni status (405 ili 404) |
| Nepoznat endpoint | 404 bez JSON tijela (prazno ili `text/plain`) **[PROVJERA]** |
| Uninstall nepostojećeg | 404 sa `{"status":"fail","error":"not installed"}` **[PROVJERA]** |
| `install` validacija | URL mora biti `http://`, IP host, bez query stringa, put ASCII; inače `fail` |
| Preuzimanje | po taskovima: za svaki URL HEAD (opciono) pa GET sa `Range: bytes=a-b` u komadima (default 8 MiB), sekvencijalno po dijelovima; do 32 istovremene konekcije ukupno |
| 30 s pravilo | ako PC server ne pošalje nijedan bajt 30 s → task `error` (stall kod) |
| Progres | `get_task_progress` vraća i flatz polja i fork polja; brojevi u formatu izabranom flagom (`--num-format dec|hexstr|number`) da bi se testirala tolerancija parsera |
| `is_exists` | iz in-memory skupa instaliranih (`--installed CUSA00001,CUSA00002`); install `gd` dodaje Title ID u skup po završetku |
| `/api/log` | vraća zadnje linije vlastitog loga mocka |
| Pause/resume/stop/unregister | mijenja stanje taska; pauza zaustavlja povlačenje |

### 8.2 Scenariji (flagovi)

`--speed 40MB` (limit), `--latency 50ms`, `--fail-install "<poruka>"`, `--fail-task-at 30% --error-code 0x80990XXX`, `--stall-after 100MB` (mock prestane da povlači, testira detekciju stalla u PSPP-u), `--no-fetch` (prihvati install ali nikad ne povuče → testira FW-01), `--legacy` (bez `/api/log`, bez fork polja → testira degradaciju), `--reject-all-403`.

### 8.3 Upotreba u testiranju

- **Unit testovi** (bez mreže): `pkg` parser nad sintetičkim fixture-ima (generator u `tests/support` pravi minimalan validan pkg: zaglavlje + entry tabela + param.sfo + mali PNG; plus namjerno oštećene varijante), `server::range`, `engine::queue` prelazi, `i18n` paritet, `rpi::types` tolerantno parsiranje, `net` validacija IP-a.
- **Integracioni testovi** (`tests/`): mock se pokreće in-process na `127.0.0.1:0`, PSPP engine + server na loopbacku (IP filter dozvoljava loopback u test konfiguraciji). Scenariji: uspješna instalacija; split (3 dijela); > 4 GB preko sparse fajla (Range offseti iznad `u32::MAX`, bez stvarnog pisanja 4 GB); već instalirano + reinstall; update bez baze; stall; fail sa kodom; pauza/nastavak; otkaz; restart engine-a sa `interrupted` stavkom.
- Nikakvi stvarni (komercijalni) pkg fajlovi nisu u repou ni u testovima.

---

## 9. Detalji komponenti

### 9.1 RPI klijent

- Base URL: `http://<console_ip>:<console_port>`; `Host` je IP; ureq ne šalje `Origin`/`Sec-Fetch-*`; proxy iz okruženja eksplicitno isključen.
- Svi pozivi `POST`, `Content-Type: application/json`. Za `/api/log` metoda **[PROVJERA]** (pokušaj POST, pa GET na 404/405; rezultat se kešira po sesiji).
- Timeouti: connect 2 s; read 5 s (install 10 s; log 5 s).
- Klasifikacija odgovora (redom):
  1. greška konekcije → `Unreachable(timeout|refused|other)`;
  2. 403 → `Forbidden`;
  3. 404 + tijelo je JSON sa `status` → `RpiFail{status:404, error}` (npr. "nije instalirano");
  4. 404 bez JSON-a → `EndpointMissing`;
  5. 2xx + JSON `status:"success"` → OK;
  6. JSON `status:"fail"` → `RpiFail{error}`;
  7. sve ostalo → `BadResponse(status, body_excerpt)`.
- Tolerantni brojevi: polje može biti JSON broj, decimalni string ili `"0x…"` hex string (flatz je koristio hex stringove za neke vrijednosti, **[PROVJERA]** za 3.58). `error` može biti broj ili string.
- `is_exists` odgovor: prihvata `exists` kao bool ili `"true"/"false"`; opciono `size`, `version` ako postoje. **[PROVJERA]** tačna polja.

### 9.2 Progress, brzina, ETA

- Izvor: `get_task_progress` svake 1 s. Ukupno: `length_total` (ili fork ukupna veličina), preneseno: `transferred_total` (ili `transferred_bytes`); fork `progress_percent` ima prednost za prikaz procenta ako postoji. Ako RPI ne da ukupnu veličinu, koristi se zbir veličina dijelova.
- Brzina: EMA nad 1-sekundnim uzorcima (Δtransferred/Δt), α = 0.2. Prikazuje se nakon 3 uzorka.
- ETA: ako `rest_sec_total` (ili `rest_sec`) postoji i > 0 → EMA (α = 0.3) nad njim; inače preostali bajtovi / izglađena brzina. Prikaz: "—" prvih 3 s; "~45 s", "~7 min", "~1 h 05 min"; promjena prikazane vrijednosti najviše jednom u 2 s da se ne trese.
- Server-side brojač poslanih bajtova se ne prikazuje kao progres (Range ponavljanja ga kvare), služi samo za FW-01 i dijagnostiku stalla ("konzola nije tražila podatke 25 s").

### 9.3 Nastavak nakon restarta

Za `interrupted` stavku sa `task_id`: "Pokušaj ponovo" → `get_task_progress(task_id)`. Ako task postoji i nije završen → `resume_task` (server je ponovo gore, URL-ovi isti jer `item_id` i ime ostaju isti) → `Transferring`. Ako task ne postoji ili vraća grešku → `unregister_task` (ignoriše grešku) i puni tok od 6.2 korak 3. **[PROVJERA]** da li RPI zadržava task nakon prekida i da li `resume_task` nastavlja od offseta.

### 9.4 HTTP file server

- Bind na **PC LAN IP** (ne `0.0.0.0`), port iz postavki (default 8095). Port zauzet → SRV-01 (prijedlog: sljedeći slobodan port u rasponu +1…+20, korisnik potvrđuje u Postavkama).
- Prihvata samo zahtjeve sa izvorne IP adrese = `console_ip` (i `127.0.0.1` za self-test/mock). Ostali → zatvaranje konekcije bez odgovora (log DEBUG). Napomena: `tiny_http` 0.12 pri drop-u neodgovorenog `Request`-a automatski šalje 500 i ne daje pristup socketu, pa "bez odgovora" traži vlastiti server; ako ostane `tiny_http`, odgovor je 403 sa praznim tijelom (odluka prototipom 14.3/23).
- Rute: `GET|HEAD /pkg/<item_id>/<safe-name>` i `HEAD /healthz` (samo loopback). Sve ostalo → 404 prazno tijelo. Metode osim GET/HEAD → 405.
- `<safe-name>`: Content ID (ASCII po formatu) + za split `_<n>`, + `.pkg`, filtrirano na `[A-Za-z0-9._-]`; npr. `/pkg/9f2c41d07ab3e5c1/UP0000-CUSA12345_00-0000000000000000_1.pkg`. Bez query stringa. Lokalno ime fajla i putanja se nikad ne pojavljuju u URL-u ni u odgovoru.
- Lookup je tačno poklapanje `(item_id, safe-name)` u registry mapi → apsolutna putanja; dijelovi URL-a se nikad ne spajaju sa putanjom fajl-sistema (nema path traversala po konstrukciji).
- Range: podržano `bytes=a-b`, `bytes=a-`, `bytes=-n`; više opsega u jednom zahtjevu → 416 (BGFT koristi jedan opseg, **[PROVJERA]**). Neispravan/izvan veličine → 416 sa `Content-Range: bytes */<size>`. Bez Range → 200 cijeli fajl. Zaglavlja: `Accept-Ranges: bytes`, `Content-Length` (u64), `Content-Range`, `Content-Type: application/octet-stream`, bez `Server` verzije.
- Čitanje: `File` + `seek` + `take(len)`, bafer 1 MiB. Ako jedno čitanje sa diska traje > 10 s → upozorenje SRV-04 (prijeti RPI-ov 30 s prekid).
- Limit 32 istovremene konekcije (atomski brojač); preko limita → 503. Keep-alive dozvoljen.
- Registry: stavka se registruje pri Starting, deregistruje kako je opisano u 6.2 korak 8; fajl se otvara po zahtjevu (ne drži se lock između zahtjeva). Promjena veličine/mtime fajla tokom prenosa → zatvaranje konekcije + SRV-03.

### 9.5 PKG parser

PS4 PKG zaglavlje je big-endian. Korištena polja (offseti relativno na početak prvog dijela):

| Offset | Tip | Polje |
|---|---|---|
| 0x00 | u32 | magic `0x7F434E54` ("\x7FCNT") |
| 0x10 | u32 | `entry_count` (0x0C je `file_count`) — potvrđeno desk-provjerom |
| 0x18 | u32 | offset tabele entry-ja (uvijek čitati, ne pretpostavljati 0x1000) |
| 0x40 | 36 B ASCII | Content ID (`XXYYYY-CUSAxxxxx_00-ZZZZZZZZZZZZZZZZ`) |
| 0x430 | u64 | `pkg_size` — ukupna veličina paketa; koristi se za validaciju split grupe (**[PROVJERA]** na stvarnom split paketu da `_0` nosi ukupnu veličinu svih dijelova) |

Entry tabela: `entry_count` × 32 B: `id u32, filename_offset u32, flags1 u32, flags2 u32, data_offset u32, data_size u32, pad u64`. Traže se `id 0x1000` (param.sfo) i `id 0x1200` (icon0.png). Ako je bit enkripcije `flags1 & 0x80000000` postavljen → entry se tretira kao nedostupan (potvrđeno).

Izvori desk-provjere (PLAN sekcija 3): orbis-pkg `header.rs`, ps4-pkg-info `lib.rs`, LibOrbisPkg `Entry.cs`/`PkgReader.cs`, playstationdev.wiki SFO. psdevwiki.com nije bio dostupan za automatsko čitanje.

param.sfo (little-endian): magic `"\0PSF"`, `key_table_start u32 @0x08`, `data_table_start u32 @0x0C`, `count u32 @0x10`; index po 16 B: `key_offset u16, fmt u16 (0x0204 utf8, 0x0004 utf8-special, 0x0404 u32), len u32, max_len u32, data_offset u32`. Čitaju se: `TITLE`, `TITLE_ID`, `CONTENT_ID`, `CATEGORY`, `APP_VER`, `VERSION`.

Mapiranje tipa (`CATEGORY`): `gd` → IGRA, `gp` → UPDATE, `ac` → DLC; ostalo (npr. `gde`, `gdk`, teme) → prikaz sirove vrijednosti u sivom badge-u, instalacija dozvoljena, provjera instaliranosti po Title ID-u kao za igru uz upozorenje **[PROVJERA]**. Verzija za prikaz: `APP_VER` za `gd`/`gp`, `VERSION` za `ac` i kao rezerva **[PROVJERA]**.

Robusnost: svi offseti i dužine se provjeravaju prema veličini fajla prije čitanja; limiti: param.sfo ≤ 64 KiB, icon0 ≤ 4 MiB, broj entry-ja ≤ 10 000, PNG dimenzije ≤ 1024×1024 (veće se odbijaju, ne skaliraju). Čita se samo ono što treba (`seek` + `read_exact`), nikad cijeli fajl. Parser ne smije panikovati ni na kakvom ulazu (unit testovi sa skraćenim/nasumičnim bajtovima). Neslaganje Content ID-a iz zaglavlja i iz param.sfo → upozorenje, koristi se zaglavlje.

Validacija split grupe: zbir veličina dijelova se poredi sa poljem ukupne veličine paketa u zaglavlju, ako je polje pouzdano lokalizovano **[PROVJERA]** (offset polja `pkg_size`); do potvrde samo provjera da tabela entry-ja i param.sfo leže unutar `_0`.

### 9.6 Detekcija PC IP-a

- `UdpSocket::bind("0.0.0.0:0")` → `connect(console_ip:console_port)` (UDP connect ne šalje pakete) → `local_addr()` = IP interfejsa koji ruta koristi prema konzoli. Bez dodatnih crate-ova.
- Rezultat `0.0.0.0` ili loopback → NET-06. Ručni override u Postavkama ima prednost.
- Ponovno određivanje pri startu, pri promjeni IP-a konzole i kad bind/konekcija padne (npr. DHCP promijenio IP) → server se restartuje na novom IP-u; aktivna stavka ne može preživjeti promjenu IP-a (URL-ovi sadrže stari IP) → `Failed` (SRV-02) sa "Pokušaj ponovo".
- Ako PC IP i konzola nisu u istoj /24 (heuristika, maska nije dostupna iz std) → upozorenje u testu veze, ne blokada.

### 9.7 Praćenje Windows teme

- `egui::ThemePreference::System`; eframe prima `WindowEvent::ThemeChanged` od winit-a i ažurira `ctx.system_theme()`. App svaki frame poredi temu sa prethodnom i pokreće `motion.theme` pretapanje (bez pollinga registra).
- **[PROVJERA]** da eframe na Windows 11 pouzdano isporučuje promjenu teme dok aplikacija radi; rezerva: čitanje `HKCU\...\Themes\Personalize\AppsUseLightTheme` pri `WM_SETTINGCHANGE`/fokusu prozora.
- Naslovna traka prozora (DWM) prati temu kroz winit; ako ne → **[PROVJERA]**, rezerva `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)`.

### 9.8 Windows Firewall pravilo

Pozadina: kad aplikacija prvi put otvori port na LAN IP-u, Windows sam prikaže "Windows Security Alert". "Allow access" pravi široko pravilo za cijeli exe (svi portovi, TCP+UDP); "Cancel" pravi **block** pravilo za exe, nakon čega Windows više ne pita. Pravilo je vezano za putanju exe-a, pa premještanje exe-a ponovo izaziva prompt. Opcija 9.8 daje uže pravilo i rješava slučaj kad je korisnik ranije kliknuo "Cancel".

**Pravilo** (ime je fiksno: `PS Package Pusher`):

```
netsh advfirewall firewall add rule name="PS Package Pusher" dir=in action=allow
      protocol=TCP localport=<port> profile=private remoteip=localsubnet program="<putanja exe-a>"
```

- Dodavanje: aplikacija pokreće **samu sebe** elevirano (`ShellExecuteExW`, verb `runas`) sa `--firewall add <port>`; UAC prozor pokazuje "PS Package Pusher". Elevirani proces nema UI: izvede `netsh … delete rule name="PS Package Pusher"` (briše staro, ignoriše "nema pravila") pa `add rule …` preko `std::process::Command` (bez `cmd.exe`, bez interpolacije stringova u shell), vrati exit kod i završi. Glavni proces čeka (pozadinska nit) i osvježava stanje.
- Uklanjanje: `--firewall remove` → samo `delete rule`.
- Promjena porta lokalnog servera dok pravilo postoji → red u Postavkama nudi "Ažuriraj pravilo" (isti postupak kao dodavanje).
- **Čitanje stanja** (bez admin prava, pozadinska nit, timeout 10 s): `powershell -NoProfile -NonInteractive -Command` sa `Get-NetFirewallRule -DisplayName 'PS Package Pusher'` + `Get-NetFirewallPortFilter` + `Get-NetFirewallApplicationFilter` → `ConvertTo-Json` (nezavisno od jezika Windowsa, za razliku od `netsh show`). Također `Get-NetFirewallProfile -PolicyStore ActiveStore` → `AllowLocalFirewallRules`; ako je `False` (politika firme), lokalna pravila nemaju efekta i UI to kaže umjesto dugmeta. **[PROVJERA]** trajanje i pouzdanost čitanja (spike S-5).
- Ne mijenja se ništa drugo (profili, default akcije, mrežna kategorija). Ako je mreža "Public", UI to samo napomene sa uputom (4.9 korak 2).

**Ishodi:**

| Ishod | Prikaz |
|---|---|
| exit 0 | "Pravilo aktivno (TCP <port>, Private)" + log INFO |
| UAC odbijen (`ERROR_CANCELLED` 1223 iz `ShellExecuteExW`) | info "Nije promijenjeno — potvrda je odbijena" (FW-02), bez dijaloga greške |
| `netsh` exit ≠ 0 | FW-03 sa izlazom `netsh`-a u tehničkim detaljima |
| lokalna pravila blokirana politikom | FW-04, bez dugmeta |

---

## 10. Katalog grešaka

Svaka greška ima stabilan kod (koristi se u logu, i18n ključevima i README troubleshooting sekciji). Poruke su ovdje sažete; puni tekstovi su u `lang/*.json` pod `error.<KOD>.what|why|action`.

| Kod | Kategorija | Kada | Poruka za korisnika (šta / zašto) | Preporučena akcija |
|---|---|---|---|---|
| CFG-01 | Postavke | IP nije validan IPv4 | Unesena vrijednost nije IPv4 adresa; RPI 3.58 odbija imena hostova | Unesi IP sa ekrana konzole (npr. 192.168.1.50) |
| CFG-03 | Unos | Title ID / Content ID ne odgovara formatu (4.12) | Neispravan ID | Title ID: 4 slova + 5 cifara (CUSA12345); Content ID kao na paketu |
| CFG-02 | Postavke | `config.json`/`queue.json` neispravan | Fajl je oštećen; sačuvan kao `.broken`, učitani defaulti | Ponovo unesi postavke; prijavi bug sa fajlom |
| NET-01 | Mreža | connect timeout | Konzola ne odgovara na IP:port | Provjeri da je konzola upaljena, RPI pokrenut, IP tačan, ista mreža |
| NET-02 | Mreža | connection refused | Konzola je dostupna, ali na portu ne sluša RPI | Pokreni RPI; provjeri port (12801 za 3.58; stari flatz koristi 12800 — nije podržan u v1) |
| NET-03 | Mreža/RPI | HTTP 403 | RPI je odbio zahtjev (zaštita od DNS rebindinga / Origin provjera) | Koristi IP adresu; isključi proxy/VPN; prijavi bug sa logom ako se ponavlja |
| NET-04 | Mreža | odgovor nije RPI JSON | Na tom portu radi nešto drugo | Provjeri IP/port |
| NET-05 | RPI | 404 bez JSON tijela | Ova verzija RPI-ja nema očekivani endpoint | Instaliraj RPI 3.58 (fork rawblue33) |
| NET-06 | Mreža | PC IP nije određen | Računar nema rutu do konzole | Spoji računar na istu mrežu; ili unesi PC IP ručno |
| FW-01 | Firewall | API OK, a konzola ne zatraži fajl u 20 s | Konzola ne može doći do računara | Uputa 4.9 (Allow access, Private mreža, VPN, AP isolation) |
| FW-02 | Firewall (info) | UAC potvrda odbijena | Pravilo nije promijenjeno | Ponovo klikni i potvrdi, ili koristi Windowsov prompt (4.9) |
| FW-03 | Firewall | `netsh` nije uspio | Windows nije prihvatio pravilo: `<izlaz>` | Uputa 4.9 (ručno dozvoljavanje) |
| FW-04 | Firewall | `AllowLocalFirewallRules = False` | Firewall ovog računara vodi politika (npr. firma); lokalna pravila nemaju efekta | Obrati se administratoru ili koristi drugi računar |
| SRV-01 | Server | port zauzet | Drugi program koristi port N | Izaberi drugi port u Postavkama (ponuđen slobodan) |
| SRV-02 | Server | bind na PC IP nije uspio / IP promijenjen | IP ovog računara se promijenio | Test veze; pokušaj ponovo |
| SRV-03 | Server/fajl | fajl nestao, nedostupan ili promijenjen tokom slanja | Fajl je pomjeren, obrisan, disk odspojen | Vrati fajl/disk; ukloni i ponovo dodaj |
| SRV-04 | Server/fajl | čitanje sa diska > 10 s (upozorenje) | Disk je spor ili se budi; konzola prekida nakon 30 s čekanja | Kopiraj pkg na lokalni SSD/HDD |
| RPI-01 | RPI | `install` → `fail` | Konzola je odbila instalaciju: `<error>` | Zavisno od prevoda; vidi konzolin log |
| RPI-02 | RPI | `is_exists` greška | Provjera instaliranosti nije uspjela | Instalacija se ne pokreće bez provjere; pokušaj ponovo |
| RPI-04 | RPI/BGFT | `error ≠ 0` u progresu | Preuzimanje/instalacija prekinuta, kod `0x…` + prevod | Iz BGFT tabele |
| RPI-05 | Stall | nema napretka 60 s | Prenos je stao (konzola ili mreža) | Provjeri konzolu (rest mode, prostor); pokušaj ponovo |
| RPI-06 | Zavisnost | Update/DLC bez osnovne igre | Osnovna igra CUSAxxxxx nije instalirana | Dodaj i instaliraj igru prije updatea/DLC-a |
| RPI-07 | Prekid | konzola nedostupna tokom prenosa / app zatvorena | Veza sa konzolom je izgubljena | Provjeri konzolu; "Pokušaj ponovo" nastavlja ako je moguće |
| RPI-08 | RPI | uninstall nije uspio (≠ 200, ≠ 404-not-installed), u Reinstaliraj ili samostalno | Deinstalacija nije uspjela; kod Reinstaliraj ništa nije instalirano | Vidi konzolin log; deinstaliraj ručno na konzoli |
| PKG-01 | Paket | magic ≠ `0x7F434E54` | Fajl nije PS4 paket | Provjeri fajl |
| PKG-02 | Paket | offseti izvan fajla | Paket je oštećen ili nepotpuno preuzet | Ponovo napravi/kopiraj paket |
| PKG-03 | Paket | param.sfo nedostaje ili je kriptovan | Naziv, verzija i tip nedostupni. Ako Content ID daje validan Title ID, stavka je `Ready` (tip "?", upozorenje); inače `Invalid`, jer bez Title ID-a provjera instaliranosti nije moguća (odluka 14.4/29) | Provjeri paket |
| PKG-04 | Paket | split dio nedostaje | Nedostaju dijelovi `_2`, `_5` | Stavi sve dijelove u isti folder |
| PKG-05 | Paket (info) | duplikat | Paket je već u redu (#n) | — |
| PKG-06 | Paket (upozorenje) | nepoznata kategorija | Nepoznat tip paketa `<cat>` | Instalacija je moguća na vlastitu odgovornost |
| PKG-07 | Paket | fajl promijenjen/nestao između dodavanja i slanja | Fajl se promijenio | Ukloni i ponovo dodaj |

### 10.1 Prevod konzolinih kodova

- `errors.rs` sadrži tabelu `kod → (ključ objašnjenja, ključ akcije, verified: bool)`. Kodovi se izvlače iz RPI `error` polja i iz konzolinog loga regexima `CE-\d{5}-\d` i `0x8099[0-9A-Fa-f]{4}`.
- Nepoznat kod → generička poruka "Konzola je prijavila kod X; nije u katalogu" + sirovi kod + preporuka da se pretraži i prijavi (issue template traži kod i log).
- Početni sadržaj (svi **nepotvrđeni** dok se ne reprodukuju na konzoli, prikazuju se sa napomenom "moguće značenje"):

| Kod | Moguće značenje | Akcija |
|---|---|---|
| CE-30005-8 | greška čitanja podataka / oštećen paket ili disk | Provjeri integritet paketa; ponovo kopiraj |
| CE-36329-3 | sistemska greška; kod homebrew paketa često znači da jailbreak/patch za instalaciju nije aktivan | Ponovo pokreni jailbreak pa RPI |
| CE-32809-2 | aplikacija ne može da se pokrene (posljedica, ne greška instalacije) | Provjeri da je jailbreak aktivan |
| `0x8099xxxx` (porodica) | BGFT greška preuzimanja/instalacije | Prikaži kod; konkretni kodovi se dodaju nakon testiranja (npr. nedovoljno prostora, nedostupan URL, prekid) |

Tabela je proširiva bez promjene logike (dodavanje reda + i18n ključevi). Popunjavanje tačnih BGFT kodova je zadatak testiranja na konzoli.

---

## 11. Sigurnost i robusnost

- **Server**: servira isključivo registrovane stavke reda po tačnom `(item_id, safe-name)`; nema directory listinga, nema path traversala (URL se nikad ne pretvara u putanju), nema lokalnih imena/putanja u URL-u ni odgovorima; bind samo na LAN IP; whitelist izvorne IP adrese (konzola + loopback); samo GET/HEAD; limit 32 konekcije; deregistracija nakon završetka/otkaza; nema `Server` zaglavlja; greške vraćaju prazno tijelo.
- **Klijent**: samo IPv4 literal, samo HTTP ka konzoli, bez proxyja, bez redirect praćenja (ureq 3.x `max_redirects(0)`), limit veličine tijela odgovora (1 MB API, 2 MB log), JSON se tretira kao nepovjerljiv (nepoznata polja ignorisana, stringovi skraćeni pri prikazu, kontrolni znakovi uklonjeni).
- **Destruktivne akcije** (uninstall) samo nakon eksplicitne odluke korisnika u dijalogu; default je nedestruktivan.
- **Parser**: bounds-check svega, limiti veličina, bez panike; ikona dekodirana sa limitom dimenzija.
- **Podaci**: ništa se ne šalje nigdje osim na konzolu iz Postavki; nema telemetrije; log anonimizuje profil korisnika.
- **Stabilnost**: atomski upis fajlova; `panic` hook upisuje backtrace u log i prikazuje poruku umjesto tihog gašenja; `SetThreadExecutionState` sprječava sleep tokom prenosa; potvrda zatvaranja tokom prenosa.
- **Bez admin prava**: glavni proces ne traži elevaciju i ne mijenja sistemske postavke (mreža, registry osim čitanja teme ako zatreba). Jedini izuzetak je firewall pravilo (9.8): kratkotrajan elevirani proces, samo na klik korisnika, uvijek kroz UAC, sa fiksnim argumentima (port je jedini parametar i validira se kao broj 1024–65535).

---

## 12. Pristupačnost

- **Tastatura** — sve je dostupno bez miša:

| Prečica | Akcija |
|---|---|
| `Ctrl+1/2/3`, `Ctrl+Tab` | tabovi |
| `Ctrl+O` / `Ctrl+Shift+O` | Dodaj fajlove / Dodaj folder |
| `Ctrl+Enter` | Pokreni red |
| `↑/↓` | izbor reda u listi |
| `Alt+↑/↓`, `Alt+Home/End` | pomjeri stavku |
| `Delete` | ukloni stavku iz liste (aktivna → potvrda) |
| `Shift+Delete` | deinstaliraj selektovanu stavku sa konzole (uvijek potvrda 4.11) |
| `Space` | pauziraj/nastavi aktivnu stavku (kad je ona selektovana) |
| `Enter` | otvori detalje greške selektovane stavke |
| `F5` | Test veze (Postavke) |
| `Esc` | zatvori dijalog (= nedestruktivna opcija) |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | zoom UI-ja (egui `gui_zoom`) |

- **Fokus**: vidljiv prsten `focus.ring` (2 px + 2 px razmak) na svakom fokusiranom elementu; redoslijed Taba prati vizuelni redoslijed; fokus se pri otvaranju dijaloga prebacuje u dijalog i vraća na izvorni element pri zatvaranju.
- **Kontrast**: svi parovi tekst/pozadina ≥ 4.5:1 i UI elementi ≥ 3:1 (tabele 2.2); izuzetak su samo onemogućene kontrole (dozvoljeno WCAG-om), koje uz to imaju i promijenjen oblik (bez fill-a).
- **Ne samo boja**: status = ikona + tekst; badge tipa = tekst (IGRA/UPDATE/DLC); greška u polju = ikona + poruka ispod polja; progress uz bar ima i procenat kao tekst; selektovan red ima i lijevu traku, ne samo drugu pozadinu.
- **Čitači ekrana**: AccessKit (eframe) — svaki interaktivni widget ima labelu; ikonska dugmad imaju tekstualni `on_hover_text` i accessible name; promjena statusa aktivne stavke se ne najavljuje svake sekunde (samo promjena stanja). Nivo podrške AccessKit-a u egui-ju je ograničen **[PROVJERA]** sa Narratorom.
- **Pokret**: poštuje Windows "Show animations" (2.6).
- **DPI/skaliranje**: egui koristi Windows DPI faktor; layout testiran na 100 %, 150 %, 200 % i min. veličini prozora 880×560.
- **Ciljne površine**: ≥ 32×32 px.

---

## 13. Build, repo, release

### 13.1 Struktura repoa

```
ps-package-pusher/
  .github/
    workflows/ci.yml
    workflows/release.yml
    ISSUE_TEMPLATE/bug_report.md      # traži verziju PSPP, verziju RPI, firmware, kod greške, log
  assets/ (fonts, icon, logo)
  docs/screenshots/ (queue-dark.png, queue-light.png, settings.png, splash.png, error.png)
  lang/ (en.json, bs.json)
  src/ (vidi 7.2)
  tests/
  build.rs                            # winresource: ikona + VERSIONINFO iz Cargo verzije
  Cargo.toml, Cargo.lock
  LICENSE (MIT, Copyright (c) 2026 pinkyBOZZ)
  README.md
  CHANGELOG.md
  DESIGN.md, PLAN.md
```

### 13.2 GitHub Actions

- **ci.yml** (push, pull_request; `windows-latest`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (unit + integracioni sa mock-om), `cargo build --release`. Keš: `~/.cargo` i `target`.
- **release.yml** (tag `v0.0.*`): build `--release --bin ps-package-pusher` sa `RUSTFLAGS=-C target-feature=+crt-static`; provjera da se verzija u tagu poklapa sa `Cargo.toml`; zip `PS-Package-Pusher-<ver>-win-x64.zip` = `ps-package-pusher.exe` + `LICENSE` + `README.md` + `THIRD-PARTY-LICENSES.txt` (OFL fontovi, Phosphor MIT, Rust crate licence generisane alatom `cargo-about` ili ručno); `SHA256SUMS.txt`; GitHub Release kao draft (objavljuje se ručno).
- Bez code signinga u v0.0.x → README objašnjava SmartScreen upozorenje ("More info → Run anyway") i daje SHA256.

### 13.3 README (struktura)

1. Naziv, logo, jedna rečenica šta radi.
2. Screenshotovi (dark + light red čekanja, Postavke, dijalog greške).
3. Disclaimer (vidljivo, odmah ispod): nije povezano sa Sony; samo vlastite kopije i homebrew; bez garancije.
4. Zahtjevi: Windows 11 x64 (Windows 10 22H2 nije testiran); jailbreakovana PS4 sa RPI 3.58 (link na fork); ista lokalna mreža.
5. Brzi start (5 koraka) + Windows Firewall.
6. Troubleshooting po kodovima grešaka (sekcija 10).
7. Build iz izvora, mock server (`cargo run --bin mock-rpi -- --installed CUSA00001`).
8. Licenca (MIT), licence trećih strana, zasluge (flatz — originalni RPI; rawblue33 — fork 3.58).

---

## 14. Otvorena pitanja i stavke za provjeru

### 14.1 Za provjeru na konzoli (RPI 3.58)

1. Tačan JSON odgovor `is_exists` (ime i tip polja `exists`, da li postoje `size`/`version`).
2. Da li `is_exists` radi za update i DLC (po Title ID-u igre ili Content ID-u DLC-a) i da li vraća instaliranu verziju.
3. Tačan JSON `get_task_progress` u 3.58: imena i tipovi polja (broj / decimalni string / hex string), fork polja (`progress_percent`, `transferred_bytes`, ukupna veličina), da li postoje `preparing_percent`/`local_copy_percent`.
4. Signal završetka: kako se razlikuje "preneseno 100 %" od "instalirano"; da li postoji faza instalacije vidljiva kroz API.
5. Format polja `error` i konkretni BGFT kodovi za tipične greške (nedovoljno prostora, URL nedostupan, prekid, neispravan paket).
6. Metoda i format `/api/log` (GET/POST, tekst ili JSON, dužina).
7. Tijela 404 odgovora: uninstall nepostojećeg vs nepoznat endpoint; status za GET na `uninstall_*` (405 ili 404).
8. Tačna lista endpointa koji provjeravaju `Origin`/`Sec-Fetch-Site`.
9. Da li BGFT šalje HEAD prije GET-a, koje oblike Range zaglavlja koristi, veličinu komada, broj paralelnih konekcija, ponavlja li zadnji Range nakon završetka.
10. Split paketi: očekivana konvencija imena, da li samo `_0` ima zaglavlje, da li je redoslijed URL-ova jedini uslov.
11. Semantika `stop_task` vs `unregister_task`; da li treba `unregister_task` nakon uspješne instalacije.
12. Da li task preživi prekid PC servera i da li `resume_task` nastavlja od offseta (9.3).
13. Da li `uninstall_game` čuva save podatke i briše li i update/DLC te igre (tekst dijaloga 4.7 i 4.11).
13a. Imena polja u tijelu za `uninstall_patch` i `uninstall_ac`; ponašanje kad se deinstalacija pošalje dok traje prenos drugog naslova (6.7).
14. Ponašanje kad konzola uđe u rest mode tokom prenosa.
15. Ograničenja dužine URL-a / dozvoljeni znakovi putanje u RPI-ju.

### 14.2 Za provjeru u PKG formatu

16. ~~Offset `entry_count` (0x10) i bit enkripcije u `flags1` (0x80000000)~~ — potvrđeno desk-provjerom (PLAN 3).
17. Ukupna veličina paketa je `pkg_size` u64 @0x430 (desk-provjera); ostaje potvrditi na stvarnom split paketu da `_0` nosi ukupnu veličinu (PLAN S-6).
18. Koji SFO ključ nosi verziju za prikaz po kategoriji (`APP_VER` vs `VERSION`).
19. Kategorije osim `gd/gp/ac` koje se realno pojavljuju (teme, `gde`, `gdk`) i kako ih RPI instalira.

### 14.3 Za provjeru prototipom (Windows/egui)

20. Preobražaj splash → glavni prozor bez treperenja (transparentnost, DWM zaobljenje, DPI); rezervna varijanta sa drugim viewportom.
21. Živa promjena Windows teme kroz eframe; tamna naslovna traka.
22. Da li Windows Firewall prompt dolazi pri bindu na konkretan LAN IP (očekivano da) i za nepotpisan exe.
23. `tiny_http`: socket timeouti i ponašanje kad konzola nestane usred odgovora (zaglavljene niti vs limit 32); odbijanje ne-whitelist IP-a bez odgovora nije moguće (drop `Request`-a šalje 500, vidi 9.4).
24. AccessKit + Narrator: koliko je UI čitljiv.
25. Veličina release exe-a (cilj < 15 MB sa fontovima).

### 14.4 Odluke za vlasnika projekta

26. ~~Naziv~~ — odlučeno: ostaje "PS Package Pusher".
27. ~~Default port~~ — odlučeno: **8095**, uz opciono firewall pravilo (9.8).
28. ~~Reinstaliraj za update~~ — odlučeno: **da, u v1** (dijalog Preskoči / Instaliraj ponovo bez deinstalacije, 6.2 korak 3).
29. ~~PKG-03~~ — odlučeno: **dozvoliti** ako Content ID daje validan Title ID, inače `Invalid`.
30. Odlučeno: autor/nosilac prava je pseudonim **pinkyBOZZ** (1.3).
31. Odlučeno: službena podrška samo **Windows 11**; Windows 10 nema okruženja za test.
