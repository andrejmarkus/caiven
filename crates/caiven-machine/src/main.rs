mod app;
mod data_dir;
mod platform;
mod port_client;
mod port_worker;
mod shell;

fn main() -> anyhow::Result<()> {
    app::run()
}
