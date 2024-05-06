use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn copy_to_stdout(author: String, from: quinn::RecvStream) -> anyhow::Result<()> {
    let mut lines = BufReader::new(from).lines();
    while let Some(line) = lines.next_line().await? {
        tracing::info!("read line: {}", line);
        println!("{}> {}\n", author, line);
    }
    Ok(())
}

pub async fn copy_stdin_to(mut to: quinn::SendStream) -> anyhow::Result<()> {
    let from = tokio::io::stdin();
    let mut lines = BufReader::new(from).lines();
    while let Some(line) = lines.next_line().await? {
        tracing::info!("read line: {}", line);
        to.write_all(format!("{}\n", line).as_bytes()).await?;
    }
    Ok(())
}
