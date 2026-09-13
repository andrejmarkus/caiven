mod app;
mod platform;
mod port_client;
mod port_worker;
mod shell;

fn main() -> anyhow::Result<()> {
    app::run()
}
