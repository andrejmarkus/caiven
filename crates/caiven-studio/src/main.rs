mod app;
mod debugger;
mod port_api;
mod port_client;
mod studio;
mod tauri_app;

fn main() -> anyhow::Result<()> {
    app::run()
}
