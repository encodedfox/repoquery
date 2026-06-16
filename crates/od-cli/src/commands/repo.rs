//! Repository metadata management commands.

use anyhow::{bail, Result};
use od_store::{open_store, RepoStore};
use std::path::{Path, PathBuf};

fn open_at(path: &Path) -> Result<Box<dyn RepoStore>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    open_store(path)
}

pub async fn tag(repo_id: String, tag: String, store_path: PathBuf) -> Result<()> {
    let s = open_at(&store_path)?;
    let mut repo = s
        .get_repo(&repo_id)?
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_id))?;
    if !repo.custom_tags.contains(&tag) {
        repo.custom_tags.push(tag.clone());
        s.upsert_repo(&repo)?;
        println!("✅ Tagged '{}' with '{}'", repo_id, tag);
    } else {
        println!("ℹ️  '{}' already has tag '{}'", repo_id, tag);
    }
    Ok(())
}

pub async fn untag(repo_id: String, tag: String, store_path: PathBuf) -> Result<()> {
    let s = open_at(&store_path)?;
    let mut repo = s
        .get_repo(&repo_id)?
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_id))?;
    let before = repo.custom_tags.len();
    repo.custom_tags.retain(|t| t != &tag);
    if repo.custom_tags.len() != before {
        s.upsert_repo(&repo)?;
        println!("✅ Removed tag '{}' from '{}'", tag, repo_id);
    } else {
        bail!("'{}' does not have tag '{}'", repo_id, tag);
    }
    Ok(())
}

pub async fn note(repo_id: String, text: String, store_path: PathBuf) -> Result<()> {
    let s = open_at(&store_path)?;
    let mut repo = s
        .get_repo(&repo_id)?
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_id))?;
    repo.curator_notes = Some(text);
    s.upsert_repo(&repo)?;
    println!("✅ Note set on '{}'", repo_id);
    Ok(())
}

pub async fn show(repo_id: String, store_path: PathBuf) -> Result<()> {
    let s = open_at(&store_path)?;
    let repo = s
        .get_repo(&repo_id)?
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_id))?;

    println!("📦 {}", repo.metadata.full_name);
    println!("  ID:          {}", repo.id);
    println!("  Description: {}", repo.metadata.description);
    println!("  Language:    {}", repo.metadata.primary_language);
    println!("  Stars:       {}", repo.metadata.stars);
    println!("  Quality:     {}/100", repo.quality_metrics.quality_score);
    println!("  Archived:    {}", repo.quality_metrics.archive_status);
    if !repo.metadata.topics.is_empty() {
        println!("  Topics:      {}", repo.metadata.topics.join(", "));
    }
    if !repo.custom_tags.is_empty() {
        println!("  Tags:        {}", repo.custom_tags.join(", "));
    }
    if let Some(notes) = &repo.curator_notes {
        println!("  Notes:       {}", notes);
    }
    if let Some(parent) = &repo.fork_parent {
        println!("  Fork of:     {}", parent);
        if let Some(ahead) = repo.fork_ahead {
            println!("  Ahead:       {}", ahead);
        }
        if let Some(behind) = repo.fork_behind {
            println!("  Behind:      {}", behind);
        }
    }
    if !repo.relations.is_empty() {
        let rels: Vec<String> = repo.relations.iter().map(|r| r.to_string()).collect();
        println!("  Relations:   {}", rels.join(", "));
    }
    Ok(())
}
