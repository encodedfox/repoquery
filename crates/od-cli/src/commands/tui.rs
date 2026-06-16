//! Interactive TUI for browsing repositories.

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use od_core::{Collection, Relation, Repository};
use ratatui::{prelude::*, widgets::*};
use std::path::PathBuf;

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum View {
    RepoList,
    RepoDetail,
    Collections,
    CollectionDetail,
    Stats,
}

struct App {
    repos: Vec<Repository>,
    filtered: Vec<usize>, // indices into repos
    collections: Vec<Collection>,
    view: View,
    selected: usize,
    search_query: String,
    search_mode: bool,
    filter_language: Option<String>,
    filter_relation: Option<Relation>,
    filter_tag: Option<String>,
    should_quit: bool,
    status_message: String,
}

impl App {
    fn new(repos: Vec<Repository>, collections: Vec<Collection>) -> Self {
        let count = repos.len();
        Self {
            repos,
            filtered: (0..count).collect(),
            collections,
            view: View::RepoList,
            selected: 0,
            search_query: String::new(),
            search_mode: false,
            filter_language: None,
            filter_relation: None,
            filter_tag: None,
            should_quit: false,
            status_message: String::new(),
        }
    }

    fn list_len(&self) -> usize {
        match self.view {
            View::RepoList | View::RepoDetail => self.filtered.len(),
            View::Collections | View::CollectionDetail => self.collections.len(),
            View::Stats => 0,
        }
    }

    fn next(&mut self) {
        let len = self.list_len();
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    fn prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn first(&mut self) {
        self.selected = 0;
    }

    fn last(&mut self) {
        let len = self.list_len();
        if len > 0 {
            self.selected = len - 1;
        }
    }

    fn enter_detail(&mut self) {
        match self.view {
            View::RepoList => self.view = View::RepoDetail,
            View::Collections => self.view = View::CollectionDetail,
            _ => {}
        }
    }

    fn back(&mut self) {
        match self.view {
            View::RepoDetail => self.view = View::RepoList,
            View::CollectionDetail => self.view = View::Collections,
            _ => {}
        }
    }

    fn apply_filter(&mut self) {
        let q = self.search_query.to_lowercase();
        self.filtered = self
            .repos
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                let name_match = q.is_empty()
                    || r.metadata.full_name.to_lowercase().contains(&q)
                    || r.metadata.description.to_lowercase().contains(&q);
                let lang_match = self
                    .filter_language
                    .as_ref()
                    .is_none_or(|l| r.classification.language_category == *l);
                let rel_match = self
                    .filter_relation
                    .as_ref()
                    .is_none_or(|rel| r.relations.contains(rel));
                let tag_match = self
                    .filter_tag
                    .as_ref()
                    .is_none_or(|t| r.custom_tags.contains(t));
                name_match && lang_match && rel_match && tag_match
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    fn cycle_language_filter(&mut self) {
        let mut langs: Vec<String> = self
            .repos
            .iter()
            .map(|r| r.classification.language_category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        langs.sort();

        self.filter_language = match &self.filter_language {
            None if !langs.is_empty() => Some(langs[0].clone()),
            Some(cur) => {
                let idx = langs.iter().position(|l| l == cur).unwrap_or(0);
                langs.get(idx + 1).cloned()
            }
            _ => None,
        };
        self.apply_filter();
    }

    fn cycle_relation_filter(&mut self) {
        const RELS: [Relation; 5] = [
            Relation::Starred,
            Relation::Owned,
            Relation::Forked,
            Relation::Watching,
            Relation::OrgMember,
        ];
        self.filter_relation = match &self.filter_relation {
            None => Some(RELS[0].clone()),
            Some(cur) => {
                let idx = RELS.iter().position(|r| r == cur).unwrap_or(0);
                RELS.get(idx + 1).cloned()
            }
        };
        self.apply_filter();
    }

    fn cycle_tag_filter(&mut self) {
        let mut tags: Vec<String> = self
            .repos
            .iter()
            .flat_map(|r| r.custom_tags.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        tags.sort();

        self.filter_tag = match &self.filter_tag {
            None if !tags.is_empty() => Some(tags[0].clone()),
            Some(cur) => {
                let idx = tags.iter().position(|t| t == cur).unwrap_or(0);
                tags.get(idx + 1).cloned()
            }
            _ => None,
        };
        self.apply_filter();
    }

    fn selected_repo(&self) -> Option<&Repository> {
        self.filtered.get(self.selected).and_then(|&i| self.repos.get(i))
    }
}

// ── Key handling ──────────────────────────────────────────────────────────────

fn handle_key(app: &mut App, key: KeyEvent) {
    if app.search_mode {
        match key.code {
            KeyCode::Esc => app.search_mode = false,
            KeyCode::Enter => {
                app.search_mode = false;
                app.apply_filter();
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.apply_filter();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.apply_filter();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.prev(),
        KeyCode::Char('g') => app.first(),
        KeyCode::Char('G') => app.last(),
        KeyCode::Char('/') => {
            app.search_mode = true;
            app.search_query.clear();
        }
        KeyCode::Enter => app.enter_detail(),
        KeyCode::Esc | KeyCode::Backspace => app.back(),
        KeyCode::Char('1') => {
            app.view = View::RepoList;
            app.selected = 0;
        }
        KeyCode::Char('2') => {
            app.view = View::Collections;
            app.selected = 0;
        }
        KeyCode::Char('3') => {
            app.view = View::Stats;
            app.selected = 0;
        }
        KeyCode::Char('l') => app.cycle_language_filter(),
        KeyCode::Char('r') => app.cycle_relation_filter(),
        KeyCode::Char('t') => app.cycle_tag_filter(),
        KeyCode::Char('a') if app.view == View::CollectionDetail => {
            app.status_message =
                "Use CLI: collections add --collection <id> --repo <id>".to_string();
        }
        _ => {}
    }
}

// ── UI rendering ──────────────────────────────────────────────────────────────

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(f.area());

    draw_header(f, app, chunks[0]);

    match app.view {
        View::RepoList => draw_repo_list(f, app, chunks[1]),
        View::RepoDetail => draw_repo_detail(f, app, chunks[1]),
        View::Collections => draw_collections(f, app, chunks[1]),
        View::CollectionDetail => draw_collection_detail(f, app, chunks[1]),
        View::Stats => draw_stats(f, app, chunks[1]),
    }

    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mut title = format!(" OmniDatum — {} repositories", app.filtered.len());
    if let Some(lang) = &app.filter_language {
        title.push_str(&format!(" [lang: {lang}]"));
    }
    if let Some(rel) = &app.filter_relation {
        title.push_str(&format!(" [rel: {rel}]"));
    }
    if let Some(tag) = &app.filter_tag {
        title.push_str(&format!(" [tag: {tag}]"));
    }
    if app.search_mode || !app.search_query.is_empty() {
        title.push_str(&format!(" [search: {}]", app.search_query));
    }
    let block = Block::bordered().title(title);
    f.render_widget(block, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let help = if app.search_mode {
        " Type to search | Enter: confirm | Esc: cancel"
    } else {
        " q:quit  j/k:nav  g/G:top/bot  /:search  Enter:detail  Esc:back  1-3:views  l:lang  r:rel  t:tag"
    };
    let text = if app.status_message.is_empty() {
        help.to_string()
    } else {
        app.status_message.clone()
    };
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn draw_repo_list(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(["Name", "Language", "Stars", "Relation", "Status"])
        .style(Style::default().bold());

    let rows: Vec<Row> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(display_idx, &repo_idx)| {
            let r = &app.repos[repo_idx];
            let relation = r
                .relations
                .first()
                .map(|rel| rel.to_string())
                .unwrap_or_default();
            let status = if r.quality_metrics.archive_status {
                "archived"
            } else {
                "active"
            };
            let style = if display_idx == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new([
                r.metadata.full_name.clone(),
                r.metadata.primary_language.clone(),
                r.metadata.stars.to_string(),
                relation,
                status.to_string(),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(20),
        Constraint::Percentage(10),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::bordered().title(" Repositories "));
    f.render_widget(table, area);
}

fn draw_repo_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(r) = app.selected_repo() else {
        f.render_widget(Paragraph::new("No repository selected."), area);
        return;
    };

    let url = r.primary_url().unwrap_or("—");
    let license = r.metadata.license.as_deref().unwrap_or("—");
    let relations: Vec<String> = r.relations.iter().map(|rel| rel.to_string()).collect();
    let topics = r.metadata.topics.join(", ");
    let fork_info = r
        .fork_parent
        .as_deref()
        .map(|p| format!("Fork of: {p}"))
        .unwrap_or_default();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Name:     ", Style::default().bold()),
            Span::raw(&r.metadata.full_name),
        ]),
        Line::from(vec![
            Span::styled("URL:      ", Style::default().bold()),
            Span::raw(url),
        ]),
        Line::from(vec![
            Span::styled("Desc:     ", Style::default().bold()),
            Span::raw(&r.metadata.description),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Language: ", Style::default().bold()),
            Span::raw(&r.metadata.primary_language),
        ]),
        Line::from(vec![
            Span::styled("Stars:    ", Style::default().bold()),
            Span::raw(r.metadata.stars.to_string()),
        ]),
        Line::from(vec![
            Span::styled("License:  ", Style::default().bold()),
            Span::raw(license),
        ]),
        Line::from(vec![
            Span::styled("Relations:", Style::default().bold()),
            Span::raw(format!(" {}", relations.join(", "))),
        ]),
        Line::from(vec![
            Span::styled("Quality:  ", Style::default().bold()),
            Span::raw(format!("{}/100", r.quality_metrics.quality_score)),
        ]),
        Line::from(vec![
            Span::styled("Archived: ", Style::default().bold()),
            Span::raw(r.quality_metrics.archive_status.to_string()),
        ]),
    ];

    if !topics.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Topics:   ", Style::default().bold()),
            Span::raw(&topics),
        ]));
    }
    if !fork_info.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Fork:     ", Style::default().bold()),
            Span::raw(&fork_info),
        ]));
        if let Some(ahead) = r.fork_ahead {
            lines.push(Line::from(vec![
                Span::styled("Ahead:    ", Style::default().bold()),
                Span::raw(ahead.to_string()),
            ]));
        }
        if let Some(behind) = r.fork_behind {
            lines.push(Line::from(vec![
                Span::styled("Behind:   ", Style::default().bold()),
                Span::raw(behind.to_string()),
            ]));
        }
    }
    if !r.custom_tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tags:     ", Style::default().bold()),
            Span::raw(r.custom_tags.join(", ")),
        ]));
    }
    if let Some(notes) = &r.curator_notes {
        lines.push(Line::from(vec![
            Span::styled("Notes:    ", Style::default().bold()),
            Span::raw(notes.as_str()),
        ]));
    }

    let para = Paragraph::new(lines)
        .block(Block::bordered().title(format!(" {} ", r.metadata.full_name)))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn draw_collections(f: &mut Frame, app: &App, area: Rect) {
    let rows: Vec<Row> = app
        .collections
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new([c.name.clone(), c.repo_ids.len().to_string()]).style(style)
        })
        .collect();

    let widths = [Constraint::Percentage(80), Constraint::Percentage(20)];
    let table = Table::new(rows, widths)
        .header(Row::new(["Collection", "Repos"]).style(Style::default().bold()))
        .block(Block::bordered().title(" Collections "));
    f.render_widget(table, area);
}

fn draw_collection_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(col) = app.collections.get(app.selected) else {
        f.render_widget(Paragraph::new("No collection selected."), area);
        return;
    };

    let items: Vec<ListItem> = col
        .repo_ids
        .iter()
        .map(|id| ListItem::new(id.as_str()))
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title(format!(" {} ({} repos) ", col.name, col.repo_ids.len())));
    f.render_widget(list, area);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let total = app.repos.len();
    let archived = app.repos.iter().filter(|r| r.quality_metrics.archive_status).count();
    let active = total - archived;

    // Language counts
    let mut lang_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for r in &app.repos {
        *lang_counts.entry(r.classification.language_category.as_str()).or_default() += 1;
    }
    let mut lang_vec: Vec<(&str, usize)> = lang_counts.into_iter().collect();
    lang_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // Relation counts
    let mut rel_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for r in &app.repos {
        for rel in &r.relations {
            *rel_counts.entry(rel.to_string()).or_default() += 1;
        }
    }

    // Average quality
    let avg_quality = if total > 0 {
        app.repos.iter().map(|r| r.quality_metrics.quality_score as usize).sum::<usize>() / total
    } else {
        0
    };

    let mut lines = vec![
        Line::from(vec![Span::styled("Overview", Style::default().bold())]),
        Line::from(format!("  Total:    {total}")),
        Line::from(format!("  Active:   {active}")),
        Line::from(format!("  Archived: {archived}")),
        Line::from(format!("  Avg quality score: {avg_quality}/100")),
        Line::from(""),
        Line::from(vec![Span::styled("Top Languages", Style::default().bold())]),
    ];

    for (lang, count) in lang_vec.iter().take(10) {
        lines.push(Line::from(format!("  {lang:<20} {count}")));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("By Relation", Style::default().bold())]));
    let mut rel_vec: Vec<(String, usize)> = rel_counts.into_iter().collect();
    rel_vec.sort_by(|a, b| b.1.cmp(&a.1));
    for (rel, count) in &rel_vec {
        lines.push(Line::from(format!("  {rel:<20} {count}")));
    }

    let para = Paragraph::new(lines).block(Block::bordered().title(" Statistics "));
    f.render_widget(para, area);
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(store_path: PathBuf) -> anyhow::Result<()> {
    let store = od_store::open_store(&store_path)?;
    let data = store.load_all()?;
    let collections = store.list_collections()?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new(data.repositories, collections);

    let result = run_loop(&mut terminal, &mut app);

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen
    )?;

    result
}

fn run_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
