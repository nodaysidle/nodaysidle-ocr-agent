use clap::Parser;
use colored::*;
use way_ocr_agent::capture::ScreenCapture;
use way_ocr_agent::dispatch::{DispatchTarget, Dispatcher};
use way_ocr_agent::formatter::CodeFormatter;
use way_ocr_agent::ocr::OcrEngine;

#[derive(Parser)]
#[command(name = "way-ocr-agent")]
#[command(author = "nodaysidle <team@nodaysidle.org>")]
#[command(version = "0.1.0")]
#[command(about = "Fast Wayland Screen-Region OCR and Code Context Injector for AI coding agents and Herdr")]
struct Cli {
    /// Language code for Tesseract (defaults to eng)
    #[arg(short, long, default_value = "eng")]
    lang: String,

    /// Disable Markdown code fence detection and output raw extracted text
    #[arg(long)]
    raw: bool,

    /// Copy to clipboard only; skip auto-injection into active agent pane
    #[arg(long)]
    no_inject: bool,

    /// Print extracted markdown to stdout
    #[arg(short, long)]
    stdout: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // 1. Capture screen region
    let image_bytes = match ScreenCapture::capture_region().await {
        Ok(bytes) => bytes,
        Err(err) => {
            if err == "Selection cancelled" {
                std::process::exit(0);
            }
            eprintln!("{} {}", "error:".red().bold(), err);
            std::process::exit(1);
        }
    };

    // 2. Perform OCR in memory
    let raw_text = match OcrEngine::extract_text(&image_bytes, &cli.lang).await {
        Ok(text) => text,
        Err(err) => {
            eprintln!("{} {}", "error:".red().bold(), err);
            std::process::exit(1);
        }
    };

    // 3. Format text / reconstruct code fences
    let (formatted_text, _) = if cli.raw {
        (raw_text.clone(), way_ocr_agent::formatter::ContentKind::Prose)
    } else {
        CodeFormatter::format(&raw_text)
    };

    let word_count = formatted_text.split_whitespace().count();

    // 4. Always copy to Wayland clipboard
    if let Err(e) = Dispatcher::copy_to_clipboard(&formatted_text).await {
        eprintln!("{} Failed to copy to clipboard: {}", "warning:".yellow(), e);
    }

    // 5. Probe for agent target and inject if enabled
    let target = if cli.no_inject {
        DispatchTarget::ClipboardOnly
    } else {
        Dispatcher::detect_agent_target().await
    };

    if let Err(e) = Dispatcher::inject_to_agent(&target, &formatted_text).await {
        eprintln!("{} Failed to inject to agent: {}", "warning:".yellow(), e);
    }

    // 6. Notify user
    Dispatcher::notify(word_count, &target).await;

    // 7. Output to stdout if requested
    if cli.stdout {
        println!("{}", formatted_text);
    }
}
