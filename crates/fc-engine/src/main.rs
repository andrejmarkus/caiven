mod app;
mod debugger;
mod editors;
mod hot_reload;
mod studio;
mod tabs;

fn main() -> anyhow::Result<()> {
    app::run()
}
