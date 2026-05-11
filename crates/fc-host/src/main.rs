mod app;
mod assembler;
mod debugger;
mod input;
mod rendering;
mod settings;
mod vm;

fn main() -> anyhow::Result<()> {
    app::run()
}
