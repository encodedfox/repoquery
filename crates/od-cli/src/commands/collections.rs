use anyhow::{bail, Result};
use od_core::Collection;
use od_store::{open_store, RepoFilter, RepoStore};
use std::path::{Path, PathBuf};

fn store() -> Result<Box<dyn RepoStore>> {
    let path = Path::new("data/omnidatum.db");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    open_store(path)
}

fn open_at(path: &Path) -> Result<Box<dyn RepoStore>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    open_store(path)
}

fn slugify(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

pub async fn list() -> Result<()> {
    let s = store()?;
    let collections = s.list_collections()?;
    if collections.is_empty() {
        println!("📂 No collections found.");
        return Ok(());
    }
    println!("📂 Collections ({}):", collections.len());
    for c in &collections {
        println!("  {} — {} ({} repos)", c.id, c.name, c.repo_ids.len());
    }
    Ok(())
}

pub async fn create(name: String, description: Option<String>) -> Result<()> {
    let s = store()?;
    let id = slugify(&name);
    if s.get_collection(&id)?.is_some() {
        bail!("Collection '{}' already exists", id);
    }
    let mut c = Collection::new(id.clone(), name);
    c.description = description;
    s.save_collection(&c)?;
    println!("✅ Created collection '{}'", id);
    Ok(())
}

pub async fn show(id: String) -> Result<()> {
    let s = store()?;
    match s.get_collection(&id)? {
        None => bail!("Collection '{}' not found", id),
        Some(c) => {
            println!("📂 {} — {}", c.id, c.name);
            if let Some(desc) = &c.description {
                println!("   {}", desc);
            }
            println!("   Repos: {}", c.repo_ids.len());
            for r in &c.repo_ids {
                println!("     - {}", r);
            }
        }
    }
    Ok(())
}

pub async fn add(collection: String, repo: String) -> Result<()> {
    let s = store()?;
    let mut c = s
        .get_collection(&collection)?
        .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection))?;
    c.add_repo(repo.clone());
    s.save_collection(&c)?;
    println!("✅ Added '{}' to '{}'", repo, collection);
    Ok(())
}

pub async fn remove(collection: String, repo: String) -> Result<()> {
    let s = store()?;
    let mut c = s
        .get_collection(&collection)?
        .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection))?;
    if !c.remove_repo(&repo) {
        bail!("'{}' is not in collection '{}'", repo, collection);
    }
    s.save_collection(&c)?;
    println!("✅ Removed '{}' from '{}'", repo, collection);
    Ok(())
}

pub async fn delete(id: String) -> Result<()> {
    let s = store()?;
    if !s.delete_collection(&id)? {
        bail!("Collection '{}' not found", id);
    }
    println!("🗑️  Deleted collection '{}'", id);
    Ok(())
}

pub async fn auto_generate(min_repos: usize, store_path: PathBuf) -> Result<()> {
    let s = open_at(&store_path)?;
    let repos = s.list_repos(&RepoFilter::default())?;

    // topic → repo ids
    let mut topic_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for repo in &repos {
        for topic in &repo.metadata.topics {
            topic_map
                .entry(topic.clone())
                .or_default()
                .push(repo.id.clone());
        }
    }

    let mut created = 0usize;
    for (topic, ids) in &topic_map {
        if ids.len() < min_repos {
            continue;
        }
        let id = format!("topic:{}", topic);
        let name = format!("Topic: {}", topic);
        let mut col = Collection::new(id, name);
        for repo_id in ids {
            col.add_repo(repo_id.clone());
        }
        s.save_collection(&col)?;
        created += 1;
    }

    println!("✅ Created {} auto-collections from topics", created);
    Ok(())
}
