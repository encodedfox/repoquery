Test case demonstrating manual repo setup and MockClient behavior

#[test]
fn test_manual_repo_setup() {
    let repos = vec![
        PlatformRepo {
            name: "repo1".to_string(),
            description: "Repo 1".to_string(),
            language: "Rust".to_string(),
            topics: vec!["rust", "cli"],
            stars: 1000,
            forks: 500,
            watchers: 200,
            created_at: "2023-05-01T00:00:00Z".to_string(),
            updated_at: "2023-05-01T00:00:00Z".to_string(),
            pushed_at: "2023-05-01T00:00:00Z".to_string(),
        },
        PlatformRepo {
            name: "repo2".to_string(),
            description: "Repo 2".to_string(),
            language: "Python".to_string(),
            topics: vec!["python", "data"],
            stars: 500,
            forks: 200,
            watchers: 100,
            created_at: "2023-05-01T00:00:00Z".to_string(),
            updated_at: "2023-05-01T00:00:00Z".to_string(),
            pushed_at: "2023-05-01T00:00:00Z".to_string(),
        },
    ];
    let mock_client = MockClient::new(repos);
    // Test mock client behavior with manual repo setup
}