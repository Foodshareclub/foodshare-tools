//! Deploy translation system commands
//!
//! Replaces the shell scripts:
//! - deploy-llm-translation.sh
//! - deploy-translation-system.sh

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::process::Command;

/// Run the deploy command
pub async fn run(
    apply_migrations: bool,
    deploy_functions: bool,
    test_endpoints: bool,
    format: &str,
) -> Result<()> {
    if format == "json" {
        return run_json(apply_migrations, deploy_functions, test_endpoints).await;
    }

    println!("{}", "Deploying Translation System".bold().cyan());
    println!("{}", "=".repeat(40).dimmed());
    println!();

    // Step 1: Check environment
    println!("{}", "Step 1: Checking environment...".bold());
    check_environment()?;
    println!("  {} Environment validated", "✓".green());
    println!();

    // Step 2: Apply database migrations
    if apply_migrations {
        println!("{}", "Step 2: Applying database migrations...".bold());
        apply_db_migrations()?;
        println!("  {} Database migrations applied", "✓".green());
        println!();
    } else {
        println!(
            "{}",
            "Step 2: Skipping database migrations (--no-migrations)".dimmed()
        );
        println!();
    }

    // Step 3: Deploy edge functions
    if deploy_functions {
        println!("{}", "Step 3: Deploying edge functions...".bold());
        deploy_edge_functions()?;
        println!("  {} Edge functions deployed", "✓".green());
        println!();
    } else {
        println!(
            "{}",
            "Step 3: Skipping edge function deployment (--no-functions)".dimmed()
        );
        println!();
    }

    // Step 4: Test deployment
    if test_endpoints {
        println!("{}", "Step 4: Testing deployment...".bold());
        test_deployment().await?;
        println!();
    } else {
        println!(
            "{}",
            "Step 4: Skipping endpoint tests (--no-test)".dimmed()
        );
        println!();
    }

    println!("{}", "Deployment complete!".bold().green());
    println!();
    println!("{}", "Next steps:".bold());
    println!(
        "  1. Test with CLI: {}",
        "foodshare-i18n test-translation --locale ru".cyan()
    );
    println!(
        "  2. View status:   {}",
        "foodshare-i18n status".cyan()
    );
    println!();

    Ok(())
}

/// Run in JSON output mode
async fn run_json(
    apply_migrations: bool,
    deploy_functions: bool,
    test_endpoints: bool,
) -> Result<()> {
    let mut results = serde_json::json!({
        "success": true,
        "steps": {}
    });

    // Check environment
    let env_ok = check_environment().is_ok();
    results["steps"]["environment"] = serde_json::json!({
        "success": env_ok,
        "message": if env_ok { "Environment validated" } else { "Environment check failed" }
    });

    if !env_ok {
        results["success"] = serde_json::json!(false);
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }

    // Apply migrations
    if apply_migrations {
        let migrations_ok = apply_db_migrations().is_ok();
        results["steps"]["migrations"] = serde_json::json!({
            "success": migrations_ok,
            "skipped": false
        });
        if !migrations_ok {
            results["success"] = serde_json::json!(false);
        }
    } else {
        results["steps"]["migrations"] = serde_json::json!({
            "skipped": true
        });
    }

    // Deploy functions
    if deploy_functions {
        let deploy_ok = deploy_edge_functions().is_ok();
        results["steps"]["functions"] = serde_json::json!({
            "success": deploy_ok,
            "skipped": false
        });
        if !deploy_ok {
            results["success"] = serde_json::json!(false);
        }
    } else {
        results["steps"]["functions"] = serde_json::json!({
            "skipped": true
        });
    }

    // Test endpoints
    if test_endpoints {
        let test_ok = test_deployment().await.is_ok();
        results["steps"]["tests"] = serde_json::json!({
            "success": test_ok,
            "skipped": false
        });
        if !test_ok {
            results["success"] = serde_json::json!(false);
        }
    } else {
        results["steps"]["tests"] = serde_json::json!({
            "skipped": true
        });
    }

    println!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

/// Check required environment variables
fn check_environment() -> Result<()> {
    let required_vars = [
        ("SUPABASE_URL", "Supabase project URL"),
        ("SUPABASE_SERVICE_ROLE_KEY", "Supabase service role key"),
    ];

    let optional_vars = [
        ("LLM_TRANSLATION_ENDPOINT", "LLM translation API endpoint"),
        ("LLM_TRANSLATION_API_KEY", "LLM translation API key"),
        ("UPSTASH_REDIS_URL", "Redis URL for caching"),
        ("UPSTASH_REDIS_TOKEN", "Redis auth token"),
    ];

    let mut missing = Vec::new();

    for (var, desc) in &required_vars {
        if std::env::var(var).is_err() {
            missing.push((*var, *desc));
        } else {
            println!("  {} {} is set", "✓".green(), var);
        }
    }

    for (var, desc) in &optional_vars {
        if std::env::var(var).is_err() {
            println!("  {} {} not set (optional: {})", "⚠".yellow(), var, desc);
        } else {
            println!("  {} {} is set", "✓".green(), var);
        }
    }

    if !missing.is_empty() {
        eprintln!();
        eprintln!("{}", "Missing required environment variables:".red().bold());
        for (var, desc) in &missing {
            eprintln!("  {} - {}", var.red(), desc);
        }
        eprintln!();
        eprintln!("Set them in your shell or .env file:");
        eprintln!("  export SUPABASE_URL='https://your-project.supabase.co'");
        eprintln!("  export SUPABASE_SERVICE_ROLE_KEY='your-service-role-key'");
        anyhow::bail!("Missing required environment variables");
    }

    Ok(())
}

/// Apply database migrations using Supabase CLI
fn apply_db_migrations() -> Result<()> {
    let output = Command::new("bunx")
        .args(["supabase", "db", "push"])
        .current_dir(find_supabase_dir()?)
        .output()
        .context("Failed to run supabase db push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("  {} Migration failed:", "✗".red());
        eprintln!("{}", stderr);
        anyhow::bail!("Database migration failed");
    }

    Ok(())
}

/// Deploy edge functions
fn deploy_edge_functions() -> Result<()> {
    let functions = ["localization", "bff"];
    let supabase_dir = find_supabase_dir()?;

    for func in functions {
        println!("  Deploying {}...", func.cyan());

        let output = Command::new("bunx")
            .args(["supabase", "functions", "deploy", func, "--no-verify-jwt"])
            .current_dir(&supabase_dir)
            .output()
            .with_context(|| format!("Failed to deploy {}", func))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("  {} Failed to deploy {}:", "✗".red(), func);
            eprintln!("{}", stderr);
            anyhow::bail!("Failed to deploy {}", func);
        }

        println!("    {} {} deployed", "✓".green(), func);
    }

    Ok(())
}

/// Test deployment endpoints
async fn test_deployment() -> Result<()> {
    let base_url =
        std::env::var("SUPABASE_URL").context("SUPABASE_URL not set")?;
    let service_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
        .context("SUPABASE_SERVICE_ROLE_KEY not set")?;

    let client = reqwest::Client::new();

    // Test localization endpoint
    println!("  Testing /localization...");
    let resp = client
        .get(format!("{}/functions/v1/localization", base_url))
        .header("Authorization", format!("Bearer {}", service_key))
        .send()
        .await
        .context("Failed to connect to localization endpoint")?;

    if resp.status().is_success() {
        println!("    {} Localization endpoint responding", "✓".green());
    } else {
        println!(
            "    {} Localization returned HTTP {}",
            "✗".red(),
            resp.status()
        );
    }

    // Test BFF endpoint
    println!("  Testing /bff...");
    let resp = client
        .get(format!("{}/functions/v1/bff", base_url))
        .header("Authorization", format!("Bearer {}", service_key))
        .send()
        .await
        .context("Failed to connect to BFF endpoint")?;

    if resp.status().is_success() {
        println!("    {} BFF endpoint responding", "✓".green());
    } else {
        println!("    {} BFF returned HTTP {}", "✗".red(), resp.status());
    }

    Ok(())
}

/// Find the supabase directory
fn find_supabase_dir() -> Result<std::path::PathBuf> {
    // Try environment variable first
    if let Ok(dir) = std::env::var("FOODSHARE_BACKEND_DIR") {
        let path = std::path::PathBuf::from(dir);
        if path.join("supabase/config.toml").exists() {
            return Ok(path);
        }
    }

    // Try current directory
    let current = std::env::current_dir()?;

    // Check if we're in foodshare-backend
    if current.join("supabase/config.toml").exists() {
        return Ok(current);
    }

    // Check parent directories
    for ancestor in current.ancestors().skip(1) {
        if ancestor.join("supabase/config.toml").exists() {
            return Ok(ancestor.to_path_buf());
        }
    }

    // Check common locations relative to home
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        let common_paths = [
            home.join("dev/work/foodshare/foodshare-backend"),
            home.join("foodshare/foodshare-backend"),
            home.join("projects/foodshare-backend"),
        ];

        for path in &common_paths {
            if path.join("supabase/config.toml").exists() {
                return Ok(path.clone());
            }
        }
    }

    anyhow::bail!(
        "Could not find foodshare-backend directory. Run from the project root or set FOODSHARE_BACKEND_DIR"
    )
}
