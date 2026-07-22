//! Command-line interface: argument parsing, headless subcommands
//! (inspect/publish) and the `run` entry point that starts the editor.

use crate::port_client::{build_multipart, capture_screenshot};
use anyhow::{Context, Result};
use caiven_vm::VmConfig;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "caiven-studio", about = "Caiven — development environment")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Enable debug-level logging
    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect a cart file and print its section table
    Inspect {
        /// Path to the .cav file
        cart: PathBuf,
    },
    /// Open Caiven Studio, the desktop editor suite
    Edit {
        /// Optional .cav file to open
        file: Option<PathBuf>,
    },
    /// Publish a .cav file to a cart sharing port
    Publish {
        /// Path to the .cav file
        cart: PathBuf,
        /// Port base URL
        #[arg(long, env = "CAIVEN_PORT_URL", default_value = "http://localhost:8080")]
        url: String,
        /// Per-user port API token (create one via the port web UI Profile
        /// page, or by logging into Caiven Studio's port tab)
        #[arg(long, env = "CAIVEN_PORT_API_KEY", default_value = "")]
        api_key: String,
        /// Cart title (defaults to cart header title)
        #[arg(long)]
        title: Option<String>,
        /// Author name (defaults to cart header author)
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
    cart_path: &'a Path,
    port_url: &'a str,
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
        cart_path,
        port_url,
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
            "no port API token given — pass --api-key or set CAIVEN_PORT_API_KEY \
             (create one via the port web UI Profile page, or by logging into Caiven Studio's port tab)"
        );
    }
    let cart = caiven_cart::load(cart_path)
        .with_context(|| format!("failed to load cart from {}", cart_path.display()))?;

    let title = title.unwrap_or(&cart.header.title);
    let author = author.unwrap_or(&cart.header.author);
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

    let cart_bytes = std::fs::read(cart_path)
        .with_context(|| format!("failed to read cart bytes from {}", cart_path.display()))?;

    let boundary = "----CaivenPortBoundary7x3k9p";
    let filename = cart_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cart.cav");

    let body = build_multipart(
        boundary,
        &[
            ("meta", None, "application/json", meta_str.as_bytes()),
            (
                "cart",
                Some(filename),
                "application/octet-stream",
                &cart_bytes,
            ),
        ],
    );

    let content_type = format!("multipart/form-data; boundary={boundary}");
    let upload_url = format!("{port_url}/api/carts");

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

    println!("published: {port_url}/api/carts/{cart_id}");

    if !no_screenshot {
        let config = VmConfig::default();
        let png_bytes = capture_screenshot(&cart, config, frames)?;

        let boundary2 = "----CaivenPortScreenshotBoundary";
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
        let screenshot_url = format!("{port_url}/api/carts/{cart_id}/screenshot");

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
        Some(Command::Inspect { cart }) => {
            let loaded = caiven_cart::load(cart)
                .with_context(|| format!("failed to load cart from {}", cart.display()))?;
            println!("cart: {}", cart.display());
            println!("  title:  {}", loaded.header.title);
            println!("  author: {}", loaded.header.author);
            println!("  program: {} bytes", loaded.program.len());
            println!("  sections ({}):", loaded.sections.len() + 1);
            println!("    [0] Program  {} bytes", loaded.program.len());
            for (i, s) in loaded.sections.iter().enumerate() {
                println!("    [{}] {:?}  {} bytes", i + 1, s.kind, s.data.len());
            }
            Ok(())
        }
        Some(Command::Publish {
            cart,
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
                cart_path: cart,
                port_url: url,
                api_key,
                title: title.as_deref(),
                author: author.as_deref(),
                description,
                tags,
                frames: *frames,
                no_screenshot: *no_screenshot,
            })?;
            Ok(())
        }
        Some(Command::Edit { file }) => crate::studio::run_studio(file.clone()),
        None => crate::studio::run_studio(None),
    }
}
