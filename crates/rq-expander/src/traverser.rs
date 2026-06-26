use crate::platform::PlatformApiClient;
use anyhow::Result;
use rq_core::{EdgeType, FgatToken, PlatformKind, Seed, SeedStatus, TokenStatus, TraversalEdge};
use rq_store::GraphStore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

// ── FGAT Pool ───────────────────────────────────────────────────────────────────

struct PoolInner {
    tokens: Vec<FgatToken>,
    in_flight: Vec<(String, usize)>,
}

/// A raw token acquired from the pool.
pub struct AcquiredToken {
    pub raw: String,
    id: String,
    inner: Arc<Mutex<PoolInner>>,
}

impl Drop for AcquiredToken {
    fn drop(&mut self) {
        let id = self.id.clone();
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut guard = inner.lock().await;
            if let Some(entry) = guard.in_flight.iter_mut().find(|(tid, _)| tid == &id) {
                entry.1 = entry.1.saturating_sub(1);
            }
        });
    }
}

/// Manages a pool of FGAT tokens, selecting the best candidate per request.
pub struct FgatPool {
    inner: Arc<Mutex<PoolInner>>,
    store: Arc<Mutex<dyn GraphStore + Send>>,
    counter: AtomicUsize,
}

impl FgatPool {
    pub fn new(store: Arc<Mutex<dyn GraphStore + Send>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PoolInner {
                tokens: Vec::new(),
                in_flight: Vec::new(),
            })),
            store,
            counter: AtomicUsize::new(0),
        }
    }

    /// Load tokens from the store into the pool.
    pub async fn load(&self, platform: PlatformKind) -> Result<()> {
        let store_guard = self.store.lock().await;
        let all = store_guard.list_fgat_tokens()?;
        let mut guard = self.inner.lock().await;
        guard.tokens.clear();
        guard.in_flight.clear();
        for t in all {
            if t.platform == platform && t.status == TokenStatus::Available {
                guard.in_flight.push((t.id.clone(), 0));
                guard.tokens.push(t);
            }
        }
        Ok(())
    }

    /// Acquire the best available token that isn't currently in use.
    pub async fn acquire(&self, _platform: PlatformKind) -> Option<AcquiredToken> {
        let mut guard = self.inner.lock().await;
        if guard.tokens.is_empty() {
            return None;
        }

        // Build a list of candidates (tokens with in_flight == 0).
        let candidates: Vec<usize> = guard
            .tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                guard
                    .in_flight
                    .get(*i)
                    .map(|(_, count)| *count == 0)
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();

        if candidates.is_empty() {
            return None;
        }

        let offset = self.counter.fetch_add(1, Ordering::Relaxed);
        let best_idx = candidates
            .iter()
            .max_by(|a, b| {
                let a_rem = guard.tokens[**a].rate_limit_remaining.unwrap_or(0);
                let b_rem = guard.tokens[**b].rate_limit_remaining.unwrap_or(0);
                a_rem.cmp(&b_rem).then_with(|| {
                    let da = (**a as i64 - offset as i64).unsigned_abs();
                    let db = (**b as i64 - offset as i64).unsigned_abs();
                    da.cmp(&db)
                })
            })
            .copied()?;

        let token = &guard.tokens[best_idx];
        let token_id = token.id.clone();
        let raw = token
            .raw_token
            .clone()
            .unwrap_or_else(|| token.token_hash.clone());

        // Mark as in-flight.
        if let Some(entry) = guard.in_flight.iter_mut().find(|(id, _)| id == &token_id) {
            entry.1 += 1;
        }

        Some(AcquiredToken {
            raw,
            id: token_id,
            inner: self.inner.clone(),
        })
    }

    /// Release a token back to the pool after use.
    pub async fn release(&self, token: AcquiredToken) {
        let mut guard = self.inner.lock().await;
        if let Some(entry) = guard.in_flight.iter_mut().find(|(tid, _)| tid == &token.id) {
            entry.1 = 0;
        }
    }

    /// Mark a token as exhausted (rate limited) and persist to store.
    pub async fn mark_exhausted(&self, id: &str) -> Result<()> {
        {
            let store_guard = self.store.lock().await;
            store_guard.update_fgat_token_status(id, "exhausted")?;
        }
        let mut guard = self.inner.lock().await;
        guard.tokens.retain(|t| t.id != id);
        guard.in_flight.retain(|(tid, _)| tid != id);
        Ok(())
    }

    /// Reload all tokens from the store (e.g. after adding new ones).
    pub async fn reload(&self, platform: PlatformKind) -> Result<()> {
        self.load(platform).await
    }
}

// ── BFS Traverser ──────────────────────────────────────────────────────────────

/// BFS traversal engine that walks the social graph starting from seeds.
pub struct BfsTraverser {
    store: Arc<Mutex<dyn GraphStore + Send>>,
    client: Arc<dyn PlatformApiClient + Send + Sync>,
    _pool: Arc<FgatPool>,
    concurrency: usize,
}

impl BfsTraverser {
    pub fn new(
        store: Arc<Mutex<dyn GraphStore + Send>>,
        client: Arc<dyn PlatformApiClient + Send + Sync>,
        pool: Arc<FgatPool>,
    ) -> Self {
        Self {
            store,
            client,
            _pool: pool,
            concurrency: 4,
        }
    }

    pub fn with_concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    /// Run one full BFS pass: process all seeds up to their max_depth.
    pub async fn run(&self) -> Result<BfsReport> {
        let mut report = BfsReport::default();

        let seeds = {
            let guard = self.store.lock().await;
            guard.list_seeds()?
        };

        if seeds.is_empty() {
            tracing::info!("No seeds to process");
            return Ok(report);
        }

        let max_overall = seeds.iter().map(|s| s.max_depth).max().unwrap_or(0);

        for depth in 0..=max_overall {
            let current: Vec<Seed> = seeds
                .iter()
                .filter(|s| {
                    s.depth == depth
                        && (s.status == SeedStatus::Pending || s.status == SeedStatus::Active)
                })
                .cloned()
                .collect();

            if current.is_empty() {
                continue;
            }

            tracing::info!(
                "Processing {} seeds at depth {}/{}",
                current.len(),
                depth,
                max_overall
            );

            let semaphore = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
            let mut handles = Vec::new();

            for seed in current {
                #[allow(clippy::expect_used)]
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("semaphore closed");
                let store = self.store.clone();
                let client = self.client.clone();
                let max_depth = seed.max_depth;

                handles.push(tokio::spawn(async move {
                    let result =
                        Self::process_single_seed(&store, &*client, &seed, max_depth).await;
                    drop(permit);
                    result
                }));
            }

            for handle in handles {
                match handle.await {
                    Ok(Ok(r)) => {
                        report.edges_discovered += r.edges_discovered;
                        report.seeds_completed += 1;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Seed processing failed: {}", e);
                        report.errors += 1;
                    }
                    Err(e) => {
                        tracing::error!("Task join error: {}", e);
                        report.errors += 1;
                    }
                }
            }
        }

        Ok(report)
    }

    async fn process_single_seed(
        store: &Arc<Mutex<dyn GraphStore + Send>>,
        client: &(dyn PlatformApiClient + Send + Sync),
        seed: &Seed,
        max_depth: u32,
    ) -> Result<SeedResult> {
        let mut result = SeedResult::default();

        {
            let guard = store.lock().await;
            guard.update_seed_status(&seed.id, "active")?;
        }

        let followers = match client.fetch_followers(&seed.username).await {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to fetch followers for {}: {}", seed.username, e);
                let guard = store.lock().await;
                let _ = guard.update_seed_status(&seed.id, "failed");
                return Err(e);
            }
        };

        let following = match client.fetch_following(&seed.username).await {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to fetch following for {}: {}", seed.username, e);
                let guard = store.lock().await;
                let _ = guard.update_seed_status(&seed.id, "failed");
                return Err(e);
            }
        };

        {
            let guard = store.lock().await;

            for follower in &followers {
                if guard.has_user_been_visited(&seed.id, follower, max_depth)? {
                    continue;
                }
                let edge = TraversalEdge {
                    id: 0,
                    seed_id: seed.id.clone(),
                    from_user_id: follower.clone(),
                    to_user_id: seed.username.clone(),
                    relation_type: EdgeType::Follows,
                    depth: seed.depth + 1,
                    discovered_at: chrono::Utc::now().to_rfc3339(),
                };
                guard.add_traversal_edge(&edge)?;
                result.edges_discovered += 1;

                // Propagate: create seed for discovered user if within max_depth.
                if edge.depth < seed.max_depth {
                    Self::ensure_seed(&*guard, seed, follower, edge.depth).ok();
                }
            }

            for followee in &following {
                if guard.has_user_been_visited(&seed.id, followee, max_depth)? {
                    continue;
                }
                let edge = TraversalEdge {
                    id: 0,
                    seed_id: seed.id.clone(),
                    from_user_id: seed.username.clone(),
                    to_user_id: followee.clone(),
                    relation_type: EdgeType::Follows,
                    depth: seed.depth + 1,
                    discovered_at: chrono::Utc::now().to_rfc3339(),
                };
                guard.add_traversal_edge(&edge)?;
                result.edges_discovered += 1;

                // Propagate: create seed for discovered user if within max_depth.
                if edge.depth < seed.max_depth {
                    Self::ensure_seed(&*guard, seed, followee, edge.depth).ok();
                }
            }

            guard.update_seed_status(&seed.id, "completed")?;
        }

        Ok(result)
    }

    /// Create a pending seed for a discovered user if not already registered.
    fn ensure_seed(
        store: &(dyn GraphStore + Send),
        parent: &Seed,
        discovered_username: &str,
        depth: u32,
    ) -> Result<()> {
        let child_id = format!("{}-{}", parent.platform, discovered_username);
        if store.get_seed(&child_id)?.is_some() {
            return Ok(()); // already a seed
        }
        let child = Seed {
            id: child_id,
            platform: parent.platform.clone(),
            username: discovered_username.to_string(),
            status: SeedStatus::Pending,
            depth,
            max_depth: parent.max_depth,
            added_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            error_message: None,
        };
        store.add_seed(&child)?;
        Ok(())
    }
}

// ── Report types ───────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct BfsReport {
    pub edges_discovered: u64,
    pub seeds_completed: u64,
    pub errors: u64,
}

#[derive(Debug, Default, Clone)]
struct SeedResult {
    edges_discovered: u64,
}
