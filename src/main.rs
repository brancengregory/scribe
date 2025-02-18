use clap::Parser;
use console::{Style, Term};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::error::Error;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// A CLI tool that records audio, transcribes it using whisper,
/// and copies the transcription to the clipboard.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {}

/// Helper: Clears the current line and prints a styled message starting with a bullet.
fn print_step(term: &Term, msg: &str, style: &Style) -> io::Result<()> {
    // Clear the current line and print a message starting with "> "
    term.clear_line()?;
    term.write_line(&format!("> {}", style.apply_to(msg)))
}

fn main() -> Result<(), Box<dyn Error>> {
    let _args = Args::parse();
    let term = Term::stdout();
    // Define a common heading style: bold cyan.
    let heading = Style::new().bold().cyan();

    print_step(&term, "Starting audio recording...", &heading)?;

    // Create a temporary filename using a UNIX timestamp.
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH)?.as_secs();
    let output_file = format!("output_{}.wav", since_epoch);

    // Spawn ffmpeg for recording.
    let mut ffmpeg_child = Command::new("ffmpeg")
        .args(&[
            "-y", // Overwrite output file without prompting.
            "-f",
            "alsa",
            "-i",
            "front:CARD=BRIO",
            "-filter:a",
            "volume=2",
            "-t",
            "3600",
            &output_file,
        ])
        .stderr(Stdio::piped())
        .spawn()?;

    print_step(
        &term,
        "Recording in progress... Press any key to stop.",
        &heading,
    )?;
    // Wait for a single key press using console's built-in method.
    let _ = term.read_key()?;
    term.clear_line()?;
    term.write_line("> Stopping recording...")?;

    // Send SIGINT to stop ffmpeg gracefully.
    kill(Pid::from_raw(ffmpeg_child.id() as i32), Signal::SIGINT)?;

    // Wait for ffmpeg to exit.
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
