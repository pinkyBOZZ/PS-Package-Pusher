//! Writes fake PS4 packages for manual UI/engine testing (PLAN 1.15). Not installable.
//! Run: cargo run --example make_fixture -- <empty-folder>

use pspp::pkg::fixture::{Fixture, write_sparse};
use std::{fs, path::PathBuf};

fn main() -> std::io::Result<()> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: make_fixture <folder>"),
    );
    fs::create_dir_all(&dir)?;
    Fixture::game("CUSA00001").write(&dir.join("Game A.pkg"))?;
    Fixture::update("CUSA00001", "01.05").write(&dir.join("Game A Update 1.05.pkg"))?;
    Fixture::dlc("CUSA00001").write(&dir.join("Game A DLC.pkg"))?;
    Fixture::update("CUSA00777", "01.02").write(&dir.join("Update without base.pkg"))?;

    let mut big = Fixture::game("CUSA00002");
    big.title = "Big Game (5 GB, sparse)".into();
    big.total_size = 5 << 30;
    big.write(&dir.join("Big Game.pkg"))?;

    let mut split = Fixture::game("CUSA00003");
    split.title = "Split Game (3 parts)".into();
    split.total_size = 3 << 30;
    split.write_split(&dir, "Split Game", 3)?;

    let mut missing = Fixture::game("CUSA00004");
    missing.title = "Split missing part".into();
    let parts = missing.write_split(&dir, "Broken Split", 3)?;
    fs::remove_file(&parts[1])?;

    let mut nosfo = Fixture::game("CUSA00005");
    nosfo.encrypt_sfo = true;
    nosfo.write(&dir.join("No sfo.pkg"))?;

    let b = Fixture::game("CUSA00006").bytes();
    write_sparse(&dir.join("Truncated.pkg"), &b[..0x300], 0x300)?;
    fs::write(dir.join("Not a pkg.pkg"), b"PK\x03\x04 this is a zip")?;
    fs::write(dir.join("notes.txt"), "ignored: not a .pkg")?;
    println!("fixtures written to {}", dir.display());
    Ok(())
}
