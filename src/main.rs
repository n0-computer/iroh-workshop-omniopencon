#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // init logging. we can now configure the log level with the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();
    // Code goes here
    Ok(())
}
