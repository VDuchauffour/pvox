use metron::tui::Tui;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let _tui = Tui::new()?;

    tokio::signal::ctrl_c().await?;

    Ok(())
}
