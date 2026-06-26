use clap::Subcommand;

pub mod languages;
pub mod list;
pub mod search;
pub mod show;
pub mod topics;

#[derive(Subcommand)]
pub enum QueryAction {
    /// List repositories with optional filters
    List(list::ListArgs),
    /// Full-text search across repository names, descriptions, and topics
    Search(search::SearchArgs),
    /// Show detailed information about a specific repository
    Show(show::ShowArgs),
    /// List all unique topics with repository counts
    Topics(topics::TopicsArgs),
    /// List all unique languages with repository counts
    Languages(languages::LanguagesArgs),
}
