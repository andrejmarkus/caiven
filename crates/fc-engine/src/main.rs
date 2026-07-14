mod app;
mod debugger;
mod editors;
mod hot_reload;
mod tabs;

fn main() -> anyhow::Result<()> {
    app::run()
}
