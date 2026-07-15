mod app;
mod debugger;
mod hot_reload;
mod hub_client;
mod studio;

fn main() -> anyhow::Result<()> {
    app::run()
}
