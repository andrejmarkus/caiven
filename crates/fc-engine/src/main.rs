mod app;
mod debugger;
mod hot_reload;
mod studio;

fn main() -> anyhow::Result<()> {
    app::run()
}
