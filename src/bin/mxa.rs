#[tokio::main]
async fn main() -> anyhow::Result<()> {
    return mxlite::agent::cli::main().await;
}
