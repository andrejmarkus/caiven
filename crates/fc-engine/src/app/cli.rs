//! Command-line interface: argument parsing, headless subcommands
//! (build/inspect/publish) and the `run` entry point that starts the editor.

use super::App;
use crate::hub_client::{build_multipart, capture_screenshot};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fc_rom::{RomHeader, SectionKind};
use fc_vm::VmConfig;
use log::info;
use std::path::{Path, PathBuf};
use winit::event_loop::{ControlFlow, EventLoop};

#[derive(Parser)]
#[command(
    name = "fc-engine",
    about = "Fantasy Console — development environment"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Enable debugger overlay
    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Assemble source and write a ROM file
    Build {
        /// Path to the .asm source file
        source: PathBuf,
        /// Output .rom path
        output: PathBuf,
    },
    /// Inspect a ROM file and print its section table
    Inspect {
        /// Path to the .rom file
        rom: PathBuf,
    },
    /// Run a .asm or .rom file
    Run {
        /// Path to .asm source (hot reload) or .rom file
        file: PathBuf,
    },
    /// Open FC Studio, the desktop editor suite
    Edit {
        /// Optional .rom or .fc file to open
        file: Option<PathBuf>,
    },
    /// Publish a .rom file to a cart sharing hub
    Publish {
        /// Path to the .rom file
        rom: PathBuf,
        /// Hub base URL
        #[arg(long, env = "FC_HUB_URL", default_value = "http://localhost:8080")]
        url: String,
        /// Per-user hub API token (create one via the hub web UI Profile
        /// page, or by logging into FC Studio's hub tab)
        #[arg(long, env = "FC_HUB_API_KEY", default_value = "")]
        api_key: String,
        /// Cart title (defaults to ROM header title)
        #[arg(long)]
        title: Option<String>,
        /// Author name (defaults to ROM header author)
        #[arg(long)]
        author: Option<String>,
        /// Short description
        #[arg(long, default_value = "")]
        description: String,
        /// Comma-separated tags
        #[arg(long, default_value = "")]
        tags: String,
        /// Frames to run before capturing screenshot
        #[arg(long, default_value_t = 30)]
        frames: u32,
        /// Skip screenshot capture and upload
        #[arg(long)]
        no_screenshot: bool,
    },
}

struct PublishArgs<'a> {
    rom_path: &'a Path,
    hub_url: &'a str,
    api_key: &'a str,
    title: Option<&'a str>,
    author: Option<&'a str>,
    description: &'a str,
    tags: &'a str,
    frames: u32,
    no_screenshot: bool,
}

fn publish_cart(args: PublishArgs) -> Result<()> {
    let PublishArgs {
        rom_path,
        hub_url,
        api_key,
        title,
        author,
        description,
        tags,
        frames,
        no_screenshot,
    } = args;
    if api_key.is_empty() {
        anyhow::bail!(
            "no hub API token given — pass --api-key or set FC_HUB_API_KEY \
             (create one via the hub web UI Profile page, or by logging into FC Studio's hub tab)"
        );
    }
    let rom = fc_rom::load(rom_path)
        .with_context(|| format!("failed to load ROM from {}", rom_path.display()))?;

    let title = title.unwrap_or(&rom.header.title);
    let author = author.unwrap_or(&rom.header.author);
    let tags_vec: Vec<&str> = tags
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let meta_str = serde_json::json!({
        "title": title,
        "author": author,
        "description": description,
        "tags": tags_vec,
    })
    .to_string();

    let rom_bytes = std::fs::read(rom_path)
        .with_context(|| format!("failed to read ROM bytes from {}", rom_path.display()))?;

    let boundary = "----FcHubBoundary7x3k9p";
    let filename = rom_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cart.rom");

    let body = build_multipart(
        boundary,
        &[
            ("meta", None, "application/json", meta_str.as_bytes()),
            (
                "rom",
                Some(filename),
                "application/octet-stream",
                &rom_bytes,
            ),
        ],
    );

    let content_type = format!("multipart/form-data; boundary={boundary}");
    let upload_url = format!("{hub_url}/api/carts");

    let response = ureq::post(&upload_url)
        .set("X-Api-Key", api_key)
        .set("Content-Type", &content_type)
        .send_bytes(&body)
        .context("failed to upload cart")?;

    let cart_id: String = {
        let val: serde_json::Value = serde_json::from_reader(response.into_reader())
            .context("failed to parse upload response")?;
        val["id"]
            .as_str()
            .context("upload response missing 'id'")?
            .to_string()
    };

    println!("published: {hub_url}/api/carts/{cart_id}");

    if !no_screenshot {
        let config = VmConfig::default();
        let png_bytes = capture_screenshot(&rom, config, frames)?;

        let boundary2 = "----FcHubScreenshotBoundary";
        let screenshot_body = build_multipart(
            boundary2,
            &[(
                "screenshot",
                Some("screenshot.png"),
                "image/png",
                &png_bytes,
            )],
        );
        let ct2 = format!("multipart/form-data; boundary={boundary2}");
        let screenshot_url = format!("{hub_url}/api/carts/{cart_id}/screenshot");

        ureq::post(&screenshot_url)
            .set("X-Api-Key", api_key)
            .set("Content-Type", &ct2)
            .send_bytes(&screenshot_body)
            .context("failed to upload screenshot")?;

        println!("screenshot uploaded");
    }

    Ok(())
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let command = cli.command;

    match &command {
        Some(Command::Build { source, output }) => {
            info!("building ROM: {} → {}", source.display(), output.display());

            let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "fc" {
                let src_text = std::fs::read_to_string(source)
                    .with_context(|| format!("failed to read source {}", source.display()))?;
                let (code, sections) = fc_rom::text::split_source(&src_text)
                    .map_err(|e| anyhow::anyhow!("bad asset block in {}: {e}", source.display()))?;
                let out = fc_lang::compile(&code).map_err(|e| {
                    anyhow::anyhow!(
                        "compile error in {}:\n{}",
                        source.display(),
                        e.render(&code)
                    )
                })?;
                let header = RomHeader::default_for(stem);
                fc_rom::write(output, &header, &out.program, &sections)
                    .with_context(|| format!("cannot write ROM to {}", output.display()))?;
                info!(
                    "ROM written to {} ({} asset sections)",
                    output.display(),
                    sections.len()
                );
                return Ok(());
            }

            let out = fc_asm::assemble_file_with_sections(source)
                .map_err(|e| anyhow::anyhow!("assembly failed: {e}"))?;

            let header = RomHeader::default_for(stem);

            let extra: Vec<(SectionKind, Vec<u8>)> = out
                .extra_sections
                .into_iter()
                .map(|(id, data)| (SectionKind::from_u16(id), data))
                .collect();

            fc_rom::write(output, &header, &out.program, &extra)
                .with_context(|| format!("cannot write ROM to {}", output.display()))?;

            info!(
                "ROM written to {} ({} extra sections)",
                output.display(),
                extra.len()
            );
            return Ok(());
        }
        Some(Command::Inspect { rom }) => {
            let loaded = fc_rom::load(rom)
                .with_context(|| format!("failed to load ROM from {}", rom.display()))?;
            println!("ROM: {}", rom.display());
            println!("  title:  {}", loaded.header.title);
            println!("  author: {}", loaded.header.author);
            println!("  program: {} bytes", loaded.program.len());
            println!("  sections ({}):", loaded.sections.len() + 1);
            println!("    [0] Program  {} bytes", loaded.program.len());
            for (i, s) in loaded.sections.iter().enumerate() {
                println!("    [{}] {:?}  {} bytes", i + 1, s.kind, s.data.len());
            }
            return Ok(());
        }
        Some(Command::Publish {
            rom,
            url,
            api_key,
            title,
            author,
            description,
            tags,
            frames,
            no_screenshot,
        }) => {
            publish_cart(PublishArgs {
                rom_path: rom,
                hub_url: url,
                api_key,
                title: title.as_deref(),
                author: author.as_deref(),
                description,
                tags,
                frames: *frames,
                no_screenshot: *no_screenshot,
            })?;
            return Ok(());
        }
        Some(Command::Edit { file }) => {
            return crate::studio::run_studio(file.clone());
        }
        None => {
            return crate::studio::run_studio(None);
        }
        _ => {}
    }

    let mut app = App::new()?;
    app.set_debug_enabled(cli.debug);

    match command {
        Some(Command::Run { file }) => {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "rom" {
                info!("running ROM: {}", file.display());
                app.load_rom(&file)?;
            } else if ext == "lua" {
                info!("running Lua source: {}", file.display());
                app.load_lua(&file)?;
            } else {
                info!("running source: {} (hot-reload active)", file.display());
                app.watch_source(file)?;
            }
        }
        None
        | Some(Command::Build { .. })
        | Some(Command::Inspect { .. })
        | Some(Command::Publish { .. })
        | Some(Command::Edit { .. }) => unreachable!(),
    }

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("event loop error")?;

    Ok(())
}
