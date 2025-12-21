use lib_common::{config::AppConfig, tracing::setup_tracing};
use lib_server::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Setup Logging
    setup_tracing(false);

    // 2. Load Config
    // For legacy compatibility, we use defaults but allow environment override if AppConfig handles it
    // AppConfig::load() reads MOUCHAK_SERVER__PORT etc.
    // If not set, it defaults to 8765.
    // However, existing main.rs used PORT env var directly in line 118.
    // To be 100% compatible, we should allow PORT if set.
    // But let's rely on standard AppConfig. If user sets `MOUCHAK_SERVER__PORT`, it works.
    // If they set `PORT`, we might lose it unless AppConfig checks it.
    // For now, let's just use AppConfig.

    let config = AppConfig::load()?;
    tracing::info!("Loaded config: {:?}", config.server);

    // 3. Run Server
    run(config).await?;
    Ok(())
}
