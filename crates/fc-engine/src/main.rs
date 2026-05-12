mod app;
mod cart_save;
mod debugger;
mod editors;
mod hot_reload;

fn main() -> anyhow::Result<()> {
    app::run()
}
