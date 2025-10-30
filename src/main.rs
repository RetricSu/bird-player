mod app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::bootstrap::start_app()?;
    Ok(())
}
