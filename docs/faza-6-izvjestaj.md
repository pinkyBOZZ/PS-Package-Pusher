# Faza 6 — izvještaj (Sesija B na konzoli, skraćena, + popravke)

Datum: 2026-11-21 · Verzija: 0.0.1-rc.5 (RC build pokrenut sa `--debug`) · Testovi: `cargo test` prolazi (uključujući novi test `sesija_b_console_fixtures`), clippy `-D warnings` i rustfmt čisti.

## Zašto skraćena sesija

Sesija B je dogovorena kao skraćena. Sesija A je završena ranije istog dana, pa B1 nije ponovljen. B7, B12, B13, B15 i B16 nisu urađeni. B14 nije moguć jer vlasnik nema split pakete. Rezultati ispod su samo ono što je izmjereno.

## Koraci sesije

| Korak | Očekivano | Dobijeno | Status |
|---|---|---|---|
| B1 | Sesija A (A1–A10), ako nije urađena | Preskočen, Sesija A završena ranije istog dana | preskočen (dogovor) |
| B2 | Firewall pravilo i Windows prompt | Prenosi rade na ovom računaru; pravilo i prompt obrađeni ranije, novi prompt nije viđen | potvrđeno (funkcionalno) |
| B3 | `pkg_dump` nad H1, G, S, X | 32 paketa (10 gd, 4 gp, 18 ac): magic ispravan za sve; `pkg_size` @0x430 jednak veličini fajla za sve; entry-ji 0x1000 (param.sfo) i 0x1200 (icon0) nisu kriptovani ni u jednom; CATEGORY prepoznat za sve. Split paketi nisu bili dostupni | potvrđeno za nesplit; split otvoren (14.2/17) |
| B4 | Obrazac HEAD / Range, veličina komada, konekcije, faza "Installing" | Konzola ne šalje HEAD. Prvo dva mala Range GET-a za zaglavlje (na odvojenim konekcijama), zatim nakon `install` jedan GET `bytes=65536-11010047`, pa uzastopni komadi od tačno 512 MiB preko jedne keep-alive konekcije. Offseti preko 4 GB rade. Zasebna faza instalacije nije bila potrebna: signal "instalirano" stigao je ~1 s nakon zadnjeg bajta | drugačije od pretpostavke (nema HEAD-a, nema faze "Installing") |
| B5 | `is_exists` za H1 nakon instalacije | `{"status":"success","exists":"true","size":9328275456,"size_hex":"0x22C023000"}`; `exists` je string, nema `version` | potvrđeno |
| B6 | Ponovno dodavanje instalirane igre: "Već instalirano" → Preskoči | Dijalog i Preskoči rade; stavka dobija `skipped` | potvrđeno (reinstalacija kroz "Pokušaj ponovo" nije posebno zabilježena) |
| B7 | Save podaci, deinstalacija G, provjera save-a, update i DLC | Nije urađeno | nije izvedeno |
| B8 | `is_exists` za igru, update i DLC nakon svake instalacije | Igra: `exists` true, `size` 9328275456. Nakon update-a: `size` 11889328128, bez verzije. DLC nije vidljiv u upitu po Title ID-u; upit po Content ID-u nije rađen | djelimično |
| B9 | Ponovna instalacija istog update-a | Konzola prihvatila, novi task, instalirano za 43 s, bez greške. Zaostali završeni task ne blokira instalaciju | potvrđeno |
| B10 | Pauza 30 s, nastavak, pa otkaz usred prenosa | Otkaz: task nestao sa konzole (`get_task_progress` → `0x80990019`) i iz Notifications → Downloads. Pauza/nastavak nisu posebno zabilježeni. Nalaz: otkaz se ne upisuje u log aplikacije | djelimično |
| B11 | Ugasiti aplikaciju pri 40 %, pokrenuti, "Pokušaj ponovo" | Task ostao na konzoli. Aplikacija pokrenuta nakon ~12 s, "resumed saved task" u logu. Konzola nastavila sa `bytes=3772186624-`, tj. tačno od prenesenih bajtova | potvrđeno (DESIGN 9.3) |
| B12 | Deinstalacija H1 usred prenosa X | Nije urađeno | nije izvedeno |
| B13 | Rest mode tokom prenosa | Nije urađeno | nije izvedeno |
| B14 | Split paket S | Nije moguće (nema split paketa) | nije izvedeno |
| B15 | Namjerne greške (skraćen paket, izvučen kabl, USB disk) | Nije urađeno | nije izvedeno |
| B16 | Drugi tip paketa (opciono) | Nije urađeno | nije izvedeno |

Nađeni kodovi konzole: `0x80990015` (install odbijen zbog starog neuspjelog taska, viđeno ranije istog dana), `0x80990019` (task ne postoji, nakon otkaza), `0x80991404` (iz Sesije A, prije rc.4: URL odgovorio 404).

## Popravke iz rc.5 nađene tokom Faze 6

- **Zaostali neuspjeli task** je blokirao sljedeću instalaciju istog paketa (`0x80990015`). rc.5 odjavljuje task pri grešci (`src/engine/mod.rs`).
- **ETA kolona** u redu čekanja: popravljen prikaz (DESIGN 9.2).
- **Automatski indikator konzole**: osvježava se sam (startup test i `ConsoleStatus` događaj).
- **Query string u URL-u paketa** (Sesija A): konzola šalje URL sa query stringom; prije rc.4 server je na to odgovarao 404 (`0x80991404`). rc.4 ignoriše query string.

## Nalazi koji odstupaju od dokumentacije

1. **Nema HEAD-a i nema faze "Installing" vidljive kroz API** (DESIGN 5.2, 6.2 korak 7, 14.1/4). Odluka da li se stanje `Installing` ugrađuje ili briše iz DESIGN-a ostaje za CP-6.
2. **Završeni taskovi ostaju na konzoli.** Konzola ih i dalje prikazuje na 100 % minutama kasnije, a aplikacija ne zove `unregister_task` nakon uspjeha. Stavke ostaju u Downloads listi (vidi README).
3. **`is_exists` nema `version`.** Dijalog "update već instaliran" (DESIGN 6.2 korak 3) se zato nikad ne pokreće; aplikacija se ponaša kako je dizajnirano za slučaj bez verzije.
4. **Fail odgovor za task nosi `error_code`, a ne `error`.** Primjer: `{"status":"fail","error_code":-2137456615,"error_code_hex":"0x80990019"}`. `classify` je ranije čitao samo `error`, pa tekst greške ostajao prazan. Popravljeno: `classify` sada čita `error_code_hex` (ili `error_code` kao hex).

## Nije provjereno / otvoreno

- Stavke B7, B12, B13, B14, B15, B16 (tabela iznad).
- DESIGN 14.1/2: DLC po Content ID-u. 14.1/9: da li konzola ponavlja zadnji Range nakon završetka. 14.1/5: ostali BGFT kodovi (nedovoljno prostora, neispravan paket, prekid).
- 14.1/6–8 i 15 (`/api/log`, 404 odgovori, Origin provjere, dužina URL-a) nisu dio ove sesije.
- 14.2/18–19 (APP_VER vs VERSION; kategorije `gde`, `gdk`, teme) nisu provjereni.
- Narrator i čitač ekrana (Faza 5).

## Mali popravci (nisu urađeni u ovoj fazi)

1. Poziv `unregister_task` nakon uspjeha, da Downloads lista na konzoli ostaje čista.
2. Upisivanje otkaza u log aplikacije.
3. Čitanje `error_code` (broj i hex) u `classify` za fail odgovor task-a. Urađeno (vidi stavku 4 iznad).

## Put do release-a

- **CP-6 (vlasnik):** ovaj izvještaj, DESIGN 14.1/14.2 upisani, odluka o `Installing`, odobrenje ulaska u release fazu.
- **Release checklist (PLAN sekcija 8):** `Cargo.toml` na 0.0.1 (sada je 0.0.1-rc.5), CHANGELOG 0.0.1, CI, ručna UI matrica na finalnom buildu, README po 13.3, licence, provjera da nema ličnih podataka u repou i fixture-ima.
- **CP-7 (vlasnik):** go/no-go po checklisti; objava taga `v0.0.1` i draft release-a.
