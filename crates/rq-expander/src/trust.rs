use anyhow::Result;
use rq_store::GraphStore;
use std::collections::HashMap;
use std::collections::VecDeque;

/// Configuration for the Web of Trust scoring algorithm.
#[derive(Debug, Clone)]
pub struct TrustConfig {
    /// How much trust decays per traversal level (0.0 – 1.0).
    /// 0.5 means each hop halves the trust contribution.
    pub decay_factor: f64,
    /// Minimum score to include in results (for filtering noise).
    pub min_score: f64,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            decay_factor: 0.5,
            min_score: 0.001,
        }
    }
}

/// A trust score entry for a user.
#[derive(Debug, Clone)]
pub struct TrustEntry {
    pub user_id: String,
    pub username: String,
    pub score: f64,
    pub depth: u32,
}

/// Web of Trust scorer that computes trust scores from traversal edges.
pub struct TrustScorer;

impl TrustScorer {
    /// Compute trust scores for all users in the traversal graph.
    ///
    /// Algorithm:
    /// 1. Seeds start with trust 1.0
    /// 2. BFS outward: each discovered user gets `source_trust × decay_factor`
    /// 3. If a user is reachable via multiple paths, use the MAX score
    ///    (best-path / optimistic model)
    /// 4. Scores are normalized so the maximum is 1.0
    pub fn compute(
        store: &(dyn GraphStore + Send),
        config: &TrustConfig,
    ) -> Result<Vec<TrustEntry>> {
        let seeds = store.list_seeds()?;
        if seeds.is_empty() {
            return Ok(Vec::new());
        }

        // Build adjacency: user → Vec<(neighbor, depth_delta)>
        let mut adjacency: HashMap<String, Vec<(String, u32)>> = HashMap::new();

        for seed in &seeds {
            let edges = store.traversal_edges_by_seed(&seed.id)?;
            for edge in &edges {
                // Edge direction: from_user_id → to_user_id
                adjacency
                    .entry(edge.from_user_id.clone())
                    .or_default()
                    .push((edge.to_user_id.clone(), edge.depth));
            }
        }

        // Initialize scores: seeds = 1.0, others = 0.0
        let mut scores: HashMap<String, f64> = HashMap::new();
        let mut depths: HashMap<String, u32> = HashMap::new();

        for seed in &seeds {
            scores.insert(seed.username.clone(), 1.0);
            depths.insert(seed.username.clone(), 0);
        }

        // BFS propagation using a queue.
        // Each entry: (username, current_trust, current_depth)
        let mut queue: VecDeque<(String, f64, u32)> = VecDeque::new();
        for seed in &seeds {
            queue.push_back((seed.username.clone(), 1.0, 0));
        }

        while let Some((user, trust, _depth)) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&user).cloned() {
                for (neighbor, edge_depth) in &neighbors {
                    let child_depth = *edge_depth;
                    let child_trust = trust * config.decay_factor;

                    let current_score = scores.get(neighbor).copied().unwrap_or(0.0);
                    if child_trust > current_score {
                        scores.insert(neighbor.clone(), child_trust);
                        depths.insert(neighbor.clone(), child_depth);
                        // Continue propagating through this neighbor.
                        queue.push_back((neighbor.clone(), child_trust, child_depth));
                    }
                }
            }
        }

        // Normalize so max is 1.0.
        let max_score = scores.values().cloned().fold(0.0, f64::max);
        let normalization = if max_score > 0.0 { max_score } else { 1.0 };

        let mut entries: Vec<TrustEntry> = scores
            .into_iter()
            .filter_map(|(user_id, raw)| {
                let normalized = raw / normalization;
                if normalized < config.min_score {
                    return None;
                }
                let username = user_id.clone();
                let depth = depths.get(&user_id).copied().unwrap_or(0);
                Some(TrustEntry {
                    user_id,
                    username,
                    score: normalized,
                    depth,
                })
            })
            .collect();

        entries.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(entries)
    }
}
