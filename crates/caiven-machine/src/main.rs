mod app;
mod platform;
mod shell;

fn main() -> anyhow::Result<()> {
    app::run()
}
