mod app;
mod debugger;
mod input;
mod isa;
mod rendering;
mod settings;
mod timing;
mod vm;

fn main() -> anyhow::Result<()> {
    app::run()
}
