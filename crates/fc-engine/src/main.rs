mod app;
mod cart_save;
mod debugger;
mod editors;
mod hot_reload;
mod tabs;

fn main() -> anyhow::Result<()> {
    app::run()
}
