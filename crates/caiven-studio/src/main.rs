mod app;
mod debugger;
mod port_client;
mod studio;

fn main() -> anyhow::Result<()> {
    app::run()
}
