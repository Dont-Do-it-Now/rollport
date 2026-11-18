//! PoC: can AFC read /PhotoData/Photos.sqlite, and what does it say?
//! See docs/media-classification.md. Run with an unlocked, trusted iPhone on USB:
//!   cargo run --example photodb_poc [udid]
//! The copied database stays in the OS temp folder (rollport-photodb-poc); delete it after.

use std::collections::{BTreeMap, HashSet};

use idevice::{
    IdeviceService,
    afc::{AfcClient, opcode::AfcFopenMode},
    usbmuxd::{Connection, UsbmuxdAddr},
};
use rusqlite::Connection as Db;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Res<()> {
    let wanted = std::env::args().nth(1);

    let addr = UsbmuxdAddr::from_env_var()?;
    let mut muxer = addr.connect(0).await?;
    let device = muxer
        .get_devices()
        .await?
        .into_iter()
        .find(|d| {
            d.connection_type == Connection::Usb && wanted.as_ref().is_none_or(|u| &d.udid == u)
        })
        .ok_or("no iPhone on USB")?;
    println!("device: {}", device.udid);

    let provider = device.to_provider(addr, "rollport-poc");
    let mut afc = AfcClient::connect(&provider).await?;

    // 1. /PhotoData
    println!("\n== /PhotoData ==");
    match afc.list_dir("/PhotoData").await {
        Ok(names) => println!("{names:?}"),
        Err(e) => return Err(format!("/PhotoData not listable: {e}").into()),
    }

    // 2. copy the database with its WAL
    let out = std::env::temp_dir().join("rollport-photodb-poc");
    std::fs::create_dir_all(&out)?;
    println!("\n== copy -> {} ==", out.display());
    for name in ["Photos.sqlite", "Photos.sqlite-wal", "Photos.sqlite-shm"] {
        let path = format!("/PhotoData/{name}");
        match copy(&mut afc, &path, &out.join(name)).await {
            Ok(n) => println!("{name}: {n} bytes"),
            Err(e) => println!("{name}: FAILED {e}"),
        }
    }

    let db = Db::open(out.join("Photos.sqlite"))?;

    // 3. schema
    let tables: Vec<String> = strings(
        &db,
        "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('ZASSET','ZGENERICASSET')",
    )?;
    let table = tables.first().ok_or("no ZASSET / ZGENERICASSET table")?.clone();
    let columns: Vec<String> = strings(&db, &format!("SELECT name FROM pragma_table_info('{table}')"))?;
    println!("\n== {table}: {} columns ==\n{}", columns.len(), columns.join(", "));
    let has = |c: &str| columns.iter().any(|x| x == c);

    let interesting = [
        "ZKIND",
        "ZKINDSUBTYPE",
        "ZPLAYBACKSTYLE",
        "ZPLAYBACKVARIATION",
        "ZDEPTHTYPE",
        "ZAVALANCHEPICKTYPE",
        "ZTRASHEDSTATE",
        "ZHIDDEN",
        "ZHASADJUSTMENTS",
        "ZSAVEDASSETTYPE",
        "ZCLOUDLOCALSTATE",
        "ZVISIBILITYSTATE",
    ];

    // 4. value distributions, with a sample file for each
    for col in interesting.iter().filter(|c| has(c)) {
        println!("\n== {col} ==");
        let sql = format!(
            "SELECT {col}, COUNT(*), MIN(ZDIRECTORY || '/' || ZFILENAME) FROM {table} GROUP BY {col} ORDER BY {col}"
        );
        let mut stmt = db.prepare(&sql)?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<i64>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (v, n, sample) = row?;
            println!("{v:?}\t{n}\te.g. {}", sample.unwrap_or_default());
        }
    }
    for col in interesting.iter().filter(|c| !has(c)) {
        println!("(missing column {col})");
    }

    // ZKIND x ZKINDSUBTYPE together
    if has("ZKIND") && has("ZKINDSUBTYPE") {
        println!("\n== ZKIND, ZKINDSUBTYPE ==");
        let mut stmt = db.prepare(&format!(
            "SELECT ZKIND, ZKINDSUBTYPE, COUNT(*) FROM {table} GROUP BY 1, 2 ORDER BY 1, 2"
        ))?;
        for row in stmt.query_map([], |r| {
            Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, Option<i64>>(1)?, r.get::<_, i64>(2)?))
        })? {
            println!("{:?}", row?);
        }
    }

    // 5. bursts
    if has("ZAVALANCHEUUID") {
        let groups: i64 = db.query_row(
            &format!("SELECT COUNT(DISTINCT ZAVALANCHEUUID) FROM {table} WHERE ZAVALANCHEUUID IS NOT NULL"),
            [],
            |r| r.get(0),
        )?;
        let frames: i64 = db.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE ZAVALANCHEUUID IS NOT NULL"),
            [],
            |r| r.get(0),
        )?;
        println!("\n== bursts: {groups} groups, {frames} frames ==");
    }

    // 6. DB rows vs files actually in /DCIM
    let mut dcim = HashSet::new();
    let mut pending = vec!["/DCIM".to_string()];
    while let Some(dir) = pending.pop() {
        for name in afc.list_dir(&dir).await? {
            if name == "." || name == ".." {
                continue;
            }
            let path = format!("{dir}/{name}");
            if name.contains('.') {
                dcim.insert(path[1..].to_uppercase());
            } else {
                pending.push(path);
            }
        }
    }
    let rows: Vec<String> = strings(
        &db,
        &format!("SELECT UPPER(ZDIRECTORY || '/' || ZFILENAME) FROM {table} WHERE ZDIRECTORY LIKE 'DCIM/%'"),
    )?;
    let in_db: HashSet<_> = rows.iter().cloned().collect();
    let only_db = rows.iter().filter(|p| !dcim.contains(*p)).count();
    let only_files: Vec<_> = dcim.iter().filter(|p| !in_db.contains(*p)).collect();
    println!(
        "\n== match ==\n/DCIM files: {}\nDB rows under DCIM: {}\nDB rows with no file: {only_db}\nfiles with no DB row: {} (by ext: {:?})",
        dcim.len(),
        rows.len(),
        only_files.len(),
        by_extension(&only_files),
    );
    for p in only_files.iter().take(5) {
        println!("  {p}");
    }

    // 7. where the portrait render and edits live
    if has("ZDEPTHTYPE") {
        let sample: Option<(String, String)> = db
            .query_row(
                &format!(
                    "SELECT ZDIRECTORY, ZFILENAME FROM {table} WHERE ZDEPTHTYPE != 0 AND ZTRASHEDSTATE = 0 LIMIT 1"
                ),
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        if let Some((dir, file)) = sample {
            let stem = file.rsplit_once('.').map_or(file.as_str(), |(s, _)| s);
            println!("\n== portrait sample {dir}/{file} ==");
            for place in [
                format!("/PhotoData/Mutations/{dir}/{stem}"),
                format!("/PhotoData/Mutations/{dir}/{stem}/Adjustments"),
            ] {
                println!("{place}: {:?}", afc.list_dir(&place).await);
            }
        }
    }
    println!("\n== /PhotoData/Mutations ==\n{:?}", afc.list_dir("/PhotoData/Mutations").await);

    Ok(())
}

async fn copy(afc: &mut AfcClient, from: &str, to: &std::path::Path) -> Res<usize> {
    let mut file = afc.open(from, AfcFopenMode::RdOnly).await?;
    let read = file.read_entire().await;
    file.close().await?;
    let bytes = read?;
    std::fs::write(to, &bytes)?;
    Ok(bytes.len())
}

fn strings(db: &Db, sql: &str) -> Res<Vec<String>> {
    let mut stmt = db.prepare(sql)?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn by_extension(paths: &[&String]) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for p in paths {
        let ext = p.rsplit_once('.').map_or("", |(_, e)| e).to_string();
        *map.entry(ext).or_default() += 1;
    }
    map
}
