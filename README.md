# Scribe

A minimal Rust-based CLI tool that records audio using **ffmpeg**, transcribes it using **whisper**, and automatically copies the transcription to your clipboard. You press any key to stop recording.

## Features

- **Record audio** from an ALSA device (e.g., `front:CARD=BRIO`) until you press a key.
- **Transcribe** the recorded audio with [whisper](https://github.com/openai/whisper).
- **Copy** the transcription to the clipboard automatically (using `cb copy`).

## Requirements

1. **Rust** (1.64+ recommended) to build this project.
2. **ffmpeg** installed and available on your `PATH`.
3. **whisper** (CLI) installed and available on your `PATH`.
4. A **clipboard** tool named `cb` that supports the `copy` subcommand (e.g., a script or alias to `xclip` or `pbcopy`).
5. **ALSA** on Linux (or adapt the ffmpeg input device line for your system).

## Installation

1. Clone this repository or copy the source files into a new Rust project.
2. In the project directory, run:

```bash
cargo build --release
```
3. The compiled binary will be at target/release/scribe.

4. (Optional) Move scribe into a directory on your PATH:

```bash
cp target/release/scribe /usr/local/bin
```

## Usage

1. Start recording by running scribe:

```bash
./scribe
```

2. Recording will begin immediately, and you will see messages like:

```bash
> Starting audio recording...
> Recording in progress... Press any key to stop.
```

3. Stop recording by pressing any key. The tool will:

* Send a SIGINT to ffmpeg
* Wait for ffmpeg to stop
* Transcribe the recorded audio with whisper
* Copy the resulting transcription to your clipboard
* Check your clipboard—the transcribed text should now be there.

## Adapting to Your Audio Device

By default, the code uses:

```bash
-f alsa -i front:CARD=BRIO
```

If your system’s device differs, adjust that argument in the `Command::new("ffmpeg")` section of the code. For example, you might use -i hw:0,0 or another device string.

## License

This project is distributed under the terms of the MIT License. See LICENSE for details (or update with your own license if needed).

## Contributing

Feel free to submit issues or pull requests to improve or extend functionality, such as:

* Supporting other audio input methods
* Integrating different transcription backends
* Handling advanced ffmpeg or whisper flags

## Acknowledgments

* Rust for providing a powerful, safe systems language
* ffmpeg for robust audio recording and processing
* whisper for speech-to-text transcription
* console for clean terminal output
