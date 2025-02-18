use clap::Parser;
use std::process::{Command, Stdio};
use std::io::{self, Write};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use crossterm::{
    event::{read, Event},
    terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType},
    execute,
};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

/// A CLI tool that records audio, transcribes it using whisper, and copies the transcription to the clipboard.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {}

fn main() -> Result<(), Box<dyn Error>> {
    let _args = Args::parse();

    // Create a temporary filename with a UNIX timestamp.
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH)?.as_secs();
    let output_file = format!("output_{}.wav", since_epoch);

    println!("Starting audio recording...");
    // Spawn ffmpeg with a long duration so it stays active until we stop it.
    let mut ffmpeg_child = Command::new("ffmpeg")
        .args(&[
            "-y", // Overwrite output file without prompting.
            "-f", "alsa",
            "-i", "front:CARD=BRIO",
            "-filter:a", "volume=2",
            "-t", "3600",
            &output_file,
        ])
        .stderr(Stdio::piped())
        .spawn()?;

    // Enable raw mode to capture a single key press.
    enable_raw_mode()?;
    println!("Press any key to stop recording...");
    // Wait for any key event.
    if let Event::Key(_) = read()? {
        // Disable raw mode as soon as the key is captured.
        disable_raw_mode()?;
        // Clear the current line to remove any stray characters.
        execute!(io::stdout(), Clear(ClearType::CurrentLine))?;
        println!(); // Print a newline to start fresh.
        // Send SIGINT to ffmpeg so it stops gracefully.
        kill(Pid::from_raw(ffmpeg_child.id() as i32), Signal::SIGINT)?;
    } else {
        disable_raw_mode()?;
    }

    // Wait for ffmpeg to exit.
    let ffmpeg_exit = ffmpeg_child.wait()?;
    if let Some(code) = ffmpeg_exit.code() {
        // Accept both 130 and 255 as valid exit codes for a SIGINT termination.
        if code == 130 || code == 255 {
            println!("Recording stopped via SIGINT (desired behavior).");
        } else if code != 0 {
            return Err(format!("Failed to record audio. Exit code: {}", code).into());
        }
    } else {
        return Err("ffmpeg terminated without an exit code".into());
    }

    println!("Transcribing audio...");
    // Run whisper and capture its output.
    let whisper_output = Command::new("whisper")
        .args(&[
            "--model", "turbo",
            "--device", "cuda",
            "--language", "en",
            &output_file,
        ])
        .output()?;
    if !whisper_output.status.success() {
        return Err("Whisper transcription failed".into());
    }
    let transcription = String::from_utf8(whisper_output.stdout)?;

    println!("Copying transcription to clipboard...");
    // Pipe the transcription to 'cb copy'.
    let mut cb_child = Command::new("cb")
        .arg("copy")
        .stdin(Stdio::piped())
        .spawn()?;
    {
        let stdin = cb_child.stdin.as_mut().ok_or("Failed to open stdin for cb")?;
        stdin.write_all(transcription.as_bytes())?;
    }
    let cb_exit = cb_child.wait()?;
    if !cb_exit.success() {
        return Err("Failed to copy transcription to clipboard".into());
    }

    println!("Process completed successfully.");
    Ok(())
}
