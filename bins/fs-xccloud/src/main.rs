//! fs-xccloud — the Xcode Cloud log streamer and diagnostics.
//!
//! Replaces the Bun script that used to drive the App Store Connect API.
//!
//! Usage:
//!   fs-xccloud status              # the recent build runs
//!   fs-xccloud start [workflowId]   # queue a build and watch it
//!   fs-xccloud watch [buildId]      # watch a build
//!   fs-xccloud logs [buildId]       # the actions, issues and artefacts
//!   fs-xccloud jwt                  # print a signed token
//!   fs-xccloud local                # the local build summaries

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};

use foodshare_xccloud::{api, auth, diagnostics};

/// How long to wait between polls when watching a build.
const POLL: Duration = Duration::from_secs(8);

/// How many build runs to show.
const RUNS_TO_SHOW: usize = 5;

#[derive(Parser, Debug)]
#[command(
    name = "fs-xccloud",
    version,
    about = "FoodShare Xcode Cloud log streamer"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,

    /// The app repository. Defaults to the directory that holds
    /// `foodshare-app`.
    #[arg(long, global = true)]
    app_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Action {
    /// The recent build runs.
    Status,
    /// The recent build runs.
    List,
    /// Queue a build run, and watch it.
    Start {
        /// The workflow to run. Defaults to the product's first workflow.
        workflow_id: Option<String>,
    },
    /// Watch a build run until it finishes.
    Watch {
        /// The build run. Defaults to the most recent one.
        build_id: Option<String>,
    },
    /// The actions, issues and artefacts of a build run.
    Logs {
        /// The build run. Defaults to the most recent one.
        build_id: Option<String>,
    },
    /// Print a signed token.
    Jwt,
    /// The local build summaries.
    Local,
    /// The local build summaries.
    Diagnostics,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let app_dir = resolve_app_dir(cli.app_dir.as_deref());
    let locations = auth::default_locations(&app_dir);

    match cli.command.unwrap_or(Action::Status) {
        Action::Jwt => {
            let credentials = require_credentials(&locations)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            println!(
                "{}",
                auth::create_token(&credentials, now)
                    .map_err(|e| anyhow::anyhow!("could not sign the token: {e}"))?
            );
        }
        Action::Status | Action::List => {
            let Some(credentials) = auth::resolve(&locations) else {
                no_credentials();
                show_local(&app_dir);
                return Ok(());
            };
            let client = api::Client::new(&credentials)?;
            show_build_runs(&client).await?;
        }
        Action::Start { workflow_id } => {
            let credentials = require_credentials(&locations)?;
            let client = api::Client::new(&credentials)?;
            let workflow_id = match workflow_id {
                Some(id) => id,
                None => first_workflow(&client).await?,
            };
            println!("🚀 Triggering a new Xcode Cloud build for workflow: {workflow_id}...");
            let run = client.trigger_build(&workflow_id).await?;
            println!(
                "✅ Build #{} successfully queued! (ID: {})",
                run.data.attributes.number.unwrap_or(0),
                run.data.id
            );
            println!("👀 Starting the live watch...\n");
            watch(&client, &run.data.id).await?;
        }
        Action::Watch { build_id } => {
            let credentials = require_credentials(&locations)?;
            let client = api::Client::new(&credentials)?;
            let build_id = match build_id {
                Some(id) => id,
                None => latest_build(&client).await?,
            };
            watch(&client, &build_id).await?;
        }
        Action::Logs { build_id } => {
            let credentials = require_credentials(&locations)?;
            let client = api::Client::new(&credentials)?;
            let build_id = match build_id {
                Some(id) => id,
                None => latest_build(&client).await?,
            };
            show_logs(&client, &build_id).await?;
        }
        Action::Local | Action::Diagnostics => show_local(&app_dir),
    }

    Ok(())
}

/// The credentials, or an explanation of how to set them.
fn require_credentials(locations: &auth::Locations) -> Result<auth::Credentials> {
    auth::resolve(locations).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not resolve the App Store Connect credentials.\n\
             Set APP_STORE_CONNECT_KEY_ID, APP_STORE_CONNECT_ISSUER_ID and \
             APP_STORE_CONNECT_PRIVATE_KEY, or place AuthKey_<KEY_ID>.p8 in \
             ~/.appstoreconnect/private_keys/ or in the app's keys/ directory."
        )
    })
}

/// Say what is missing, and how to provide it.
fn no_credentials() {
    println!("⚠️ App Store Connect credentials not detected in the environment.");
    println!("ℹ️ To enable live API streaming, set the following environment variables:");
    println!("   • APP_STORE_CONNECT_KEY_ID");
    println!("   • APP_STORE_CONNECT_ISSUER_ID");
    println!(
        "   • APP_STORE_CONNECT_PRIVATE_KEY (or place AuthKey_<KEY_ID>.p8 in ~/.appstoreconnect/private_keys/)"
    );
}

/// The recent build runs, and the issues of the ones that failed.
async fn show_build_runs(client: &api::Client) -> Result<()> {
    println!("☁️ Connecting to the App Store Connect Xcode Cloud API...");

    let products = client.products().await?;
    if products.data.is_empty() {
        println!("ℹ️ No Xcode Cloud products found for this App Store Connect team.");
        return Ok(());
    }

    let names: Vec<&str> = products
        .data
        .iter()
        .filter_map(|p| p.attributes.name.as_deref())
        .collect();
    println!(
        "📦 Found {} Xcode Cloud Product(s): {}",
        products.data.len(),
        names.join(", ")
    );

    for product in &products.data {
        let Some(name) = product.attributes.name.as_deref() else {
            continue;
        };
        // The team has other products; this tool is about FoodShare.
        if !name.to_lowercase().contains("foodshare") {
            continue;
        }

        println!("\n🔍 Fetching the recent build runs for: {name}...");
        let runs = client.build_runs(&product.id, RUNS_TO_SHOW).await?;
        println!(
            "\n📋 Recent Xcode Cloud Build Runs (Found: {}):",
            runs.data.len()
        );
        println!(
            "================================================================================"
        );

        for run in &runs.data {
            let attributes = &run.attributes;
            let workflow_id = run
                .relationships
                .as_ref()
                .and_then(|r| r.workflow.as_ref())
                .and_then(|w| w.data.as_ref())
                .map(|d| d.id.clone());
            let workflow_name = workflow_id
                .and_then(|id| {
                    runs.included
                        .iter()
                        .find(|i| i.kind == "ciWorkflows" && i.id == id)
                })
                .and_then(|i| i.name().map(str::to_string))
                .unwrap_or_else(|| "Workflow".to_string());

            println!(
                "{} Build #{} [{}]",
                attributes.glyph(),
                attributes.number.unwrap_or(0),
                workflow_name
            );
            println!("   Status: {}", attributes.status());
            println!("   ID: {}", run.id);
            if let Some(commit) = &attributes.source_commit {
                if let Some(sha) = &commit.commit_sha {
                    let short: String = sha.chars().take(7).collect();
                    let message = commit.message.as_deref().unwrap_or_default().trim();
                    println!("   Commit: {short} (\"{message}\")");
                }
            }
            println!(
                "   Started: {}",
                attributes.created_date.as_deref().unwrap_or("N/A")
            );
            if let Some(finished) = &attributes.finished_date {
                println!("   Finished: {finished}");
            }

            if matches!(
                attributes.completion_status.as_deref(),
                Some("FAILED") | Some("ERRORED")
            ) {
                if let Ok(issues) = client.build_issues(&run.id).await {
                    if !issues.data.is_empty() {
                        println!("   ⚠️ Issues ({}):", issues.data.len());
                        for issue in issues.data.iter().take(5) {
                            print_issue(&issue.attributes);
                        }
                    }
                }
            }
            println!(
                "--------------------------------------------------------------------------------"
            );
        }
    }
    Ok(())
}

/// Print one issue.
fn print_issue(issue: &foodshare_xccloud::IssueAttributes) {
    println!(
        "      • [{}] {}",
        issue.issue_type.as_deref().unwrap_or("Error"),
        issue.message.as_deref().unwrap_or_default()
    );
    if let Some(source) = &issue.file_source {
        if let Some(path) = &source.path {
            let line = source
                .line_number
                .map(|n| n.to_string())
                .unwrap_or_default();
            println!("        File: {path}:{line}");
        }
    }
}

/// The actions, issues and artefacts of a build run.
async fn show_logs(client: &api::Client, build_id: &str) -> Result<()> {
    println!("📥 Fetching the build actions and logs for Build ID: {build_id}...");

    let actions = client.actions(build_id).await?;
    println!("📋 Found {} Action(s):", actions.data.len());

    for action in &actions.data {
        let attributes = &action.attributes;
        let name = attributes
            .name
            .as_deref()
            .or(attributes.action_type.as_deref())
            .unwrap_or("Action");
        println!(
            "\n🔹 Action: {name} [{}]",
            attributes.action_type.as_deref().unwrap_or("unknown")
        );
        println!("   Status: {}", attributes.status());
        if let Some(started) = &attributes.started_date {
            println!("   Started: {started}");
        }
        if let Some(finished) = &attributes.finished_date {
            println!("   Finished: {finished}");
        }

        if let Ok(issues) = client.action_issues(&action.id).await {
            if !issues.data.is_empty() {
                println!("\n   ⚠️ Action Issues ({}):", issues.data.len());
                for issue in &issues.data {
                    print_issue(&issue.attributes);
                }
            }
        }

        if let Ok(artifacts) = client.artifacts(&action.id).await {
            if !artifacts.data.is_empty() {
                println!("\n   📦 Artifacts ({}):", artifacts.data.len());
                for artifact in &artifacts.data {
                    println!(
                        "      • {} ({})",
                        artifact
                            .attributes
                            .file_name
                            .as_deref()
                            .unwrap_or("unnamed"),
                        artifact
                            .attributes
                            .file_type
                            .as_deref()
                            .unwrap_or("unknown")
                    );
                    if let Some(url) = &artifact.attributes.download_url {
                        println!("        Download URL: {url}");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Watch a build run until it finishes.
async fn watch(client: &api::Client, build_id: &str) -> Result<()> {
    println!("📡 Watching Xcode Cloud Build ID: {build_id}...");
    let mut last_progress = String::new();

    loop {
        let run = client.build_run(build_id).await?;
        let attributes = &run.data.attributes;
        let progress = attributes
            .execution_progress
            .as_deref()
            .unwrap_or("UNKNOWN");
        let status = attributes
            .completion_status
            .as_deref()
            .unwrap_or("IN PROGRESS");

        if progress != last_progress || status != "IN PROGRESS" {
            println!(
                "[{}] 🔄 Build #{} - Progress: {progress} | Status: {status}",
                now_label(),
                attributes.number.unwrap_or(0)
            );
            last_progress = progress.to_string();
        }

        if status != "IN PROGRESS" && (progress == "COMPLETE" || attributes.finished_date.is_some())
        {
            println!("\n🏁 Build finished with status: {status}");
            if status == "SUCCEEDED" {
                println!("🎉 Xcode Cloud build succeeded!");
            } else {
                println!("❌ Build failed. Fetching the action issues...");
                show_logs(client, build_id).await?;
            }
            return Ok(());
        }

        tokio::time::sleep(POLL).await;
    }
}

/// The most recent build run of the FoodShare product.
async fn latest_build(client: &api::Client) -> Result<String> {
    let product = client
        .find_product("foodshare")
        .await?
        .ok_or_else(|| anyhow::anyhow!("No FoodShare Xcode Cloud product was found."))?;
    client
        .latest_build(&product.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No build run found."))
}

/// The first workflow of the FoodShare product.
async fn first_workflow(client: &api::Client) -> Result<String> {
    let products = client.products().await?;
    let product = products
        .data
        .iter()
        .find(|p| p.attributes.name.as_deref() == Some("FoodShare App"))
        .or_else(|| products.data.first())
        .ok_or_else(|| anyhow::anyhow!("No Xcode Cloud product was found."))?;

    let workflows = client.workflows(&product.id).await?;
    workflows
        .data
        .first()
        .map(|w| w.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Could not find an active Xcode Cloud workflow to trigger."))
}

/// The local build summaries.
fn show_local(app_dir: &std::path::Path) {
    let summaries = diagnostics::read_summaries(app_dir, 3, 1500);
    if summaries.is_empty() {
        println!("ℹ️ No local build summaries found in logs/xcodecloud/.");
        return;
    }
    println!(
        "\n📂 Found {} local Xcode Cloud build summaries:",
        summaries.len()
    );
    for (path, contents) in summaries {
        println!("\n--- [{}] ---", path.display());
        println!("{contents}");
    }
}

/// The current time, for the watch log.
fn now_label() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

/// Find the app repository.
fn resolve_app_dir(explicit: Option<&std::path::Path>) -> PathBuf {
    if let Some(dir) = explicit {
        return dir.to_path_buf();
    }
    let candidates = [
        std::env::current_dir().ok(),
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join("foodshare-app")),
    ];
    for candidate in candidates.into_iter().flatten() {
        if candidate
            .join("Darwin")
            .join("FoodShare.xcodeproj")
            .is_dir()
        {
            return candidate;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
