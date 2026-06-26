use anyhow::Result;
use async_trait::async_trait;
use rq_core::{FgatToken, PlatformKind, Seed, SeedStatus, TokenStatus};
use rq_expander::{
    BfsTraverser, FgatPool, PlatformApiClient, PlatformRepo, RepoCollector, TrustEntry, TrustScorer,
};
use rq_store::{GraphStore, RepoStore, SqliteStore};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

// ── Helpers ────────────────────────────────────────────────────────────────────

fn make_store() -> Arc<Mutex<dyn GraphStore + Send>> {
    let store = SqliteStore::new(std::path::Path::new(":memory:")).unwrap();
    Arc::new(Mutex::new(store))
}

fn make_token(id: &str, hash: &str, remaining: u32) -> FgatToken {
    FgatToken {
        id: id.to_string(),
        platform: PlatformKind::GitHub,
        token_hash: hash.to_string(),
        raw_token: Some(hash.to_string()),
        status: TokenStatus::Available,
        requests_used: 0,
        rate_limit_limit: Some(5000),
        rate_limit_remaining: Some(remaining),
        rate_limit_reset_at: None,
        last_used_at: None,
        added_at: chrono::Utc::now().to_rfc3339(),
        expires_at: None,
        notes: None,
    }
}

fn make_seed(id: &str, username: &str, depth: u32, max_depth: u32) -> Seed {
    Seed {
        id: id.to_string(),
        platform: PlatformKind::GitHub,
        username: username.to_string(),
        status: SeedStatus::Pending,
        depth,
        max_depth,
        added_at: chrono::Utc::now().to_rfc3339(),
        completed_at: None,
        error_message: None,
    }
}

// ── Mock PlatformApiClient ─────────────────────────────────────────────────────

struct MockClient {
    followers: Vec<String>,
    following: Vec<String>,
    repos: Vec<PlatformRepo>,
    fetch_count: Arc<AtomicUsize>,
}

impl MockClient {
    fn new(followers: Vec<String>, following: Vec<String>) -> Self {
        Self {
            followers,
            following,
            repos: Vec::new(),
            fetch_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl PlatformApiClient for MockClient {
    fn platform(&self) -> PlatformKind {
        PlatformKind::GitHub
    }

    async fn fetch_followers(&self, _username: &str) -> Result<Vec<String>> {
        self.fetch_count.fetch_add(1, Ordering::Relaxed);
        Ok(self.followers.clone())
    }

    async fn fetch_following(&self, _username: &str) -> Result<Vec<String>> {
        self.fetch_count.fetch_add(1, Ordering::Relaxed);
        Ok(self.following.clone())
    }

    async fn fetch_user_repos(&self, _username: &str) -> Result<Vec<PlatformRepo>> {
        self.fetch_count.fetch_add(1, Ordering::Relaxed);
        Ok(self.repos.clone())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_fgat_pool_acquire_release() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));

    store
        .lock()
        .await
        .add_fgat_token(&make_token("t1", "ghp_token1", 5000))
        .unwrap();

    pool.load(PlatformKind::GitHub).await.unwrap();

    let t = pool.acquire(PlatformKind::GitHub).await;
    assert!(t.is_some());
    assert_eq!(t.as_ref().unwrap().raw, "ghp_token1");

    pool.release(t.unwrap()).await;

    let t2 = pool.acquire(PlatformKind::GitHub).await;
    assert!(t2.is_some());
    pool.release(t2.unwrap()).await;
}

#[tokio::test]
async fn test_fgat_pool_mark_exhausted() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));

    store
        .lock()
        .await
        .add_fgat_token(&make_token("t1", "ghp_token1", 0))
        .unwrap();

    pool.load(PlatformKind::GitHub).await.unwrap();

    pool.mark_exhausted("t1").await.unwrap();

    let t = pool.acquire(PlatformKind::GitHub).await;
    assert!(t.is_none());
}

#[tokio::test]
async fn test_fgat_pool_pick_best_token() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));

    // Add three tokens with different remaining quotas.
    let mut t1 = make_token("high", "token_high", 4500);
    let mut t2 = make_token("mid", "token_mid", 2500);
    let mut t3 = make_token("low", "token_low", 500);
    // Stagger timestamps for predictable ordering.
    t1.added_at = "2025-01-01T00:00:00Z".to_string();
    t2.added_at = "2025-01-01T00:00:01Z".to_string();
    t3.added_at = "2025-01-01T00:00:02Z".to_string();

    {
        let g = store.lock().await;
        g.add_fgat_token(&t3).unwrap();
        g.add_fgat_token(&t2).unwrap();
        g.add_fgat_token(&t1).unwrap();
    }

    pool.load(PlatformKind::GitHub).await.unwrap();

    // First acquire should return the highest-remaining token.
    let token = pool.acquire(PlatformKind::GitHub).await.unwrap();
    assert_eq!(token.raw, "token_high");
    pool.release(token).await;

    // Mark the high token as exhausted.
    pool.mark_exhausted("high").await.unwrap();

    // Next acquire should return the next best (mid).
    let token = pool.acquire(PlatformKind::GitHub).await.unwrap();
    assert_eq!(token.raw, "token_mid");
    pool.release(token).await;

    // Mark mid as exhausted too.
    pool.mark_exhausted("mid").await.unwrap();

    // Last acquire should return the lowest.
    let token = pool.acquire(PlatformKind::GitHub).await.unwrap();
    assert_eq!(token.raw, "token_low");
    pool.release(token).await;
}

#[tokio::test]
async fn test_fgat_pool_empty() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));
    pool.load(PlatformKind::GitHub).await.unwrap();

    let t = pool.acquire(PlatformKind::GitHub).await;
    assert!(t.is_none());
}

#[tokio::test]
async fn test_bfs_traverser_basic() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));

    store
        .lock()
        .await
        .add_seed(&make_seed("seed-1", "testuser", 0, 1))
        .unwrap();

    let mock = MockClient::new(
        vec!["follower1".to_string(), "follower2".to_string()],
        vec!["followee1".to_string()],
    );

    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);
    let traverser = BfsTraverser::new(store.clone(), client.clone(), pool.clone());

    let report = traverser.run().await.unwrap();

    assert_eq!(report.seeds_completed, 1);
    assert_eq!(report.edges_discovered, 3);
    assert_eq!(report.errors, 0);

    let edges = store
        .lock()
        .await
        .traversal_edges_by_seed("seed-1")
        .unwrap();
    assert_eq!(edges.len(), 3);

    let updated = store.lock().await.get_seed("seed-1").unwrap().unwrap();
    assert_eq!(updated.status, SeedStatus::Completed);
}

#[tokio::test]
async fn test_bfs_traverser_no_duplicate_edges() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));

    store
        .lock()
        .await
        .add_seed(&make_seed("seed-1", "testuser", 0, 1))
        .unwrap();

    let mock = MockClient::new(vec!["user_a".to_string()], vec!["user_a".to_string()]);

    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);
    let traverser = BfsTraverser::new(store.clone(), client.clone(), pool.clone());

    let report = traverser.run().await.unwrap();
    assert_eq!(report.edges_discovered, 2);

    let report2 = traverser.run().await.unwrap();
    assert_eq!(report2.edges_discovered, 0);
}

#[tokio::test]
async fn test_bfs_no_seeds() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));

    let mock = MockClient::new(vec![], vec![]);
    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);
    let traverser = BfsTraverser::new(store, client, pool);

    let report = traverser.run().await.unwrap();
    assert_eq!(report.seeds_completed, 0);
    assert_eq!(report.edges_discovered, 0);
}

#[tokio::test]
async fn test_graph_store_seed_crud() {
    let store = make_store();
    let g = store.lock().await;

    let seed = make_seed("s1", "alice", 0, 2);
    g.add_seed(&seed).unwrap();

    let loaded = g.get_seed("s1").unwrap().unwrap();
    assert_eq!(loaded.username, "alice");
    assert_eq!(loaded.status, SeedStatus::Pending);

    g.update_seed_status("s1", "active").unwrap();
    let loaded = g.get_seed("s1").unwrap().unwrap();
    assert_eq!(loaded.status, SeedStatus::Active);

    let seeds = g.list_seeds().unwrap();
    assert_eq!(seeds.len(), 1);
}

#[tokio::test]
async fn test_graph_store_traversal_edges() {
    let store = make_store();
    let g = store.lock().await;

    // Add seed first (FK reference for traversal_edges.seed_id)
    let seed = make_seed("s1", "alice", 0, 2);
    g.add_seed(&seed).unwrap();

    let edge = rq_core::TraversalEdge {
        id: 0,
        seed_id: "s1".to_string(),
        from_user_id: "alice".to_string(),
        to_user_id: "bob".to_string(),
        relation_type: rq_core::EdgeType::Follows,
        depth: 1,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    };
    g.add_traversal_edge(&edge).unwrap();

    let edges = g.traversal_edges_by_seed("s1").unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to_user_id, "bob");

    let user_edges = g.traversal_edges_by_user("alice").unwrap();
    assert_eq!(user_edges.len(), 1);

    let visited = g.has_user_been_visited("s1", "bob", 2).unwrap();
    assert!(visited);

    let not_visited = g.has_user_been_visited("s1", "charlie", 2).unwrap();
    assert!(!not_visited);
}

#[tokio::test]
async fn test_graph_store_fgat_tokens() {
    let store = make_store();
    let g = store.lock().await;

    let token = make_token("t1", "ghp_abc123", 5000);
    g.add_fgat_token(&token).unwrap();

    let tokens = g.list_fgat_tokens().unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_hash, "ghp_abc123");

    g.update_fgat_token_status("t1", "exhausted").unwrap();
    let tokens = g.list_fgat_tokens().unwrap();
    assert_eq!(tokens[0].status, rq_core::TokenStatus::Exhausted);
}

#[tokio::test]
async fn test_graph_store_domain_identity() {
    let store = make_store();
    let g = store.lock().await;

    let domain = rq_core::NormalizedDomain {
        domain: "example.com".to_string(),
        canonical_domain: "example.com".to_string(),
        platform: Some(PlatformKind::GitHub),
        is_verified: true,
        confidence_score: 0.95,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    g.add_domain(&domain).unwrap();

    let loaded = g.get_domain("example.com").unwrap().unwrap();
    assert_eq!(loaded.canonical_domain, "example.com");
    assert!(loaded.is_verified);

    let identity = rq_core::UnifiedIdentity {
        id: "u1".to_string(),
        canonical_username: "alice".to_string(),
        primary_domain: "example.com".to_string(),
        confidence_score: 0.9,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    g.add_unified_identity(&identity).unwrap();

    let alias = rq_core::IdentityAlias {
        unified_id: "u1".to_string(),
        platform: PlatformKind::GitHub,
        username: "alice_dev".to_string(),
        verified: true,
        verification_method: Some("domain_check".to_string()),
    };
    g.add_identity_alias(&alias).unwrap();

    let found = g.find_identity_by_alias("github", "alice_dev").unwrap();
    assert_eq!(found, Some("u1".to_string()));

    let repo = rq_core::DomainRepository {
        id: "dr1".to_string(),
        domain: "example.com".to_string(),
        repo_path: "alice/awesome".to_string(),
        full_name: "alice/awesome".to_string(),
        platform: PlatformKind::GitHub,
        metadata_json: "{}".to_string(),
        indexed_at: chrono::Utc::now().to_rfc3339(),
    };
    g.add_domain_repository(&repo).unwrap();

    let repos = g.domain_repositories_by_domain("example.com").unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].full_name, "alice/awesome");
}

#[tokio::test]
async fn test_bfs_seed_propagation() {
    let store = make_store();
    let pool = Arc::new(FgatPool::new(store.clone()));

    // Seed at depth 0, max_depth 2 — should propagate to depth 1.
    store
        .lock()
        .await
        .add_seed(&make_seed("github-testuser", "testuser", 0, 2))
        .unwrap();

    let mock = MockClient::new(
        vec!["follower1".to_string(), "follower2".to_string()],
        vec!["followee1".to_string()],
    );

    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);
    let traverser = BfsTraverser::new(store.clone(), client.clone(), pool.clone());

    let report = traverser.run().await.unwrap();
    assert_eq!(report.seeds_completed, 1);
    assert_eq!(report.edges_discovered, 3);

    // After run, follower1, follower2, followee1 should be registered as seeds at depth 1.
    let g = store.lock().await;
    let all_seeds = g.list_seeds().unwrap();
    assert_eq!(all_seeds.len(), 4); // original + 3 propagated

    let follower1 = g.get_seed("github-follower1").unwrap().unwrap();
    assert_eq!(follower1.status, SeedStatus::Pending);
    assert_eq!(follower1.depth, 1);
    assert_eq!(follower1.max_depth, 2);

    let followee1 = g.get_seed("github-followee1").unwrap().unwrap();
    assert_eq!(followee1.status, SeedStatus::Pending);
    assert_eq!(followee1.depth, 1);
}

#[tokio::test]
async fn test_trust_scorer_basic() {
    let store = make_store();
    let g = store.lock().await;

    // Add a seed and some edges.
    let seed = make_seed("github-alice", "alice", 0, 2);
    g.add_seed(&seed).unwrap();

    // Alice follows bob (depth 1).
    g.add_traversal_edge(&rq_core::TraversalEdge {
        id: 0,
        seed_id: "github-alice".to_string(),
        from_user_id: "alice".to_string(),
        to_user_id: "bob".to_string(),
        relation_type: rq_core::EdgeType::Follows,
        depth: 1,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    })
    .unwrap();

    // Alice also follows charlie (depth 1).
    g.add_traversal_edge(&rq_core::TraversalEdge {
        id: 0,
        seed_id: "github-alice".to_string(),
        from_user_id: "alice".to_string(),
        to_user_id: "charlie".to_string(),
        relation_type: rq_core::EdgeType::Follows,
        depth: 1,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    })
    .unwrap();

    drop(g);

    let config = rq_expander::TrustConfig::default();
    let entries = TrustScorer::compute(&*store.lock().await, &config).unwrap();

    // Should have 3 entries: alice, bob, charlie
    assert_eq!(entries.len(), 3);

    // Alice (seed) should have the highest score.
    let alice = entries.iter().find(|e| e.username == "alice").unwrap();
    let bob = entries.iter().find(|e| e.username == "bob").unwrap();
    let charlie = entries.iter().find(|e| e.username == "charlie").unwrap();

    assert!(alice.score > bob.score);
    assert!(alice.score > charlie.score);

    // Bob and charlie should have similar scores (same depth from seed).
    let diff = (bob.score - charlie.score).abs();
    assert!(diff < 0.01);
}

#[tokio::test]
async fn test_trust_scorer_decay() {
    let store = make_store();
    let g = store.lock().await;

    let seed = make_seed("github-alice", "alice", 0, 3);
    g.add_seed(&seed).unwrap();

    // Alice → bob (depth 1), bob → charlie (depth 2)
    g.add_traversal_edge(&rq_core::TraversalEdge {
        id: 0,
        seed_id: "github-alice".to_string(),
        from_user_id: "alice".to_string(),
        to_user_id: "bob".to_string(),
        relation_type: rq_core::EdgeType::Follows,
        depth: 1,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    })
    .unwrap();

    g.add_traversal_edge(&rq_core::TraversalEdge {
        id: 0,
        seed_id: "github-alice".to_string(),
        from_user_id: "bob".to_string(),
        to_user_id: "charlie".to_string(),
        relation_type: rq_core::EdgeType::Follows,
        depth: 2,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    })
    .unwrap();

    drop(g);

    let config = rq_expander::TrustConfig {
        decay_factor: 0.5,
        ..Default::default()
    };
    let entries = TrustScorer::compute(&*store.lock().await, &config).unwrap();

    let alice = entries.iter().find(|e| e.username == "alice").unwrap();
    let bob = entries.iter().find(|e| e.username == "bob").unwrap();
    let charlie = entries.iter().find(|e| e.username == "charlie").unwrap();

    // alice > bob > charlie (each hop decays)
    assert!(alice.score > bob.score);
    assert!(bob.score > charlie.score);

    // bob is at depth 1, charlie at depth 2.
    assert_eq!(bob.depth, 1);
    assert_eq!(charlie.depth, 2);
}

#[tokio::test]
async fn test_trust_scorer_empty() {
    let store = make_store();
    let config = rq_expander::TrustConfig::default();
    let entries = TrustScorer::compute(&*store.lock().await, &config).unwrap();
    assert!(entries.is_empty());
}

// ── RepoCollector tests ──────────────────────────────────────────────────────────

fn make_repo_store() -> Arc<Mutex<dyn RepoStore + Send>> {
    let store = SqliteStore::new(std::path::Path::new(":memory:")).unwrap();
    Arc::new(Mutex::new(store))
}

fn make_trust_entry(username: &str, depth: u32, score: f64) -> TrustEntry {
    TrustEntry {
        user_id: format!("github-{}", username),
        username: username.to_string(),
        depth,
        score,
    }
}

fn make_mock_repo(name: &str, owner: &str, stars: u32, fork: bool) -> PlatformRepo {
    PlatformRepo {
        name: name.to_string(),
        owner: owner.to_string(),
        full_name: format!("{}/{}", owner, name),
        description: Some(format!("The {} project", name)),
        language: Some("Rust".to_string()),
        stars,
        topics: vec!["rust".to_string(), "cli".to_string()],
        license: Some("MIT".to_string()),
        is_archived: false,
        fork,
        html_url: format!("https://github.com/{}/{}", owner, name),
    }
}

#[tokio::test]
async fn test_collector_basic() {
    let repo_store = make_repo_store();
    let mut mock = MockClient::new(vec![], vec![]);
    mock.repos = vec![
        make_mock_repo("repo-a", "alice", 100, false),
        make_mock_repo("repo-b", "alice", 50, false),
    ];
    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);

    let collector = RepoCollector::new(repo_store.clone(), client);
    let users = vec![make_trust_entry("alice", 0, 1.0)];

    let report = collector.collect(&users, 0.5, "test").await.unwrap();
    assert_eq!(report.users_processed, 1);
    assert_eq!(report.repos_collected, 2);
    assert_eq!(report.repos_merged, 2);
    assert_eq!(report.errors, 0);

    let guard = repo_store.lock().await;
    let repo = guard.get_repo("github-com-alice-repo-a").unwrap().unwrap();
    assert_eq!(repo.metadata.full_name, "alice/repo-a");
    assert_eq!(repo.discovered_via.as_deref(), Some("test"));
}

#[tokio::test]
async fn test_collector_filters_by_trust_threshold() {
    let repo_store = make_repo_store();
    let mut mock = MockClient::new(vec![], vec![]);
    mock.repos = vec![make_mock_repo("repo-a", "alice", 100, false)];
    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);

    let collector = RepoCollector::new(repo_store.clone(), client);
    let users = vec![
        make_trust_entry("alice", 1, 0.3),
        make_trust_entry("bob", 2, 0.05), // below threshold
        make_trust_entry("charlie", 1, 0.6),
    ];

    let report = collector.collect(&users, 0.1, "test").await.unwrap();
    assert_eq!(report.users_processed, 2); // alice and charlie
    assert_eq!(report.errors, 0);

    let guard = repo_store.lock().await;
    // alice's repo should exist
    let found = guard.get_repo("github-com-alice-repo-a").unwrap();
    assert!(found.is_some());
}

#[tokio::test]
async fn test_collector_skips_forks() {
    let repo_store = make_repo_store();
    let mut mock = MockClient::new(vec![], vec![]);
    mock.repos = vec![
        make_mock_repo("original", "alice", 200, false),
        make_mock_repo("forked", "alice", 5, true), // fork — should be skipped
    ];
    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);

    let collector = RepoCollector::new(repo_store.clone(), client);
    let users = vec![make_trust_entry("alice", 0, 1.0)];

    let report = collector.collect(&users, 0.5, "test").await.unwrap();
    assert_eq!(report.repos_collected, 1); // only non-fork
    assert_eq!(report.repos_merged, 1);

    let guard = repo_store.lock().await;
    let forked = guard.get_repo("github-com-alice-forked").unwrap();
    assert!(forked.is_none());
}

#[tokio::test]
async fn test_collector_no_users_meet_threshold() {
    let repo_store = make_repo_store();
    let mock = MockClient::new(vec![], vec![]);
    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);

    let collector = RepoCollector::new(repo_store.clone(), client);
    let users = vec![make_trust_entry("alice", 2, 0.01)];

    let report = collector.collect(&users, 0.5, "test").await.unwrap();
    assert_eq!(report.users_processed, 0);
    assert_eq!(report.repos_collected, 0);
}

#[tokio::test]
async fn test_collector_empty_users() {
    let repo_store = make_repo_store();
    let mock = MockClient::new(vec![], vec![]);
    let client: Arc<dyn PlatformApiClient + Send + Sync> = Arc::new(mock);

    let collector = RepoCollector::new(repo_store.clone(), client);
    let report = collector.collect(&[], 0.5, "test").await.unwrap();
    assert_eq!(report.users_processed, 0);
    assert_eq!(report.repos_collected, 0);
}
