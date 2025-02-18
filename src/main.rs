use clap::Parser;
use console::{Style, Term};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Application configuration loaded from a TOML file.
#[derive(Debug, Deserialize)]
struct Config {
    device: Option<String>,
    duration: Option<u64>,
    volume: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: Some("front:CARD=BRIO".to_string()),
            duration: Some(3600),
            volume: Some(2.0),
        }
    }
}

/// CLI arguments with possible overrides for configuration.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file (TOML format)
    #[arg(short, long, default_value = "~/.config/scribe/config.toml")]
    config: String,
    /// Audio input device
    #[arg(long)]
    device: Option<String>,
    /// Recording duration in seconds
    #[arg(long)]
    duration: Option<u64>,
    /// Audio volume multiplier
    #[arg(long)]
    volume: Option<f32>,
}

/// Load configuration from a file, expanding '~' if necessary.
fn load_config(path: &str) -> Config {
    let expanded_path = if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            path.replacen("~", home.to_str().unwrap_or(""), 1)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };

    fs::read_to_string(&expanded_path)
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

/// Merge CLI arguments with configuration file values, with CLI taking precedence.
fn merged_config(args: Args, file_config: Config) -> (String, u64, f32) {
    let device = args
        .device
        .or(file_config.device)
        .unwrap_or_else(|| "front:CARD=BRIO".to_string());
    let duration = args.duration.or(file_config.duration).unwrap_or(3600);
    let volume = args.volume.or(file_config.volume).unwrap_or(2.0);
    (device, duration, volume)
}

/// Clears the current line and prints a styled message starting with a bullet.
fn print_step(term: &Term, msg: &str, style: &Style) -> io::Result<()> {
    term.clear_line()?;
    term.write_line(&format!("> {}", style.apply_to(msg)))
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse CLI arguments and load config file.
    let args = Args::parse();
    let file_config = load_config(&args.config);
    let (device, duration, volume) = merged_config(args, file_config);

    let term = Term::stdout();
    let heading = Style::new().bold().cyan();

    print_step(&term, "Starting audio recording...", &heading)?;

    // Create a temporary filename using a UNIX timestamp.
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH)?.as_secs();
    let output_file = format!("output_{}.wav", since_epoch);

    // Spawn ffmpeg for recording with configured parameters.
    let mut ffmpeg_child = Command::new("ffmpeg")
        .args(&[
            "-y", // Overwrite output file without prompting.
            "-f",
            "alsa",
            "-i",
            &device,
            "-filter:a",
            &format!("volume={}", volume),
            "-t",
            &duration.to_string(),
            &output_file,
        ])
        .stderr(Stdio::piped())
        .spawn()?;

    print_step(
        &term,
        "Recording in progress... Press any key to stop.",
        &heading,
    )?;
    // Wait for a single key press.
    let _ = term.read_key()?;
    term.clear_line()?;
    term.write_line("> Stopping recording...")?;

    // Send SIGINT to stop ffmpeg gracefully.
    kill(Pid::from_raw(ffmpeg_child.id() as i32), Signal::SIGINT)?;
    let ffmpeg_exit = ffmpeg_child.wait()?;
    if let Some(code) = ffmpeg_exit.code() {
        // Accept both 130 and 255 as graceful SIGINT terminations.
        if code == 130 || code == 255 {
            print_step(
                &term,
                "Recording stopped via SIGINT (desired behavior).",
                &heading,
            )?;
        } else if code != 0 {
            return Err(format!("Failed to record audio. Exit code: {}", code).into());
        }
    } else {
        return Err("ffmpeg terminated without an exit code".into());
    }

    print_step(&term, "Transcribing audio...", &heading)?;
    // Run whisper to transcribe the audio.
    let whisper_output = Command::new("whisper")
        .args(&[
            "--model",
            "turbo",
            "--device",
            "cuda",
            "--language",
            "en",
            &output_file,
        ])
        .output()?;
    if !whisper_output.status.success() {
        return Err("Whisper transcription failed.".into());
    }
    let transcription = String::from_utf8(whisper_output.stdout)?;
    print_step(&term, "Transcription complete.", &heading)?;

    // Always copy the transcription to the clipboard.
    print_step(&term, "Copying transcription to clipboard...", &heading)?;
    let mut cb_child = Command::new("cb")
        .arg("copy")
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(stdin) = cb_child.stdin.as_mut() {
        stdin.write_all(transcription.as_bytes())?;
    }
    let cb_exit = cb_child.wait()?;
    if !cb_exit.success() {
        return Err("Failed to copy transcription to clipboard.".into());
    }
    print_step(&term, "✔ Copied transcription to clipboard.", &heading)?;

    print_step(&term, "Process completed successfully.", &heading)?;
    Ok(())
}
