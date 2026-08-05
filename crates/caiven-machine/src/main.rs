mod app;
mod platform;
mod port_client;
mod shell;

fn main() -> anyhow::Result<()> {
    app::run()
}
