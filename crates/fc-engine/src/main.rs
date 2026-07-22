mod app;
mod debugger;
mod hub_client;
mod studio;

fn main() -> anyhow::Result<()> {
    app::run()
}
