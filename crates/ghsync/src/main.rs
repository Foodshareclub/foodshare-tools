//! ghsync — entry point.
//!
//! The command logic lives in [`ghsync::cli`]; this binary only owns the
//! async runtime and the exit code.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    ghsync::cli::run(std::env::args().collect()).await
}
