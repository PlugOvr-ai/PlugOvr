use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// HuggingFace repository for the ONNX model files.
const MODEL_REPO: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";

/// Files required by parakeet-rs for TDT inference.
const MODEL_FILES: &[&str] = &[
    "encoder-model.onnx",
    "encoder-model.onnx.data",
    "decoder_joint-model.onnx",
    "vocab.txt",
];

/// Messages sent to the ASR model thread.
enum AudioMessage {
    /// Audio samples (already resampled to 16kHz mono f32) to transcribe.
    Transcribe(Vec<f32>),
    /// Shut down the ASR model thread.
    #[allow(dead_code)]
    Shutdown,
}

/// Voice control module for PlugOvr.
///
/// Captures audio from the microphone, transcribes it using NVIDIA's
/// Parakeet TDT 0.6B v3 multilingual model via ONNX Runtime, and returns
/// the transcribed text for use as computer control commands.
///
/// The ONNX model files are automatically downloaded from HuggingFace
/// on first use and cached in `~/.plugovr/parakeet-tdt-v3-model/`.
///
/// ## Usage
///
/// Press **F5** to start recording, press **F5** again to stop.
/// The audio is transcribed and executed as a computer control command.
pub struct VoiceControl {
    /// Whether audio is currently being recorded.
    pub is_recording: Arc<Mutex<bool>>,
    /// Whether the ASR model has finished loading.
    pub is_model_loaded: Arc<Mutex<bool>>,
    /// Whether a transcription is currently in progress.
    pub is_transcribing: Arc<Mutex<bool>>,
    /// Whether the model files are currently being downloaded.
    pub is_downloading: Arc<Mutex<bool>>,
    /// Human-readable download/loading progress message.
    pub download_progress: Arc<Mutex<String>>,
    /// The most recent transcription result.
    pub last_transcription: Arc<Mutex<String>>,
    /// Whether to show the voice control status UI.
    pub show_status: bool,
    /// Sender to submit audio to the ASR model thread.
    audio_tx: Option<mpsc::Sender<AudioMessage>>,
    /// Receiver for transcription results from the ASR model thread.
    result_rx: Option<mpsc::Receiver<String>>,
    /// Buffer for accumulating raw audio samples during recording.
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    /// The sample rate of the recording device.
    device_sample_rate: Arc<Mutex<u32>>,
}

impl VoiceControl {
    /// Create a new VoiceControl instance and begin downloading/loading the
    /// ASR model in a background thread.
    pub fn new() -> Self {
        let mut vc = Self {
            is_recording: Arc::new(Mutex::new(false)),
            is_model_loaded: Arc::new(Mutex::new(false)),
            is_transcribing: Arc::new(Mutex::new(false)),
            is_downloading: Arc::new(Mutex::new(false)),
            download_progress: Arc::new(Mutex::new(String::new())),
            last_transcription: Arc::new(Mutex::new(String::new())),
            show_status: false,
            audio_tx: None,
            result_rx: None,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            device_sample_rate: Arc::new(Mutex::new(44100)),
        };
        vc.init_model();
        vc
    }

    /// Spawn the ASR model thread which downloads model files if needed,
    /// loads the Parakeet TDT model, and waits for audio to transcribe.
    fn init_model(&mut self) {
        let model_path = get_model_path();
        let is_model_loaded = self.is_model_loaded.clone();
        let is_downloading = self.is_downloading.clone();
        let download_progress = self.download_progress.clone();

        let (audio_tx, audio_rx) = mpsc::channel::<AudioMessage>();
        let (result_tx, result_rx) = mpsc::channel::<String>();

        self.audio_tx = Some(audio_tx);
        self.result_rx = Some(result_rx);

        thread::Builder::new()
            .name("ASR Model Thread".to_string())
            .spawn(move || {
                // Download model files if any are missing
                if !model_files_present(&model_path) {
                    *is_downloading.lock().unwrap() = true;
                    *download_progress.lock().unwrap() =
                        "Preparing to download model...".to_string();

                    match download_model_files(&model_path, &download_progress) {
                        Ok(()) => {
                            println!("Voice Control: All model files ready.");
                        }
                        Err(e) => {
                            eprintln!("Voice Control: Failed to download model files: {}", e);
                            *download_progress.lock().unwrap() =
                                format!("Download failed: {}", e);
                            // Keep is_downloading true so the UI shows the error
                            return;
                        }
                    }
                    *is_downloading.lock().unwrap() = false;
                }

                // Load the model
                *download_progress.lock().unwrap() = "Loading ASR model...".to_string();
                println!(
                    "Voice Control: Loading Parakeet TDT v3 model from {:?}...",
                    model_path
                );

                match ParakeetTDT::from_pretrained(&model_path, None) {
                    Ok(mut model) => {
                        println!("Voice Control: Parakeet TDT v3 model loaded successfully");
                        *is_model_loaded.lock().unwrap() = true;
                        *download_progress.lock().unwrap() = String::new();

                        // Process transcription requests until shutdown
                        while let Ok(msg) = audio_rx.recv() {
                            match msg {
                                AudioMessage::Transcribe(audio) => {
                                    println!(
                                        "Voice Control: Transcribing {} samples ({:.1}s)...",
                                        audio.len(),
                                        audio.len() as f32 / 16000.0
                                    );
                                    match model.transcribe_samples(
                                        audio,
                                        16000,
                                        1,
                                        Some(TimestampMode::Sentences),
                                    ) {
                                        Ok(transcription) => {
                                            println!(
                                                "Voice Control: Transcription: \"{}\"",
                                                transcription.text
                                            );
                                            let _ = result_tx.send(transcription.text);
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Voice Control: Transcription error: {:?}",
                                                e
                                            );
                                            let _ = result_tx.send(String::new());
                                        }
                                    }
                                }
                                AudioMessage::Shutdown => break,
                            }
                        }
                        println!("Voice Control: ASR model thread shutting down");
                    }
                    Err(e) => {
                        eprintln!("Voice Control: Failed to load Parakeet TDT model: {:?}", e);
                        *download_progress.lock().unwrap() =
                            format!("Failed to load model: {}", e);
                    }
                }
            })
            .expect("Failed to spawn ASR model thread");
    }

    /// Toggle voice recording on/off.
    ///
    /// - First press: starts microphone recording.
    /// - Second press: stops recording, resamples to 16kHz, and sends to ASR.
    pub fn toggle_recording(&mut self) {
        self.show_status = true;

        // Don't allow toggling while transcription is in progress
        if *self.is_transcribing.lock().unwrap() {
            return;
        }

        // Don't allow toggling while downloading
        if *self.is_downloading.lock().unwrap() {
            println!("Voice Control: Model is still downloading, please wait...");
            return;
        }

        // Check if model is loaded
        if !*self.is_model_loaded.lock().unwrap() {
            println!("Voice Control: Model is still loading, please wait...");
            return;
        }

        let was_recording = *self.is_recording.lock().unwrap();
        if was_recording {
            // Stop recording - the recording thread will handle transcription
            *self.is_recording.lock().unwrap() = false;
        } else {
            self.start_recording();
        }
    }

    /// Start microphone recording in a background thread.
    fn start_recording(&mut self) {
        self.audio_buffer.lock().unwrap().clear();
        *self.last_transcription.lock().unwrap() = String::new();
        *self.is_recording.lock().unwrap() = true;

        let is_recording = self.is_recording.clone();
        let audio_buffer = self.audio_buffer.clone();
        let device_sample_rate = self.device_sample_rate.clone();
        let audio_tx = self.audio_tx.as_ref().cloned();
        let is_transcribing = self.is_transcribing.clone();

        thread::Builder::new()
            .name("Audio Recording Thread".to_string())
            .spawn(move || {
                let host = cpal::default_host();
                let device = match host.default_input_device() {
                    Some(d) => d,
                    None => {
                        eprintln!("Voice Control: No audio input device available");
                        *is_recording.lock().unwrap() = false;
                        return;
                    }
                };

                println!(
                    "Voice Control: Using input device: {:?}",
                    device
                        .description()
                        .map(|d| d.name().to_string())
                        .unwrap_or_else(|_| "Unknown".to_string())
                );

                let supported_config = match device.default_input_config() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Voice Control: Failed to get input config: {:?}", e);
                        *is_recording.lock().unwrap() = false;
                        return;
                    }
                };

                let source_rate = supported_config.sample_rate();
                let channels = supported_config.channels();
                let sample_format = supported_config.sample_format();
                *device_sample_rate.lock().unwrap() = source_rate;

                println!(
                    "Voice Control: Audio config: rate={}Hz, channels={}, format={:?}",
                    source_rate, channels, sample_format
                );

                let stream_config: cpal::StreamConfig = supported_config.into();

                // Build the input stream based on sample format
                let stream = match sample_format {
                    cpal::SampleFormat::F32 => {
                        let buf = audio_buffer.clone();
                        device.build_input_stream(
                            &stream_config,
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                let mut buffer = buf.lock().unwrap();
                                if channels > 1 {
                                    for chunk in data.chunks(channels as usize) {
                                        let mono: f32 =
                                            chunk.iter().sum::<f32>() / channels as f32;
                                        buffer.push(mono);
                                    }
                                } else {
                                    buffer.extend_from_slice(data);
                                }
                            },
                            |err| eprintln!("Voice Control: Audio stream error: {:?}", err),
                            None,
                        )
                    }
                    cpal::SampleFormat::I16 => {
                        let buf = audio_buffer.clone();
                        device.build_input_stream(
                            &stream_config,
                            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                let mut buffer = buf.lock().unwrap();
                                if channels > 1 {
                                    for chunk in data.chunks(channels as usize) {
                                        let mono: f32 = chunk
                                            .iter()
                                            .map(|&s| s as f32 / 32768.0)
                                            .sum::<f32>()
                                            / channels as f32;
                                        buffer.push(mono);
                                    }
                                } else {
                                    buffer.extend(data.iter().map(|&s| s as f32 / 32768.0));
                                }
                            },
                            |err| eprintln!("Voice Control: Audio stream error: {:?}", err),
                            None,
                        )
                    }
                    _ => {
                        // Fallback: try f32 (cpal may convert internally)
                        let buf = audio_buffer.clone();
                        device.build_input_stream(
                            &stream_config,
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                let mut buffer = buf.lock().unwrap();
                                if channels > 1 {
                                    for chunk in data.chunks(channels as usize) {
                                        let mono: f32 =
                                            chunk.iter().sum::<f32>() / channels as f32;
                                        buffer.push(mono);
                                    }
                                } else {
                                    buffer.extend_from_slice(data);
                                }
                            },
                            |err| eprintln!("Voice Control: Audio stream error: {:?}", err),
                            None,
                        )
                    }
                };

                let stream = match stream {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Voice Control: Failed to build input stream: {:?}", e);
                        *is_recording.lock().unwrap() = false;
                        return;
                    }
                };

                if let Err(e) = stream.play() {
                    eprintln!("Voice Control: Failed to start audio stream: {:?}", e);
                    *is_recording.lock().unwrap() = false;
                    return;
                }

                println!("Voice Control: Recording started. Press F5 to stop.");

                // Poll until recording is stopped
                while *is_recording.lock().unwrap() {
                    thread::sleep(std::time::Duration::from_millis(50));
                }

                // Drop stream to stop recording
                drop(stream);
                // Brief pause to ensure all buffered samples are flushed
                thread::sleep(std::time::Duration::from_millis(100));

                println!("Voice Control: Recording stopped.");

                let audio = audio_buffer.lock().unwrap().clone();
                let source_rate = *device_sample_rate.lock().unwrap();

                if audio.is_empty() {
                    eprintln!("Voice Control: No audio was recorded");
                    return;
                }

                let duration_secs = audio.len() as f32 / source_rate as f32;
                println!(
                    "Voice Control: Recorded {:.1}s of audio ({} samples at {}Hz)",
                    duration_secs,
                    audio.len(),
                    source_rate
                );

                // Reject very short recordings (< 0.3 seconds)
                if duration_secs < 0.3 {
                    eprintln!(
                        "Voice Control: Recording too short ({:.1}s), ignoring",
                        duration_secs
                    );
                    return;
                }

                // Resample to 16kHz if needed
                let resampled = if source_rate != 16000 {
                    println!(
                        "Voice Control: Resampling from {}Hz to 16000Hz...",
                        source_rate
                    );
                    resample_linear(&audio, source_rate, 16000)
                } else {
                    audio
                };

                // Send to ASR model thread for transcription
                *is_transcribing.lock().unwrap() = true;
                if let Some(tx) = audio_tx {
                    if let Err(e) = tx.send(AudioMessage::Transcribe(resampled)) {
                        eprintln!(
                            "Voice Control: Failed to send audio to ASR thread: {:?}",
                            e
                        );
                        *is_transcribing.lock().unwrap() = false;
                    }
                }
            })
            .expect("Failed to spawn audio recording thread");
    }

    /// Check for a transcription result (non-blocking).
    ///
    /// Returns `Some(text)` if a new transcription is available, `None` otherwise.
    pub fn check_transcription_result(&mut self) -> Option<String> {
        if let Some(rx) = &self.result_rx {
            match rx.try_recv() {
                Ok(text) => {
                    *self.is_transcribing.lock().unwrap() = false;
                    let trimmed = text.trim().to_string();
                    *self.last_transcription.lock().unwrap() = trimmed.clone();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                }
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    eprintln!("Voice Control: ASR model thread disconnected");
                    None
                }
            }
        } else {
            None
        }
    }

    /// Display the voice control status overlay using egui.
    pub fn show_voice_status(&self, egui_context: &egui::Context) {
        if !self.show_status {
            return;
        }

        let is_recording = *self.is_recording.lock().unwrap();
        let is_transcribing = *self.is_transcribing.lock().unwrap();
        let is_model_loaded = *self.is_model_loaded.lock().unwrap();
        let is_downloading = *self.is_downloading.lock().unwrap();
        let download_progress = self.download_progress.lock().unwrap().clone();
        let last_transcription = self.last_transcription.lock().unwrap().clone();

        // Don't show if nothing interesting to display
        if !is_recording
            && !is_transcribing
            && !is_downloading
            && download_progress.is_empty()
            && last_transcription.is_empty()
            && is_model_loaded
        {
            return;
        }

        egui::Window::new("Voice Control")
            .movable(true)
            .collapsible(true)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-10.0, 10.0))
            .show(egui_context, |ui| {
                if is_downloading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.vertical(|ui| {
                            ui.label("Downloading ASR model...");
                            if !download_progress.is_empty() {
                                ui.label(
                                    egui::RichText::new(&download_progress)
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        });
                    });
                } else if !download_progress.is_empty() && !is_model_loaded {
                    // Error or loading state
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(&download_progress);
                    });
                } else if !is_model_loaded {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading ASR model...");
                    });
                } else if is_recording {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("●")
                                .color(egui::Color32::RED)
                                .size(20.0),
                        );
                        ui.label("Recording... Press F5 to stop");
                    });
                } else if is_transcribing {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Transcribing...");
                    });
                } else if !last_transcription.is_empty() {
                    ui.label("Voice command:");
                    ui.label(
                        egui::RichText::new(&last_transcription)
                            .italics()
                            .color(egui::Color32::DARK_GREEN),
                    );
                }
            });
    }
}

// ---------------------------------------------------------------------------
// Model download helpers
// ---------------------------------------------------------------------------

/// Check whether all required model files are present in `model_path`.
fn model_files_present(model_path: &Path) -> bool {
    MODEL_FILES
        .iter()
        .all(|f| model_path.join(f).exists())
}

/// Download any missing model files from HuggingFace into `model_path`.
///
/// Updates `progress` with a human-readable status after each file.
fn download_model_files(
    model_path: &Path,
    progress: &Arc<Mutex<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(model_path)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(None) // no timeout for large files
        .build()?;

    for (i, filename) in MODEL_FILES.iter().enumerate() {
        let file_path = model_path.join(filename);
        if file_path.exists() {
            println!("Voice Control: {} already exists, skipping.", filename);
            *progress.lock().unwrap() = format!(
                "[{}/{}] {} (cached)",
                i + 1,
                MODEL_FILES.len(),
                filename
            );
            continue;
        }

        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            MODEL_REPO, filename
        );

        *progress.lock().unwrap() = format!(
            "[{}/{}] Downloading {}...",
            i + 1,
            MODEL_FILES.len(),
            filename
        );
        println!("Voice Control: Downloading {} from {}", filename, url);

        let response = client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP {} when downloading {}",
                response.status(),
                filename
            )
            .into());
        }

        let total_bytes = response.content_length();
        if let Some(size) = total_bytes {
            let mb = size as f64 / 1_048_576.0;
            *progress.lock().unwrap() = format!(
                "[{}/{}] Downloading {} ({:.1} MB)...",
                i + 1,
                MODEL_FILES.len(),
                filename,
                mb
            );
            println!("Voice Control: {} size: {:.1} MB", filename, mb);
        }

        // Stream to a temp file first, then rename on success to avoid
        // partial files if the download is interrupted.
        let tmp_path = file_path.with_extension("tmp");
        {
            let mut tmp_file = std::fs::File::create(&tmp_path)?;
            let mut reader = std::io::BufReader::new(response);
            std::io::copy(&mut reader, &mut tmp_file)?;
            tmp_file.flush()?;
        }
        std::fs::rename(&tmp_path, &file_path)?;

        println!("Voice Control: {} downloaded successfully.", filename);
    }

    *progress.lock().unwrap() = "Download complete.".to_string();
    Ok(())
}

// ---------------------------------------------------------------------------
// Audio resampler
// ---------------------------------------------------------------------------

/// Simple linear interpolation resampler.
///
/// Resamples mono audio from `from_rate` to `to_rate` using linear
/// interpolation. This is adequate quality for speech recognition input.
fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx_floor = src_idx.floor() as usize;
        let frac = (src_idx - idx_floor as f64) as f32;

        if idx_floor + 1 < input.len() {
            let sample = input[idx_floor] * (1.0 - frac) + input[idx_floor + 1] * frac;
            output.push(sample);
        } else if idx_floor < input.len() {
            output.push(input[idx_floor]);
        }
    }

    output
}

/// Get the path where the Parakeet TDT ONNX model files are stored.
pub fn get_model_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".plugovr")
        .join("parakeet-tdt-v3-model")
}
