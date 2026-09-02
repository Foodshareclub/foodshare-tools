//! fs-auto — FoodShare headless auto-orchestrator
//! Run-until-green, build, auto-close & clean. Neat, robust, efficient.

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;

#[derive(Parser, Debug)]
#[command(
    name = "fs-auto",
    about = "FoodShare auto — run until green, build, close & clean"
)]
struct Cli {
    #[arg(long)]
    keep: bool,
    #[arg(long, default_value_t = 2)]
    retry: u32,
    #[arg(long)]
    clean: bool,
    #[arg(long)]
    quick: bool,
    #[arg(long)]
    sequential: bool,
}

fn workspace_root() -> PathBuf {
    let m = Path::new(env!("CARGO_MANIFEST_DIR"));
    m.join("../../..")
        .canonicalize()
        .unwrap_or(m.join("../../.."))
}

fn artifact_dirs(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("foodshare-tools/target"),
        root.join("foodshare-app/.build"),
        root.join("foodshare-app/Android/app/build"),
        root.join("foodshare-app/Android/.gradle"),
        root.join("foodshare-app/Android/.skip-stub"),
        root.join("foodshare-web/.next"),
        root.join("foodshare-web/.turbo"),
        root.join("foodshare-web/node_modules/.cache"),
        root.join("foodshare-backend/supabase/.temp"),
        root.join(".turbo"),
    ]
}

fn clean_artifacts(root: &Path) {
    println!("\n🧹 Cleaning artifacts...");
    let mut n = 0;
    for p in artifact_dirs(root) {
        if p.exists() {
            match std::fs::remove_dir_all(&p) {
                Ok(_) => {
                    println!(
                        "  ✓ removed {}",
                        p.strip_prefix(root).unwrap_or(&p).display()
                    );
                    n += 1;
                }
                Err(e) => eprintln!("  ⚠ {}: {e}", p.display()),
            }
        }
    }
    let marker = root.join("foodshare-app/Android/.skip-plugin-marker");
    if marker.exists() {
        let _ = std::fs::remove_file(&marker);
    }
    println!("  ✓ cleaned {n} dirs");
}

async fn run_cmd(label: &str, mut cmd: Command) -> Result<()> {
    println!("▶ {label}: {:?}", cmd.as_std().get_program());
    let out = cmd
        .output()
        .await
        .with_context(|| format!("spawn {label}"))?;
    if out.status.success() {
        println!("✅ {label} passed");
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    eprintln!("\n--- {label} stdout ---\n{stdout}\n--- stderr ---\n{stderr}");
    anyhow::bail!("{label} failed exit {:?}", out.status.code());
}

async fn run_with_retry<F, Fut>(label: &str, mut f: F, retries: u32) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    for attempt in 1..=retries + 1 {
        pb.set_message(format!("{label} attempt {attempt}/{} ", retries + 1));
        match f().await {
            Ok(_) => {
                pb.finish_with_message(format!("✅ {label} passed"));
                return Ok(());
            }
            Err(e) if attempt <= retries => {
                pb.set_message(format!("⚠ {label} failed: {e} — retrying..."));
                tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
            }
            Err(e) => {
                pb.finish_with_message(format!("❌ {label} failed"));
                return Err(e);
            }
        }
    }
    unreachable!()
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = workspace_root();
    println!("🤖 FoodShare Auto — run-until-green, build, auto-close & clean");
    println!("   root: {}", root.display());
    println!(
        "   keep={}, retry={}, clean={}, quick={}, sequential={}",
        cli.keep, cli.retry, cli.clean, cli.quick, cli.sequential
    );
    if let Ok(out) = Command::new("df").arg("-h").arg(&root).output().await {
        println!(
            "{}",
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
        );
        println!(
            "{}",
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .nth(1)
                .unwrap_or("")
        );
    }
    if cli.clean {
        clean_artifacts(&root);
    }
    let start = std::time::Instant::now();
    // Phase 1: Rust
    run_with_retry(
        "Rust (cargo test --workspace)",
        || {
            let r = root.clone();
            async move {
                let mut c = Command::new("cargo");
                c.arg("test")
                    .arg("--workspace")
                    .current_dir(r.join("foodshare-tools"));
                run_cmd("Rust", c).await
            }
        },
        cli.retry,
    )
    .await?;
    // Phase 2: WASM
    if cli.quick {
        println!("📦 WASM — quick, using cached .wasm");
    } else {
        run_with_retry(
            "WASM (wasm-pack 5 crates)",
            || {
                let r = root.clone();
                async move {
                    let mut c = Command::new("bun");
                    c.args(["tools/build-wasm.ts"])
                        .current_dir(r.join("foodshare-tools"));
                    run_cmd("WASM", c).await
                }
            },
            cli.retry,
        )
        .await?;
    }
    let web_root = root.join("foodshare-web");
    let be_root = root.join("foodshare-backend");
    if !cli.sequential {
        println!("⚡ Web + Backend (parallel)");
        let (r1, r2) = tokio::join!(
            run_with_retry(
                "Web type-check",
                || {
                    let p = web_root.clone();
                    async move {
                        let mut c = Command::new("bun");
                        c.args(["run", "type-check"]).current_dir(p);
                        run_cmd("Web type-check", c).await
                    }
                },
                cli.retry
            ),
            run_with_retry(
                "Backend (deno test)",
                || {
                    let p = be_root.clone();
                    async move {
                        let mut c = Command::new("deno");
                        c.args([
                            "test",
                            "--allow-all",
                            "--config",
                            "supabase/functions/deno.json",
                            "supabase/functions/__tests__/",
                        ])
                        .current_dir(p);
                        run_cmd("Backend", c).await
                    }
                },
                cli.retry
            )
        );
        r1?;
        r2?;
        run_with_retry(
            "Web build (Turbopack)",
            || {
                let p = web_root.clone();
                async move {
                    let mut c = Command::new("bun");
                    c.args(["run", "build"]).current_dir(p);
                    run_cmd("Web build", c).await
                }
            },
            cli.retry,
        )
        .await?;
    } else {
        run_with_retry(
            "Web type-check",
            || {
                let p = web_root.clone();
                async move {
                    let mut c = Command::new("bun");
                    c.args(["run", "type-check"]).current_dir(p);
                    run_cmd("Web type-check", c).await
                }
            },
            cli.retry,
        )
        .await?;
        run_with_retry(
            "Web build",
            || {
                let p = web_root.clone();
                async move {
                    let mut c = Command::new("bun");
                    c.args(["run", "build"]).current_dir(p);
                    run_cmd("Web build", c).await
                }
            },
            cli.retry,
        )
        .await?;
        run_with_retry(
            "Backend",
            || {
                let p = be_root.clone();
                async move {
                    let mut c = Command::new("deno");
                    c.args([
                        "test",
                        "--allow-all",
                        "--config",
                        "supabase/functions/deno.json",
                        "supabase/functions/__tests__/",
                    ])
                    .current_dir(p);
                    run_cmd("Backend", c).await
                }
            },
            cli.retry,
        )
        .await?;
    }
    let app_root = root.join("foodshare-app");
    run_with_retry(
        "Mobile Maestro syntax",
        || {
            let p = app_root.clone();
            async move {
                let mut c = Command::new("bun");
                c.args(["tools/maestro-runner.ts", "syntax"]).current_dir(p);
                run_cmd("Maestro syntax", c).await
            }
        },
        cli.retry,
    )
    .await?;
    run_with_retry(
        "Mobile Gradle :app:testDebugUnitTest",
        || {
            let p = app_root.join("Android");
            async move {
                let mut c = Command::new("./gradlew");
                c.args([
                    ":app:testDebugUnitTest",
                    "--build-cache",
                    "--parallel",
                    "--no-configuration-cache",
                ])
                .current_dir(p)
                .env(
                    "ANDROID_HOME",
                    std::env::var("ANDROID_HOME")
                        .unwrap_or("/Users/organic/Library/Android/sdk".into()),
                );
                run_cmd("Gradle", c).await
            }
        },
        cli.retry,
    )
    .await?;
    let wasm_ok = root
        .join("foodshare-web/src/wasm/foodshare-search/foodshare_search_bg.wasm")
        .exists();
    let next_ok = root.join("foodshare-web/.next").exists();
    println!(
        "\n🔨 Artifacts: WASM {}  Web .next {}",
        if wasm_ok { "✅" } else { "❌" },
        if next_ok { "✅" } else { "❌" }
    );
    let dur = start.elapsed().as_secs_f32();
    println!("\n==========================================");
    println!("🎉 AUTO SUCCESS — all green in {dur:.1}s");
    println!("==========================================");
    if !cli.keep {
        clean_artifacts(&root);
        println!("✨ Auto-closed & cleaned — repo neat");
    } else {
        println!("📦 Keeping artifacts (--keep)");
    }
    Ok(())
}
