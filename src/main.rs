use clap::Parser;
use colored::*;
use nodaysidle_ocr_agent::capture::ScreenCapture;
use nodaysidle_ocr_agent::dispatch::{Dispatcher, InjectOutcome};
use nodaysidle_ocr_agent::error::OcrError;
use nodaysidle_ocr_agent::formatter::CodeFormatter;
use nodaysidle_ocr_agent::liquid::classify_with_liquid;
use nodaysidle_ocr_agent::ocr::OcrEngine;

fn default_lang() -> String {
    std::env::var("OMARCHY_OCR_LANGS")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "eng".to_string())
}

#[derive(Parser)]
#[command(version, author, about)]
struct Cli {
    /// Language code for Tesseract (defaults to OMARCHY_OCR_LANGS or eng)
    #[arg(short, long, default_value_t = default_lang())]
    lang: String,

    /// Disable Markdown code fence detection and output raw extracted text
    #[arg(long)]
    raw: bool,

    /// Automatically inject formatted text into active agent pane (Herdr/Tmux) instead of clipboard only
    #[arg(short, long)]
    inject: bool,

    /// Print extracted markdown to stdout
    #[arg(short, long)]
    stdout: bool,

    /// Skip Liquid d1.2 classification even when LIQUID_API_KEY is set
    #[arg(long)]
    no_ai: bool,
}

fn acquire_instance_lock() -> Option<std::fs::File> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let lock_path = runtime_dir.join("way-ocr-agent.lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .ok()?;
    match file.try_lock() {
        Ok(()) => Some(file),
        Err(_) => None,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _lock = match acquire_instance_lock() {
        Some(file) => file,
        None => {
            eprintln!("way-ocr-agent is already running");
            std::process::exit(0);
        }
    };

    let cli = Cli::parse();

    // 1. Capture screen region
    let image_bytes = match ScreenCapture::capture_region().await {
        Ok(bytes) => bytes,
        Err(OcrError::Cancelled) => std::process::exit(0),
        Err(err) => {
            let msg = err.notification_message();
            eprintln!("{} {}", "error:".red().bold(), msg);
            Dispatcher::notify_error(&msg).await;
            std::process::exit(err.exit_code());
        }
    };

    // 2. Perform OCR in memory
    let raw_text = match OcrEngine::extract_text(&image_bytes, &cli.lang).await {
        Ok(text) => text,
        Err(err) => {
            let msg = err.notification_message();
            eprintln!("{} {}", "error:".red().bold(), msg);
            Dispatcher::notify_error(&msg).await;
            std::process::exit(err.exit_code());
        }
    };

    // 3. Format text / reconstruct code fences (uses Liquid d1.2 if LIQUID_API_KEY is set, else local heuristics)
    let (formatted_text, _) = if cli.raw {
        (
            raw_text.clone(),
            nodaysidle_ocr_agent::formatter::ContentKind::Prose,
        )
    } else if !cli.no_ai {
        if let Ok(api_key) = std::env::var("LIQUID_API_KEY") {
            if let Some(res) = classify_with_liquid(&raw_text, &api_key).await {
                if res.contains_secret {
                    eprintln!(
                        "{} possible secret or API key detected in OCR capture",
                        "warning:".yellow()
                    );
                }
                CodeFormatter::format_with_kind(&raw_text, res.kind)
            } else {
                CodeFormatter::format(&raw_text)
            }
        } else {
            CodeFormatter::format(&raw_text)
        }
    } else {
        CodeFormatter::format(&raw_text)
    };

    let word_count = raw_text.split_whitespace().count();

    // 4. Always copy to Wayland clipboard
    if let Err(err) = Dispatcher::copy_to_clipboard(&formatted_text).await {
        let msg = err.notification_message();
        eprintln!(
            "{} Failed to copy to clipboard: {}",
            "warning:".yellow(),
            msg
        );
        Dispatcher::notify_error(&msg).await;
    }

    // 5. Probe for agent target and inject ONLY if explicitly requested with --inject
    let outcome = if cli.inject {
        let target = Dispatcher::detect_agent_target().await;
        let res = Dispatcher::inject_to_agent(&target, &formatted_text).await;
        if let InjectOutcome::Failed { ref reason } = res {
            eprintln!(
                "{} Failed to inject to agent: {}",
                "warning:".yellow(),
                reason
            );
        }
        res
    } else {
        InjectOutcome::Skipped
    };

    // 6. Notify user
    Dispatcher::notify(word_count, &outcome).await;

    // 7. Output to stdout if requested
    if cli.stdout {
        println!("{}", formatted_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_picks_up_omarchy_ocr_langs() {
        let prev = std::env::var_os("OMARCHY_OCR_LANGS");
        std::env::set_var("OMARCHY_OCR_LANGS", "eng+slv");
        let cli = Cli::try_parse_from(["way-ocr-agent"]).unwrap();
        match prev {
            Some(val) => std::env::set_var("OMARCHY_OCR_LANGS", val),
            None => std::env::remove_var("OMARCHY_OCR_LANGS"),
        }
        assert_eq!(cli.lang, "eng+slv");
    }

    #[test]
    fn test_single_instance_lock_blocks_second_acquire() {
        let dir = std::env::temp_dir().join(format!("way-ocr-lock-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let prev = std::env::var_os("XDG_RUNTIME_DIR");
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
        let first = acquire_instance_lock();
        let second = acquire_instance_lock();
        drop(first);
        let third = acquire_instance_lock();
        match prev {
            Some(val) => std::env::set_var("XDG_RUNTIME_DIR", val),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(second.is_none());
        assert!(third.is_some());
    }
}
