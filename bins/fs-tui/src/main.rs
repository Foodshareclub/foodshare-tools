//! FoodShare TUI — live mono-ecosystem build/test monitor
//!
//! The TUI is a front-end over `foodshare-orchestrator`: the same
//! implementation the CLI and the HTTP daemon drive. It shows the five
//! verification domains and their live state, and starts them on demand.
//!
//! - Rust 2024 (cargo test --workspace, bench)
//! - WASM (wasm-pack 5 crates)
//! - Web (Next.js Turbopack, tsrs, oxlint/biome)
//! - Backend (deno edge function tests)
//! - Mobile (Skip transpile, Maestro, Gradle)
//!
//! Run: `cargo run --bin fs-tui` or `cargo run --bin fs-tui -- verify`

use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::Backend,
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use foodshare_orchestrator::{DomainKind, Orchestrator, Status, Workspace, disk};

/// How often the screen is redrawn.
const TICK: Duration = Duration::from_millis(200);

/// How long the keyboard is polled for between redraws.
const POLL: Duration = Duration::from_millis(50);

#[derive(Parser)]
#[command(name = "fs-tui", version, about = "FoodShare live build/test TUI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run full verify (Rust→WASM→Web→Backend→Mobile)
    Verify,
    /// Run Rust workspace tests
    Rust,
    /// Run WASM build
    Wasm,
    /// Run Web checks
    Web,
    /// Run Backend Deno tests
    Backend,
    /// Run Mobile Maestro syntax + Gradle
    Mobile,
}

impl Commands {
    /// The domain the command runs, or `None` for the whole plan.
    fn domain(self) -> Option<DomainKind> {
        match self {
            Self::Verify => None,
            Self::Rust => Some(DomainKind::Rust),
            Self::Wasm => Some(DomainKind::Wasm),
            Self::Web => Some(DomainKind::Web),
            Self::Backend => Some(DomainKind::Backend),
            Self::Mobile => Some(DomainKind::Mobile),
        }
    }
}

/// The TUI's own state.
struct App {
    orchestrator: Orchestrator,
    selected: usize,
    started: Option<Instant>,
    message: String,
    should_quit: bool,
    /// Whether a run is in progress, so a second one is not started.
    running: bool,
}

impl App {
    fn new(orchestrator: Orchestrator) -> Self {
        Self {
            orchestrator,
            selected: 0,
            started: None,
            message: "q:quit  r:run domain  v:verify all  c:clean disk  h:help".into(),
            should_quit: false,
            running: false,
        }
    }

    fn domains() -> [DomainKind; 5] {
        DomainKind::ALL
    }

    fn selected_domain(&self) -> DomainKind {
        Self::domains()[self.selected]
    }

    fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('h') => {
                self.message =
                    "h:help  q:quit  r:run selected  v:verify all  c:clean disk  j/k:nav".into()
            }
            KeyCode::Char('r') => self.start_domain(self.selected_domain()),
            KeyCode::Char('v') => self.start_plan(),
            KeyCode::Char('c') => self.clean_disk(),
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1) % Self::domains().len();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = if self.selected == 0 {
                    Self::domains().len() - 1
                } else {
                    self.selected - 1
                };
            }
            _ => {}
        }
    }

    /// Start one domain, if nothing else is running.
    fn start_domain(&mut self, kind: DomainKind) {
        if self.running {
            self.message = format!("⚠ {} is already running", kind.label());
            return;
        }
        self.running = true;
        self.started = Some(Instant::now());
        self.message = format!("▶ running {}...", kind.label());

        let orchestrator = self.orchestrator.clone();
        tokio::spawn(async move {
            let outcome = orchestrator.run_domain(kind).await;
            match outcome {
                Ok(outcome) => tracing::info!(
                    domain = kind.name(),
                    success = outcome.success,
                    duration_ms = outcome.duration_ms,
                    "domain finished"
                ),
                Err(e) => tracing::error!(domain = kind.name(), error = %e, "domain failed"),
            }
        });
    }

    /// Start the whole plan.
    fn start_plan(&mut self) {
        if self.running {
            self.message = "⚠ a run is already in progress".into();
            return;
        }
        self.running = true;
        self.started = Some(Instant::now());
        self.message = "Running full verify — Rust→WASM→Web→Backend→Mobile".into();

        let orchestrator = self.orchestrator.clone();
        tokio::spawn(async move {
            let outcome = orchestrator.run_plan().await;
            match outcome {
                Ok(outcome) => tracing::info!(
                    success = outcome.success,
                    duration_ms = outcome.duration_ms,
                    "plan finished"
                ),
                Err(e) => tracing::error!(error = %e, "plan failed"),
            }
        });
    }

    /// Prune the build artefacts.
    fn clean_disk(&mut self) {
        let root = self.orchestrator.workspace().root().to_path_buf();
        let removed = disk::clean_artifacts(&root);
        self.message = if removed.is_empty() {
            "🧹 nothing to clean".into()
        } else {
            format!("🧹 cleaned {} artefact director(ies)", removed.len())
        };
    }

    /// Refresh the running flag from the orchestrator's state.
    fn on_tick(&mut self) {
        let snapshot = self.orchestrator.snapshot();
        if !snapshot.running && self.running {
            self.running = false;
            self.message = "✓ run finished".into();
        }
    }
}

fn status_style(status: Status) -> Style {
    let color = match status {
        Status::Idle => Color::DarkGray,
        Status::Running => Color::Yellow,
        Status::Pass => Color::Green,
        Status::Fail => Color::Red,
    };
    Style::default().fg(color)
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let title = Paragraph::new(
        "  FoodShare Mono-Ecosystem — Rust 2024 + Bun + Next.js 16 + Supabase (TUI)",
    )
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::default().borders(Borders::ALL).title(" fs-tui "));
    f.render_widget(title, chunks[0]);

    // Domains table
    let header = Row::new(vec![
        Cell::from(" Domain "),
        Cell::from(" Status "),
        Cell::from(" Detail "),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let snapshot = app.orchestrator.snapshot();
    let rows = App::domains().into_iter().enumerate().map(|(index, kind)| {
        let status = snapshot
            .domain(kind)
            .map(|d| d.status)
            .unwrap_or(Status::Idle);
        let style = if index == app.selected {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default()
        };
        Row::new(vec![
            Cell::from(format!(" {} {} ", kind.icon(), kind.label())),
            Cell::from(format!(" {} ", status.label())).style(status_style(status)),
            Cell::from(format!(" {} ", kind.detail())),
        ])
        .style(style)
        .height(1)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Min(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Domains (j/k select, r run, v verify) "),
    )
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(table, chunks[1]);

    // Logs for the selected domain
    let log_text = snapshot
        .domain(app.selected_domain())
        .map(|d| d.logs.join("\n"))
        .unwrap_or_default();
    let logs = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Logs: {} {} ",
            app.selected_domain().icon(),
            app.selected_domain().label()
        )))
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    f.render_widget(logs, chunks[2]);

    // Footer
    let elapsed = app
        .started
        .map(|s| format!("{:.1}s", s.elapsed().as_secs_f32()))
        .unwrap_or_else(|| "-".into());
    let footer = Paragraph::new(format!(
        " {} | elapsed: {} | {}",
        app.message,
        elapsed,
        app.selected_domain().detail()
    ))
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::ALL).title(" Help "));
    f.render_widget(footer, chunks[3]);
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let mut tick = tokio::time::interval(TICK);

    loop {
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
            _ = tick.tick() => {
                app.on_tick();
            }
            maybe_event = async {
                if event::poll(POLL).unwrap_or(false) {
                    Some(event::read())
                } else {
                    None
                }
            } => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    app.on_key(key.code);
                    if app.should_quit {
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let workspace = Workspace::discover()?;
    let orchestrator = Orchestrator::new(workspace);

    // Non-interactive mode: run one domain, or the whole plan.
    if let Some(command) = cli.command {
        match command.domain() {
            Some(kind) => {
                println!("fs-tui: running {}", kind.label());
                let outcome = orchestrator.run_domain(kind).await?;
                println!(
                    "{} {} ({}ms)",
                    outcome.kind.label(),
                    if outcome.success { "PASS" } else { "FAIL" },
                    outcome.duration_ms
                );
                std::process::exit(if outcome.success { 0 } else { 1 });
            }
            None => {
                println!("fs-tui: verifying every domain");
                let outcome = orchestrator.run_plan().await?;
                print_outcome(&outcome);
                std::process::exit(if outcome.success { 0 } else { 1 });
            }
        }
    }

    // Interactive TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(orchestrator);
    let res = run_app(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("tui error: {e:?}");
    }
    Ok(())
}

/// Print what a run did.
fn print_outcome(outcome: &foodshare_orchestrator::PlanOutcome) {
    for domain in &outcome.domains {
        println!(
            "{} {} ({}ms)",
            domain.kind.label(),
            if domain.success { "PASS" } else { "FAIL" },
            domain.duration_ms
        );
    }
    if outcome.success {
        println!("🎉 all green in {}ms", outcome.duration_ms);
    } else {
        println!("❌ failed after {}ms", outcome.duration_ms);
    }
}
