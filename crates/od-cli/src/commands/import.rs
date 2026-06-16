use anyhow::Result;
use od_store::open_store;
use std::path::PathBuf;

pub async fn run(from: PathBuf, to: PathBuf) -> Result<()> {
    println!("📥 Importing: {} → {}", from.display(), to.display());
    let src = open_store(&from)?;
    let dst = open_store(&to)?;
    let data = src.load_all()?;
    println!(
        "  Loaded: {} repos, {} books, {} refs, {} manual",
        data.repositories.len(),
        data.books.len(),
        data.web_references.len(),
        data.manual_projects.len()
    );
    dst.save_all(&data)?;
    println!("✅ Import complete");
    Ok(())
}
