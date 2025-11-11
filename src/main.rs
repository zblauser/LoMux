#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone, PartialEq)]
enum Preset {
	Mp4,
	Mkv,
	Webm,
	Mp3,
	Flac,
	Gif,
}

impl Preset {
	fn as_str(&self) -> &str {
		match self {
			Preset::Mp4 => "MP4",
			Preset::Mkv => "MKV",
			Preset::Webm => "WEBM",
			Preset::Mp3 => "MP3",
			Preset::Flac => "FLAC",
			Preset::Gif => "GIF",
		}
	}

	fn extension(&self) -> &str {
		match self {
			Preset::Mp4 => "mp4",
			Preset::Mkv => "mkv",
			Preset::Webm => "webm",
			Preset::Mp3 => "mp3",
			Preset::Flac => "flac",
			Preset::Gif => "gif",
		}
	}
}

struct LoMuxApp {
	input_files: Vec<PathBuf>,
	output_dir: Option<PathBuf>,
	preset: Preset,
	video_bitrate: u32,
	audio_bitrate: u32,
	flac_level: u8,
	gif_fps: u8,
	gif_width: u16,
	extra_args: String,
	console_output: Arc<Mutex<String>>,
	is_processing: Arc<Mutex<bool>>,
	progress: Arc<Mutex<f32>>,
	status: Arc<Mutex<String>>,
	ffmpeg_path: Option<PathBuf>,
	ffprobe_path: Option<PathBuf>,
}

impl Default for Preset {
	fn default() -> Self {
		Preset::Mp4
	}
}

impl LoMuxApp {
	fn new() -> Self {
		let mut app = Self {
			input_files: Vec::new(),
			output_dir: None,
			preset: Preset::Mp4,
			video_bitrate: 1000,
			audio_bitrate: 128,
			flac_level: 5,
			gif_fps: 10,
			gif_width: 320,
			extra_args: String::new(),
			console_output: Arc::new(Mutex::new(String::new())),
			is_processing: Arc::new(Mutex::new(false)),
			progress: Arc::new(Mutex::new(0.0)),
			status: Arc::new(Mutex::new("Ready".to_string())),
			ffmpeg_path: None,
			ffprobe_path: None,
		};

		app.ffmpeg_path = which::which("ffmpeg").ok();
		app.ffprobe_path = which::which("ffprobe").ok();

		#[cfg(target_os = "macos")]
		{
			if app.ffmpeg_path.is_none() {
				for path in &["/usr/local/bin/ffmpeg", "/opt/homebrew/bin/ffmpeg"] {
					let p = std::path::PathBuf::from(path);
					if p.exists() {
						app.ffmpeg_path = Some(p);
						break;
					}
				}
			}
			if app.ffprobe_path.is_none() {
				for path in &["/usr/local/bin/ffprobe", "/opt/homebrew/bin/ffprobe"] {
					let p = std::path::PathBuf::from(path);
					if p.exists() {
						app.ffprobe_path = Some(p);
						break;
					}
				}
			}
		}

		#[cfg(target_os = "windows")]
		{
			if app.ffmpeg_path.is_none() {
				let p = std::path::PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe");
				if p.exists() {
					app.ffmpeg_path = Some(p);
				}
			}
			if app.ffprobe_path.is_none() {
				let p = std::path::PathBuf::from("C:\\ffmpeg\\bin\\ffprobe.exe");
				if p.exists() {
					app.ffprobe_path = Some(p);
				}
			}
		}

		#[cfg(target_os = "linux")]
		{
			if app.ffmpeg_path.is_none() {
				let p = std::path::PathBuf::from("/usr/bin/ffmpeg");
				if p.exists() {
					app.ffmpeg_path = Some(p);
				}
			}
			if app.ffprobe_path.is_none() {
				let p = std::path::PathBuf::from("/usr/bin/ffprobe");
				if p.exists() {
					app.ffprobe_path = Some(p);
				}
			}
		}

		app
	}

	fn can_process(&self) -> bool {
		!self.input_files.is_empty()
			&& self.output_dir.is_some()
			&& self.ffmpeg_path.is_some()
			&& !*self.is_processing.lock().unwrap()
	}

	fn select_input_files(&mut self) {
		if let Some(files) = rfd::FileDialog::new()
			.set_title("Select Media Files")
			.pick_files()
		{
			self.input_files = files;
		}
	}

	fn select_output_dir(&mut self) {
		if let Some(dir) = rfd::FileDialog::new()
			.set_title("Select Output Directory")
			.pick_folder()
		{
			self.output_dir = Some(dir);
		}
	}

	fn build_ffmpeg_args(&self, input: &Path, output: &Path) -> Vec<String> {
		let mut args = vec![
			"-hide_banner".to_string(),
			"-threads".to_string(),
			"2".to_string(),
			"-progress".to_string(),
			"pipe:1".to_string(),
			"-nostats".to_string(),
			"-i".to_string(),
			input.to_string_lossy().to_string(),
		];

		match self.preset {
			Preset::Mp4 => {
				args.extend([
					"-c:v".to_string(), "libx264".to_string(),
					"-b:v".to_string(), format!("{}k", self.video_bitrate),
					"-c:a".to_string(), "aac".to_string(),
					"-b:a".to_string(), format!("{}k", self.audio_bitrate),
				]);
			}
			Preset::Mkv => {
				args.extend([
					"-c:v".to_string(), "libx265".to_string(),
					"-b:v".to_string(), format!("{}k", self.video_bitrate),
					"-c:a".to_string(), "libopus".to_string(),
					"-b:a".to_string(), format!("{}k", self.audio_bitrate),
				]);
			}
			Preset::Webm => {
				args.extend([
					"-c:v".to_string(), "libvpx-vp9".to_string(),
					"-b:v".to_string(), format!("{}k", self.video_bitrate),
					"-c:a".to_string(), "libopus".to_string(),
					"-b:a".to_string(), format!("{}k", self.audio_bitrate),
				]);
			}
			Preset::Mp3 => {
				args.extend([
					"-vn".to_string(),
					"-c:a".to_string(), "libmp3lame".to_string(),
					"-b:a".to_string(), format!("{}k", self.audio_bitrate),
				]);
			}
			Preset::Flac => {
				args.extend([
					"-vn".to_string(),
					"-c:a".to_string(), "flac".to_string(),
					"-compression_level".to_string(), self.flac_level.to_string(),
				]);
			}
			Preset::Gif => {
				args.extend([
					"-vf".to_string(),
					format!("fps={},scale={}:-1:flags=lanczos", self.gif_fps, self.gif_width),
				]);
			}
		}

		if !self.extra_args.is_empty() {
			args.extend(self.extra_args.split_whitespace().map(|s| s.to_string()));
		}

		args.extend(["-y".to_string(), output.to_string_lossy().to_string()]);
		args
	}

	fn start_processing(&mut self) {
		if !self.can_process() {
			return;
		}

		*self.is_processing.lock().unwrap() = true;
		*self.console_output.lock().unwrap() = String::new();

		let files = self.input_files.clone();
		let output_dir = self.output_dir.clone().unwrap();
		let preset = self.preset.clone();
		let ffmpeg = self.ffmpeg_path.clone().unwrap();
		let ffprobe = self.ffprobe_path.clone();
		let console = self.console_output.clone();
		let progress = self.progress.clone();
		let status = self.status.clone();
		let is_processing = self.is_processing.clone();

		let video_bitrate = self.video_bitrate;
		let audio_bitrate = self.audio_bitrate;
		let flac_level = self.flac_level;
		let gif_fps = self.gif_fps;
		let gif_width = self.gif_width;
		let extra_args = self.extra_args.clone();

		thread::spawn(move || {
			let app_temp = LoMuxApp {
				preset: preset.clone(),
				video_bitrate,
				audio_bitrate,
				flac_level,
				gif_fps,
				gif_width,
				extra_args,
				input_files: vec![],
				output_dir: None,
				console_output: Arc::new(Mutex::new(String::new())),
				is_processing: Arc::new(Mutex::new(false)),
				progress: Arc::new(Mutex::new(0.0)),
				status: Arc::new(Mutex::new(String::new())),
				ffmpeg_path: None,
				ffprobe_path: None,
			};

			let total = files.len();
			for (idx, input) in files.iter().enumerate() {
				let current = idx + 1;
				*status.lock().unwrap() = format!("Processing {}/{}: {}",
					current, total, input.file_name().unwrap().to_string_lossy());

				let stem = input.file_stem().unwrap().to_string_lossy();
				let output = output_dir.join(format!("{}_{}.{}",
					stem, preset.extension(), preset.extension()));

				let duration = if let Some(ref ffprobe) = ffprobe {
					get_duration(ffprobe, input).unwrap_or(0.0)
				} else {
					0.0
				};

				let args = app_temp.build_ffmpeg_args(input, &output);

				console.lock().unwrap().push_str(&format!(
					"\n=== Converting {} ({}/{}) ===\n",
					input.file_name().unwrap().to_string_lossy(), current, total
				));

				let mut child = Command::new(&ffmpeg)
					.args(&args)
					.stdout(Stdio::piped())
					.stderr(Stdio::piped())
					.spawn()
					.expect("Failed to spawn ffmpeg");

				let stdout = child.stdout.take().unwrap();
				let reader = BufReader::new(stdout);

				for line in reader.lines() {
					if let Ok(line) = line {
						console.lock().unwrap().push_str(&line);
						console.lock().unwrap().push('\n');

						if line.contains("out_time_ms=") && duration > 0.0 {
							if let Some(ms_str) = line.split("out_time_ms=").nth(1) {
								if let Ok(ms) = ms_str.trim().parse::<i64>() {
									let pct = (ms as f64 / 1000.0 / duration * 100.0).min(100.0);
									*progress.lock().unwrap() = pct as f32;
								}
							}
						}
					}
				}

				let _ = child.wait();
				*progress.lock().unwrap() = (current as f32 / total as f32) * 100.0;
			}

			*status.lock().unwrap() = "All tasks complete!".to_string();
			*progress.lock().unwrap() = 100.0;
			console.lock().unwrap().push_str("\nAll tasks complete!\n");
			*is_processing.lock().unwrap() = false;
		});
	}

	fn show_input_output(&mut self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.label("Input & Output");
			ui.separator();

			if ui.button("Select Files...").clicked() {
				self.select_input_files();
			}

			ui.label(if self.input_files.is_empty() {
				"No files selected".to_string()
			} else {
				format!("{} file(s) selected", self.input_files.len())
			});

			if !self.input_files.is_empty() {
				egui::ScrollArea::vertical()
					.id_salt("files")
					.max_height(3000.0)
					.show(ui, |ui| {
						for file in &self.input_files {
							ui.label(format!("• {}", file.file_name().unwrap().to_string_lossy()));
						}
					});
			}

			ui.add_space(4.0);

			if ui.button("Select Output Dir...").clicked() {
				self.select_output_dir();
			}

			ui.label(if let Some(ref dir) = self.output_dir {
				format!("📁 {}", dir.file_name().unwrap_or(dir.as_os_str()).to_string_lossy())
			} else {
				"No output directory".to_string()
			});
		});
	}

	fn show_presets(&mut self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.label("Presets & Controls");
			ui.separator();

			egui::ComboBox::from_label("Format")
				.selected_text(self.preset.as_str())
				.show_ui(ui, |ui| {
					ui.selectable_value(&mut self.preset, Preset::Mp4, "MP4");
					ui.selectable_value(&mut self.preset, Preset::Mkv, "MKV");
					ui.selectable_value(&mut self.preset, Preset::Webm, "WEBM");
					ui.selectable_value(&mut self.preset, Preset::Mp3, "MP3");
					ui.selectable_value(&mut self.preset, Preset::Flac, "FLAC");
					ui.selectable_value(&mut self.preset, Preset::Gif, "GIF");
				});

			ui.add_space(2.0);
			self.show_preset_controls(ui);
		});
	}

	fn show_preset_controls(&mut self, ui: &mut egui::Ui) {
		match self.preset {
			Preset::Mp4 | Preset::Mkv | Preset::Webm => {
				ui.horizontal(|ui| {
					ui.label("Video:");
					egui::ComboBox::from_id_salt("vbr")
						.selected_text(format!("{}k", self.video_bitrate))
						.width(70.0)
						.show_ui(ui, |ui| {
							for rate in [250, 500, 1000, 2000, 4000] {
								ui.selectable_value(&mut self.video_bitrate, rate, format!("{}k", rate));
							}
						});
				});
				ui.horizontal(|ui| {
					ui.label("Audio:");
					egui::ComboBox::from_id_salt("abr")
						.selected_text(format!("{}k", self.audio_bitrate))
						.width(70.0)
						.show_ui(ui, |ui| {
							for rate in [64, 96, 128, 192, 256, 320] {
								ui.selectable_value(&mut self.audio_bitrate, rate, format!("{}k", rate));
							}
						});
				});
			}
			Preset::Mp3 => {
				ui.horizontal(|ui| {
					ui.label("Bitrate:");
					egui::ComboBox::from_id_salt("mp3br")
						.selected_text(format!("{}k", self.audio_bitrate))
						.width(70.0)
						.show_ui(ui, |ui| {
							for rate in [64, 96, 128, 192, 256, 320] {
								ui.selectable_value(&mut self.audio_bitrate, rate, format!("{}k", rate));
							}
						});
				});
			}
			Preset::Flac => {
				ui.horizontal(|ui| {
					ui.label("Level:");
					ui.add(egui::Slider::new(&mut self.flac_level, 0..=8).show_value(true));
				});
			}
			Preset::Gif => {
				ui.horizontal(|ui| {
					ui.label("FPS:");
					egui::ComboBox::from_id_salt("fps")
						.selected_text(format!("{}", self.gif_fps))
						.width(50.0)
						.show_ui(ui, |ui| {
							for fps in [10, 15, 24, 30, 60] {
								ui.selectable_value(&mut self.gif_fps, fps, format!("{}", fps));
							}
						});
				});
				ui.horizontal(|ui| {
					ui.label("Width:");
					egui::ComboBox::from_id_salt("width")
						.selected_text(format!("{}px", self.gif_width))
						.width(70.0)
						.show_ui(ui, |ui| {
							for width in [320, 480, 640, 800, 1024] {
								ui.selectable_value(&mut self.gif_width, width, format!("{}px", width));
							}
						});
				});
			}
		}
	}

	fn show_advanced(&mut self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.label("Advanced");
			ui.separator();
			ui.label("Extra args:");
			ui.text_edit_singleline(&mut self.extra_args);
		});
	}

	fn show_process_controls(&mut self, ui: &mut egui::Ui) {
		let is_processing = *self.is_processing.lock().unwrap();
		ui.add_enabled_ui(!is_processing && self.can_process(), |ui| {
			if ui.button("Start Processing").clicked() {
				self.start_processing();
			}
		});

		let progress = *self.progress.lock().unwrap();
		ui.add(egui::ProgressBar::new(progress / 100.0).show_percentage());

		ui.label(self.status.lock().unwrap().clone());

		if !self.ffmpeg_path.is_some() {
			ui.colored_label(egui::Color32::RED, "⚠ FFmpeg not found!");
		}
	}

	fn show_console(&self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.set_height(ui.available_height());

			ui.label("Console Output");
			ui.separator();

			egui::ScrollArea::vertical()
				.id_salt("console")
				.auto_shrink(false)
				.stick_to_bottom(true)
				.show(ui, |ui| {
					let output = self.console_output.lock().unwrap();
					if output.is_empty() {
						ui.colored_label(egui::Color32::DARK_GRAY, "Output will appear here...");
					} else {
						ui.monospace(&*output);
					}
				});
		});
	}
}

fn get_duration(ffprobe: &Path, input: &Path) -> Result<f64, Box<dyn std::error::Error>> {
	let output = Command::new(ffprobe)
		.args([
			"-v", "error",
			"-show_entries", "format=duration",
			"-of", "csv=p=0",
			input.to_str().unwrap(),
		])
		.output()?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	Ok(stdout.trim().parse().unwrap_or(0.0))
}

impl eframe::App for LoMuxApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
			ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);
			ui.spacing_mut().menu_margin = egui::Margin::symmetric(6.0, 4.0);

			egui::Frame::none()
				.inner_margin(egui::Margin::same(8.0))
				.show(ui, |ui| {
					ui.horizontal(|ui| {
						ui.heading("LoMux");
						ui.separator();
						ui.label("v1.0.1");
					});
					ui.separator();

					ui.horizontal(|ui| {
						ui.vertical(|ui| {
							ui.set_min_width(220.0);
							ui.set_max_width(280.0);
							ui.set_max_height(400.0);

							egui::ScrollArea::vertical()
								.id_salt("controls")
								.show(ui, |ui| {
									self.show_input_output(ui);
									ui.add_space(4.0);
									self.show_presets(ui);
									ui.add_space(4.0);
									self.show_advanced(ui);
									ui.add_space(4.0);
									self.show_process_controls(ui);
								});
						});

						ui.separator();

						ui.vertical(|ui| {
							self.show_console(ui);
						});
					});
				});

			ctx.request_repaint_after(std::time::Duration::from_millis(100));
		});
	}
}

fn main() -> eframe::Result {
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([640.0, 400.0])
			.with_min_inner_size([480.0, 320.0]),
		..Default::default()
	};

	eframe::run_native(
		"LoMux",
		options,
		Box::new(|_cc| Ok(Box::new(LoMuxApp::new()))),
	)
}
