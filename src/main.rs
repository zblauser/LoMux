#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{BufRead, BufReader};

// ============= PRESET SYSTEM =============

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]  // These are available for future presets
enum Container {
	Mp4,
	Mkv,
	Webm,
	Mp3,
	Flac,
	Aac,
	Opus,
	Wav,
	Gif,
	Avi,
	Mov,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]  // Available for custom presets
enum VideoCodec {
	H264,
	H265,
	Vp8,
	Vp9,
	Av1,
	ProRes,
	DnxHd,
	Copy,
	None,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]  // Available for custom presets
enum AudioCodec {
	Aac,
	Mp3,
	Opus,
	Flac,
	Vorbis,
	Pcm,
	Copy,
	None,
}

#[derive(Debug, Clone, PartialEq)]
enum PresetCategory {
	WebSocial,
	Device,
	Professional,
	Audio,
	MatchSource,
	Custom,
}

#[derive(Debug, Clone)]
struct EncodingPreset {
	name: String,
	category: PresetCategory,
	container: Container,
	video_codec: VideoCodec,
	audio_codec: AudioCodec,
	video_bitrate: Option<u32>,
	audio_bitrate: Option<u32>,
	video_crf: Option<u8>,
	fps: Option<u8>,
	resolution: Option<String>,
	description: String,
}

impl EncodingPreset {
	fn get_extension(&self) -> &str {
		match self.container {
			Container::Mp4 => "mp4",
			Container::Mkv => "mkv",
			Container::Webm => "webm",
			Container::Mp3 => "mp3",
			Container::Flac => "flac",
			Container::Aac => "aac",
			Container::Opus => "opus",
			Container::Wav => "wav",
			Container::Gif => "gif",
			Container::Avi => "avi",
			Container::Mov => "mov",
		}
	}

	fn youtube_1080p() -> Self {
		Self {
			name: "YouTube 1080p HD".to_string(),
			category: PresetCategory::WebSocial,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: Some(8000),
			audio_bitrate: Some(320),
			video_crf: None,
			fps: None,
			resolution: Some("1920x1080".to_string()),
			description: "Optimized for YouTube 1080p uploads".to_string(),
		}
	}

	fn youtube_4k() -> Self {
		Self {
			name: "YouTube 4K UHD".to_string(),
			category: PresetCategory::WebSocial,
			container: Container::Mp4,
			video_codec: VideoCodec::H265,
			audio_codec: AudioCodec::Aac,
			video_bitrate: Some(35000),
			audio_bitrate: Some(320),
			video_crf: None,
			fps: None,
			resolution: Some("3840x2160".to_string()),
			description: "High quality 4K for YouTube".to_string(),
		}
	}

	fn instagram_feed() -> Self {
		Self {
			name: "Instagram Feed".to_string(),
			category: PresetCategory::WebSocial,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: Some(3500),
			audio_bitrate: Some(128),
			video_crf: None,
			fps: Some(30),
			resolution: Some("1080x1080".to_string()),
			description: "Square video for Instagram feed".to_string(),
		}
	}

	fn tiktok() -> Self {
		Self {
			name: "TikTok".to_string(),
			category: PresetCategory::WebSocial,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: Some(6000),
			audio_bitrate: Some(128),
			video_crf: None,
			fps: Some(30),
			resolution: Some("1080x1920".to_string()),
			description: "Vertical video for TikTok".to_string(),
		}
	}

	fn prores_422() -> Self {
		Self {
			name: "ProRes 422".to_string(),
			category: PresetCategory::Professional,
			container: Container::Mov,
			video_codec: VideoCodec::ProRes,
			audio_codec: AudioCodec::Pcm,
			video_bitrate: None,
			audio_bitrate: None,
			video_crf: None,
			fps: None,
			resolution: None,
			description: "Apple ProRes 422 for editing".to_string(),
		}
	}

	fn dnxhd_1080p() -> Self {
		Self {
			name: "DNxHD 1080p".to_string(),
			category: PresetCategory::Professional,
			container: Container::Mov,
			video_codec: VideoCodec::DnxHd,
			audio_codec: AudioCodec::Pcm,
			video_bitrate: Some(185000),
			audio_bitrate: None,
			video_crf: None,
			fps: None,
			resolution: Some("1920x1080".to_string()),
			description: "Avid DNxHD for broadcast".to_string(),
		}
	}

	fn iphone_ipad() -> Self {
		Self {
			name: "iPhone/iPad".to_string(),
			category: PresetCategory::Device,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: Some(5000),
			audio_bitrate: Some(160),
			video_crf: None,
			fps: None,
			resolution: None,
			description: "Compatible with all iOS devices".to_string(),
		}
	}

	fn android() -> Self {
		Self {
			name: "Android".to_string(),
			category: PresetCategory::Device,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: Some(4000),
			audio_bitrate: Some(128),
			video_crf: None,
			fps: None,
			resolution: None,
			description: "Compatible with Android devices".to_string(),
		}
	}

	fn match_source_high() -> Self {
		Self {
			name: "Match Source - High".to_string(),
			category: PresetCategory::MatchSource,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: None,
			audio_bitrate: None,
			video_crf: Some(18),
			fps: None,
			resolution: None,
			description: "High quality, larger file".to_string(),
		}
	}

	fn match_source_medium() -> Self {
		Self {
			name: "Match Source - Medium".to_string(),
			category: PresetCategory::MatchSource,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: None,
			audio_bitrate: None,
			video_crf: Some(23),
			fps: None,
			resolution: None,
			description: "Balanced quality and size".to_string(),
		}
	}

	fn match_source_low() -> Self {
		Self {
			name: "Match Source - Low".to_string(),
			category: PresetCategory::MatchSource,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: None,
			audio_bitrate: None,
			video_crf: Some(28),
			fps: None,
			resolution: None,
			description: "Lower quality, smaller file".to_string(),
		}
	}

	fn mp3_high_quality() -> Self {
		Self {
			name: "MP3 High Quality".to_string(),
			category: PresetCategory::Audio,
			container: Container::Mp3,
			video_codec: VideoCodec::None,
			audio_codec: AudioCodec::Mp3,
			video_bitrate: None,
			audio_bitrate: Some(320),
			video_crf: None,
			fps: None,
			resolution: None,
			description: "High quality MP3 audio".to_string(),
		}
	}

	fn mp3_standard() -> Self {
		Self {
			name: "MP3 Standard".to_string(),
			category: PresetCategory::Audio,
			container: Container::Mp3,
			video_codec: VideoCodec::None,
			audio_codec: AudioCodec::Mp3,
			video_bitrate: None,
			audio_bitrate: Some(192),
			video_crf: None,
			fps: None,
			resolution: None,
			description: "Standard quality MP3".to_string(),
		}
	}

	fn flac_lossless() -> Self {
		Self {
			name: "FLAC Lossless".to_string(),
			category: PresetCategory::Audio,
			container: Container::Flac,
			video_codec: VideoCodec::None,
			audio_codec: AudioCodec::Flac,
			video_bitrate: None,
			audio_bitrate: None,
			video_crf: None,
			fps: None,
			resolution: None,
			description: "Lossless FLAC audio".to_string(),
		}
	}

	fn aac_high_quality() -> Self {
		Self {
			name: "AAC High Quality".to_string(),
			category: PresetCategory::Audio,
			container: Container::Aac,
			video_codec: VideoCodec::None,
			audio_codec: AudioCodec::Aac,
			video_bitrate: None,
			audio_bitrate: Some(256),
			video_crf: None,
			fps: None,
			resolution: None,
			description: "High quality AAC audio".to_string(),
		}
	}

	fn get_all_presets() -> Vec<Self> {
		vec![
			Self::youtube_1080p(),
			Self::youtube_4k(),
			Self::instagram_feed(),
			Self::tiktok(),

			Self::iphone_ipad(),
			Self::android(),

			Self::prores_422(),
			Self::dnxhd_1080p(),

			Self::mp3_high_quality(),
			Self::mp3_standard(),
			Self::flac_lossless(),
			Self::aac_high_quality(),

			Self::match_source_high(),
			Self::match_source_medium(),
			Self::match_source_low(),
		]
	}
}

// ============= FILE & METADATA MANAGEMENT =============

#[derive(Debug, Clone)]
struct MediaFile {
	path: PathBuf,
	metadata: AudioMetadata,
	apply_metadata: bool,
	is_youtube: bool,
	youtube_url: Option<String>,
	download_status: DownloadStatus,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]  // Downloading variant will be used for progress tracking
enum DownloadStatus {
	NotStarted,
	Downloading(f32),
	Downloaded,
	Failed,
}

impl MediaFile {
	fn new(path: PathBuf) -> Self {
		Self {
			path,
			metadata: AudioMetadata::default(),
			apply_metadata: false,
			is_youtube: false,
			youtube_url: None,
			download_status: DownloadStatus::NotStarted,
		}
	}

	fn new_youtube(url: String) -> Self {
		Self {
			path: PathBuf::new(),
			metadata: AudioMetadata::default(),
			apply_metadata: false,
			is_youtube: true,
			youtube_url: Some(url),
			download_status: DownloadStatus::NotStarted,
		}
	}

	fn display_name(&self) -> String {
		if self.is_youtube {
			format!("📺 {}", self.youtube_url.as_ref().unwrap_or(&"YouTube".to_string()))
		} else {
			self.path.file_name().unwrap_or_default().to_string_lossy().to_string()
		}
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AudioMetadata {
	title: Option<String>,
	artist: Option<String>,
	album: Option<String>,
	year: Option<String>,
	genre: Option<String>,
	track: Option<String>,
	comment: Option<String>,
}

impl AudioMetadata {
	fn is_empty(&self) -> bool {
		self.title.is_none()
			&& self.artist.is_none()
			&& self.album.is_none()
			&& self.year.is_none()
			&& self.genre.is_none()
			&& self.track.is_none()
			&& self.comment.is_none()
	}

	fn clear(&mut self) {
		*self = Self::default();
	}

	fn from_filename(filename: &str) -> Self {
		let mut metadata = Self::default();

		if let Some((artist, title)) = filename.rsplit_once('.').and_then(|(name, _)| {
			name.split_once(" - ")
		}) {
			metadata.artist = Some(artist.trim().to_string());
			metadata.title = Some(title.trim().to_string());
		} else {
			if let Some(name) = filename.rsplit_once('.') {
				metadata.title = Some(name.0.to_string());
			}
		}

		metadata
	}
}

// ============= TOOL DETECTION =============

struct ToolDetector {
	ffmpeg_path: Option<PathBuf>,
	ffprobe_path: Option<PathBuf>,
	ytdlp_path: Option<PathBuf>,
}

impl ToolDetector {
	fn new() -> Self {
		let mut detector = Self {
			ffmpeg_path: None,
			ffprobe_path: None,
			ytdlp_path: None,
		};
		detector.detect_all();
		detector
	}

	fn detect_all(&mut self) {
		self.ffmpeg_path = Self::find_tool("ffmpeg");
		self.ffprobe_path = Self::find_tool("ffprobe");
		self.ytdlp_path = Self::find_tool("yt-dlp");

		if self.ytdlp_path.is_none() {
			self.ytdlp_path = Self::find_tool("youtube-dl");
		}
	}

	fn find_tool(tool_name: &str) -> Option<PathBuf> {
		if let Ok(path) = which::which(tool_name) {
			return Some(path);
		}

		let search_paths = Self::get_search_paths(tool_name);

		for path_str in search_paths {
			let path = PathBuf::from(path_str);
			if path.exists() {
				return Some(path);
			}
		}

		#[cfg(target_os = "macos")]
		{
			if let Ok(output) = Command::new("brew")
				.args(&["--prefix"])
				.output()
			{
				let brew_prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
				let brew_path = PathBuf::from(format!("{}/bin/{}", brew_prefix, tool_name));
				if brew_path.exists() {
					return Some(brew_path);
				}
			}
		}

		None
	}

	fn get_search_paths(tool_name: &str) -> Vec<&'static str> {
		match tool_name {
			"ffmpeg" => {
				#[cfg(target_os = "windows")]
				return vec![
					"C:\\ffmpeg\\bin\\ffmpeg.exe",
					"C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
					"C:\\Program Files (x86)\\ffmpeg\\bin\\ffmpeg.exe",
				];

				#[cfg(target_os = "macos")]
				return vec![
					"/usr/local/bin/ffmpeg",
					"/opt/homebrew/bin/ffmpeg",
					"/opt/local/bin/ffmpeg",
					"/usr/bin/ffmpeg",
				];

				#[cfg(target_os = "linux")]
				return vec![
					"/usr/bin/ffmpeg",
					"/usr/local/bin/ffmpeg",
					"/snap/bin/ffmpeg",
				];
			}
			"ffprobe" => {
				#[cfg(target_os = "windows")]
				return vec![
					"C:\\ffmpeg\\bin\\ffprobe.exe",
					"C:\\Program Files\\ffmpeg\\bin\\ffprobe.exe",
					"C:\\Program Files (x86)\\ffmpeg\\bin\\ffprobe.exe",
				];

				#[cfg(target_os = "macos")]
				return vec![
					"/usr/local/bin/ffprobe",
					"/opt/homebrew/bin/ffprobe",
					"/opt/local/bin/ffprobe",
					"/usr/bin/ffprobe",
				];

				#[cfg(target_os = "linux")]
				return vec![
					"/usr/bin/ffprobe",
					"/usr/local/bin/ffprobe",
					"/snap/bin/ffprobe",
				];
			}
			"yt-dlp" | "youtube-dl" => {
				#[cfg(target_os = "windows")]
				return vec![
					"C:\\yt-dlp\\yt-dlp.exe",
					"C:\\Program Files\\yt-dlp\\yt-dlp.exe",
					"C:\\youtube-dl\\youtube-dl.exe",
				];

				#[cfg(target_os = "macos")]
				return vec![
					"/usr/local/bin/yt-dlp",
					"/opt/homebrew/bin/yt-dlp",
					"/opt/local/bin/yt-dlp",
					"/usr/bin/yt-dlp",
					"/usr/local/bin/youtube-dl",
					"/opt/homebrew/bin/youtube-dl",
				];

				#[cfg(target_os = "linux")]
				return vec![
					"/usr/bin/yt-dlp",
					"/usr/local/bin/yt-dlp",
					"/snap/bin/yt-dlp",
					"/usr/bin/youtube-dl",
					"/usr/local/bin/youtube-dl",
				];
			}
			_ => vec![]
		}
	}
}

// ============= MAIN APPLICATION =============

struct LoMuxApp {
	media_files: Vec<MediaFile>,
	output_dir: Option<PathBuf>,
	selected_file_index: Option<usize>,

	selected_preset: EncodingPreset,
	presets: Vec<EncodingPreset>,
	preset_filter: PresetCategory,

	metadata_mode: MetadataMode,
	global_metadata: AudioMetadata,

	youtube_url_input: String,
	youtube_format: YtFormat,
	temp_download_dir: PathBuf,

	console_output: Arc<Mutex<String>>,
	is_processing: Arc<Mutex<bool>>,
	progress: Arc<Mutex<f32>>,
	status: Arc<Mutex<String>>,

	tools: ToolDetector,

	dark_mode: bool,
	extra_args: String,
}

#[derive(Debug, Clone, PartialEq)]
enum MetadataMode {
	None,
	Global,
	PerFile,
	Selected,
}

#[derive(Debug, Clone, PartialEq)]
enum YtFormat {
	BestVideo,
	BestAudio,
	Mp4_1080p,
	Mp4_720p,
	Mp3,
}

impl LoMuxApp {
	fn new() -> Self {
		let presets = EncodingPreset::get_all_presets();
		let temp_dir = std::env::temp_dir().join("lomux_downloads");
		std::fs::create_dir_all(&temp_dir).ok();

		Self {
			media_files: Vec::new(),
			output_dir: None,
			selected_file_index: None,
			selected_preset: EncodingPreset::youtube_1080p(),
			presets,
			preset_filter: PresetCategory::WebSocial,
			metadata_mode: MetadataMode::None,
			global_metadata: AudioMetadata::default(),
			youtube_url_input: String::new(),
			youtube_format: YtFormat::BestVideo,
			temp_download_dir: temp_dir,
			console_output: Arc::new(Mutex::new(String::new())),
			is_processing: Arc::new(Mutex::new(false)),
			progress: Arc::new(Mutex::new(0.0)),
			status: Arc::new(Mutex::new("Ready".to_string())),
			tools: ToolDetector::new(),
			dark_mode: true,
			extra_args: String::new(),
		}
	}

	fn can_process(&self) -> bool {
		!self.media_files.is_empty()
			&& self.output_dir.is_some()
			&& self.tools.ffmpeg_path.is_some()
			&& !*self.is_processing.lock().unwrap()
	}

	fn select_input_files(&mut self) {
		if let Some(files) = rfd::FileDialog::new()
			.set_title("Select Media Files")
			.pick_files()
		{
			for file in files {
				self.media_files.push(MediaFile::new(file));
			}
		}
	}

	fn add_youtube_url(&mut self) {
		if !self.youtube_url_input.is_empty() && self.tools.ytdlp_path.is_some() {
			self.media_files.push(MediaFile::new_youtube(self.youtube_url_input.clone()));
			self.youtube_url_input.clear();
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

	fn start_processing(&mut self) {
		if !self.can_process() {
			return;
		}

		*self.progress.lock().unwrap() = 0.0;
		*self.is_processing.lock().unwrap() = true;
		*self.console_output.lock().unwrap() = String::new();

		let files = self.media_files.clone();
		let output_dir = self.output_dir.clone().unwrap();
		let preset = self.selected_preset.clone();
		let ffmpeg = self.tools.ffmpeg_path.clone().unwrap();
		let ffprobe = self.tools.ffprobe_path.clone();
		let ytdlp = self.tools.ytdlp_path.clone();
		let console = self.console_output.clone();
		let progress = self.progress.clone();
		let status = self.status.clone();
		let is_processing = self.is_processing.clone();
		let temp_dir = self.temp_download_dir.clone();
		let metadata_mode = self.metadata_mode.clone();
		let global_metadata = self.global_metadata.clone();
		let youtube_format = self.youtube_format.clone();

		thread::spawn(move || {
			let total = files.len();
			let mut processed = 0;

			for (idx, mut file) in files.into_iter().enumerate() {
				let current = idx + 1;

				if file.is_youtube {
					if let Some(ref ytdlp_path) = ytdlp {
						*status.lock().unwrap() = format!("Downloading YouTube video {}/{}...", current, total);

						if let Some(ref url) = file.youtube_url {
							let downloaded_path = download_youtube_video(
								ytdlp_path,
								url,
								&temp_dir,
								&youtube_format,
								&console,
								&progress,
							);

							if let Some(path) = downloaded_path {
								file.path = path;
								file.download_status = DownloadStatus::Downloaded;
								console.lock().unwrap().push_str(&format!("✅ Downloaded: {}\n", file.path.display()));
							} else {
								file.download_status = DownloadStatus::Failed;
								console.lock().unwrap().push_str(&format!("❌ Failed to download: {}\n", url));
								continue;
							}
						}
					} else {
						console.lock().unwrap().push_str("❌ yt-dlp not found, skipping YouTube URL\n");
						continue;
					}
				}

				if !file.path.exists() {
					console.lock().unwrap().push_str(&format!("❌ File not found: {}\n", file.path.display()));
					continue;
				}

				*status.lock().unwrap() = format!(
					"Processing {}/{}: {}",
					current,
					total,
					file.display_name()
				);

				let stem = file.path.file_stem().unwrap().to_string_lossy();
				let output = output_dir.join(format!("{}.{}", stem, preset.get_extension()));

				let metadata = match metadata_mode {
					MetadataMode::Global => &global_metadata,
					MetadataMode::PerFile if file.apply_metadata => &file.metadata,
					_ => &AudioMetadata::default(),
				};

				let duration = if let Some(ref ffprobe) = ffprobe {
					get_duration(ffprobe, &file.path).unwrap_or(0.0)
				} else {
					0.0
				};

				let args = build_ffmpeg_args(&preset, &file.path, &output, metadata);

				console.lock().unwrap().push_str(&format!(
					"\n=== Converting {} ({}/{}) ===\n",
					file.display_name(),
					current,
					total
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
									let file_progress = (ms as f64 / 1000.0 / duration).min(1.0);
									let total_progress = (processed as f32 + file_progress as f32) / total as f32 * 100.0;
									*progress.lock().unwrap() = total_progress;
								}
							}
						}
					}
				}

				let _ = child.wait();
				
				if output.exists() {
					console.lock().unwrap().push_str(&format!("✅ Created: {}\n", output.display()));
				} else {
					console.lock().unwrap().push_str(&format!("❌ Failed to create output file\n"));
				}
				
				processed += 1;
				*progress.lock().unwrap() = (processed as f32 / total as f32) * 100.0;

				if file.is_youtube && file.path.exists() {
					if let Err(e) = std::fs::remove_file(&file.path) {
						console.lock().unwrap().push_str(&format!("⚠️  Could not remove temp file: {}\n", e));
					} else {
						console.lock().unwrap().push_str(&format!("🗑️  Cleaned up temp file: {}\n", file.path.display()));
					}
				}
			}

			*status.lock().unwrap() = "All tasks complete!".to_string();
			*progress.lock().unwrap() = 100.0;
			console.lock().unwrap().push_str("\n✅ All tasks complete!\n");
			*is_processing.lock().unwrap() = false;
		});
	}

	fn show_header(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			ui.heading("🎬 LoMux");
			ui.separator();
			ui.label("v1.0.2");
			ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
				let icon = if self.dark_mode { "🌙" } else { "☀" };
				if ui.button(icon).clicked() {
					self.dark_mode = !self.dark_mode;
				}
			});
		});
	}

	fn show_files_panel(&mut self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.strong("📁 Files & Sources");
			ui.separator();

			ui.horizontal(|ui| {
				if ui.button("➕ Add Files").clicked() {
					self.select_input_files();
				}

				if self.tools.ytdlp_path.is_some() {
					ui.separator();
					ui.add(egui::TextEdit::singleline(&mut self.youtube_url_input)
						.desired_width(200.0)
						.hint_text("Paste Youtube URL"));

					egui::ComboBox::from_id_salt("yt_format")
						.selected_text(match self.youtube_format {
							YtFormat::BestVideo => "Best Video",
							YtFormat::BestAudio => "Audio Only",
							YtFormat::Mp4_1080p => "1080p",
							YtFormat::Mp4_720p => "720p",
							YtFormat::Mp3 => "MP3",
						})
						.show_ui(ui, |ui| {
							ui.selectable_value(&mut self.youtube_format, YtFormat::BestVideo, "Best Video");
							ui.selectable_value(&mut self.youtube_format, YtFormat::Mp4_1080p, "1080p");
							ui.selectable_value(&mut self.youtube_format, YtFormat::Mp4_720p, "720p");
							ui.selectable_value(&mut self.youtube_format, YtFormat::BestAudio, "Audio Only");
							ui.selectable_value(&mut self.youtube_format, YtFormat::Mp3, "MP3");
						});

					if ui.button("➕ Add").on_hover_text("Add YouTube URL to queue").clicked() {
						self.add_youtube_url();
					}
				}

				ui.separator();
				if ui.button("📂 Output Folder").clicked() {
					self.select_output_dir();
				}
			});

			if let Some(ref dir) = self.output_dir {
				ui.label(format!("Output: {}", dir.display()));
			}

			ui.separator();

			if !self.media_files.is_empty() {
				ui.label(format!("{} file(s) in queue:", self.media_files.len()));

				egui::ScrollArea::vertical()
					.id_salt("files")
					.max_height(150.0)
					.show(ui, |ui| {
						ui.set_min_width(ui.available_width() - 10.0);

						for (idx, file) in self.media_files.iter_mut().enumerate() {
							ui.horizontal(|ui| {
								let is_selected = self.selected_file_index == Some(idx);
								if ui.checkbox(&mut file.apply_metadata, "").clicked() {
									if file.apply_metadata {
										self.selected_file_index = Some(idx);
									}
								}

								let display = if file.is_youtube {
									match file.download_status {
										DownloadStatus::Downloading(pct) => {
											format!("📺 Downloading... {:.0}%", pct)
										}
										DownloadStatus::Downloaded => {
											format!("📺 ✅ Ready: {}", file.youtube_url.as_ref().unwrap())
										}
										_ => format!("📺 {}", file.youtube_url.as_ref().unwrap()),
									}
								} else {
									format!("📄 {}", file.path.file_name().unwrap_or_default().to_string_lossy())
								};

								if ui.selectable_label(is_selected, display).clicked() {
									self.selected_file_index = Some(idx);
								}

								if ui.small_button("📝").on_hover_text("Edit metadata").clicked() {
									self.selected_file_index = Some(idx);
									self.metadata_mode = MetadataMode::PerFile;
								}
							});
						}
					});

				ui.horizontal(|ui| {
					if ui.small_button("🗑 Clear All").clicked() {
						self.media_files.clear();
						self.selected_file_index = None;
					}
					if ui.small_button("🗑 Remove Selected").clicked() {
						if let Some(idx) = self.selected_file_index {
							if idx < self.media_files.len() {
								self.media_files.remove(idx);
								self.selected_file_index = None;
							}
						}
					}
				});
			} else {
				ui.colored_label(egui::Color32::DARK_GRAY, "No files selected");
			}
		});
	}

	fn show_presets_panel(&mut self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.strong("🎯 Encoding Presets");
			ui.separator();

			ui.horizontal(|ui| {
				ui.label("Category:");
				for category in &[
					PresetCategory::WebSocial,
					PresetCategory::Device,
					PresetCategory::Professional,
					PresetCategory::Audio,
					PresetCategory::MatchSource,
					PresetCategory::Custom,
				] {
					if ui.selectable_label(
						self.preset_filter == *category,
						match category {
							PresetCategory::WebSocial => "Web & Social",
							PresetCategory::Device => "Devices",
							PresetCategory::Professional => "Professional",
							PresetCategory::Audio => "Audio",
							PresetCategory::MatchSource => "Match Source",
							PresetCategory::Custom => "Custom",
						}
					).clicked() {
						self.preset_filter = category.clone();
					}
				}
			});

			ui.separator();

			egui::ScrollArea::vertical()
				.id_salt("presets")
				.max_width(300.0)
				.max_height(120.0)
				.show(ui, |ui| {
					for preset in &self.presets {
						if preset.category == self.preset_filter {
							let is_selected = self.selected_preset.name == preset.name;
							if ui.selectable_label(is_selected, &preset.name).clicked() {
								self.selected_preset = preset.clone();
							}
							if is_selected {
								ui.label(format!("  ↳ {}", preset.description));
							}
						}
					}
				});

			if self.preset_filter == PresetCategory::Custom {
				ui.separator();
				ui.label("Custom Settings:");

				ui.columns(2, |columns| {
					columns[0].label("Container:");
					egui::ComboBox::from_id_salt("container")
						.selected_text(format!("{:?}", self.selected_preset.container))
						.show_ui(&mut columns[0], |ui| {
							ui.selectable_value(&mut self.selected_preset.container, Container::Mp4, "MP4");
							ui.selectable_value(&mut self.selected_preset.container, Container::Mkv, "MKV");
							ui.selectable_value(&mut self.selected_preset.container, Container::Webm, "WebM");
							ui.selectable_value(&mut self.selected_preset.container, Container::Mp3, "MP3");
							ui.selectable_value(&mut self.selected_preset.container, Container::Flac, "FLAC");
						});

					columns[1].label("Video Codec:");
					egui::ComboBox::from_id_salt("vcodec")
						.selected_text(format!("{:?}", self.selected_preset.video_codec))
						.show_ui(&mut columns[1], |ui| {
							ui.selectable_value(&mut self.selected_preset.video_codec, VideoCodec::H264, "H.264");
							ui.selectable_value(&mut self.selected_preset.video_codec, VideoCodec::H265, "H.265");
							ui.selectable_value(&mut self.selected_preset.video_codec, VideoCodec::Vp9, "VP9");
							ui.selectable_value(&mut self.selected_preset.video_codec, VideoCodec::Av1, "AV1");
							ui.selectable_value(&mut self.selected_preset.video_codec, VideoCodec::Copy, "Copy");
							ui.selectable_value(&mut self.selected_preset.video_codec, VideoCodec::None, "None");
						});
				});

				ui.horizontal(|ui| {
					ui.label("Bitrate:");
					if let Some(ref mut vb) = self.selected_preset.video_bitrate {
						ui.add(egui::DragValue::new(vb).suffix(" kbps"));
					}
					ui.separator();
					ui.label("CRF:");
					if let Some(ref mut crf) = self.selected_preset.video_crf {
						ui.add(egui::DragValue::new(crf).range(0..=51));
					}
				});
			}
		});
	}

	fn show_metadata_panel(&mut self, ui: &mut egui::Ui) {
		ui.collapsing("🎵 Metadata Editor", |ui| {
			ui.horizontal(|ui| {
				ui.label("Apply to:");
				ui.radio_value(&mut self.metadata_mode, MetadataMode::None, "None");
				ui.radio_value(&mut self.metadata_mode, MetadataMode::Global, "All Files");
				ui.radio_value(&mut self.metadata_mode, MetadataMode::Selected, "Selected Only");
				ui.radio_value(&mut self.metadata_mode, MetadataMode::PerFile, "Per File");
			});

			ui.separator();

			if self.metadata_mode == MetadataMode::PerFile {
				if let Some(idx) = self.selected_file_index {
					if idx < self.media_files.len() {
						let filename = self.media_files[idx].display_name();
						
						ui.horizontal(|ui| {
							ui.label("Title:");
							let mut title = self.media_files[idx].metadata.title.clone().unwrap_or_default();
							if ui.text_edit_singleline(&mut title).changed() {
								self.media_files[idx].metadata.title = if title.is_empty() { None } else { Some(title) };
							}
							if ui.small_button("📋").on_hover_text("From filename").clicked() {
								self.media_files[idx].metadata = AudioMetadata::from_filename(&filename);
							}
						});

						ui.horizontal(|ui| {
							ui.label("Artist:");
							let mut artist = self.media_files[idx].metadata.artist.clone().unwrap_or_default();
							if ui.text_edit_singleline(&mut artist).changed() {
								self.media_files[idx].metadata.artist = if artist.is_empty() { None } else { Some(artist) };
							}
						});

						ui.horizontal(|ui| {
							ui.label("Album:");
							let mut album = self.media_files[idx].metadata.album.clone().unwrap_or_default();
							if ui.text_edit_singleline(&mut album).changed() {
								self.media_files[idx].metadata.album = if album.is_empty() { None } else { Some(album) };
							}
						});

						ui.columns(3, |columns| {
							columns[0].label("Year:");
							let mut year = self.media_files[idx].metadata.year.clone().unwrap_or_default();
							if columns[0].text_edit_singleline(&mut year).changed() {
								self.media_files[idx].metadata.year = if year.is_empty() { None } else { Some(year) };
							}

							columns[1].label("Track:");
							let mut track = self.media_files[idx].metadata.track.clone().unwrap_or_default();
							if columns[1].text_edit_singleline(&mut track).changed() {
								self.media_files[idx].metadata.track = if track.is_empty() { None } else { Some(track) };
							}

							columns[2].label("Genre:");
							let mut genre = self.media_files[idx].metadata.genre.clone().unwrap_or_default();
							if columns[2].text_edit_singleline(&mut genre).changed() {
								self.media_files[idx].metadata.genre = if genre.is_empty() { None } else { Some(genre) };
							}
						});

						ui.horizontal(|ui| {
							if ui.button("Clear All").clicked() {
								self.media_files[idx].metadata.clear();
							}
							if ui.button("Copy to All").clicked() {
								let metadata = self.media_files[idx].metadata.clone();
								for file in &mut self.media_files {
									file.metadata = metadata.clone();
									file.apply_metadata = true;
								}
							}
						});
					}
				}
			} else {
				ui.horizontal(|ui| {
					ui.label("Title:");
					let mut title = self.global_metadata.title.clone().unwrap_or_default();
					if ui.text_edit_singleline(&mut title).changed() {
						self.global_metadata.title = if title.is_empty() { None } else { Some(title) };
					}
				});

				ui.horizontal(|ui| {
					ui.label("Artist:");
					let mut artist = self.global_metadata.artist.clone().unwrap_or_default();
					if ui.text_edit_singleline(&mut artist).changed() {
						self.global_metadata.artist = if artist.is_empty() { None } else { Some(artist) };
					}
				});

				ui.horizontal(|ui| {
					ui.label("Album:");
					let mut album = self.global_metadata.album.clone().unwrap_or_default();
					if ui.text_edit_singleline(&mut album).changed() {
						self.global_metadata.album = if album.is_empty() { None } else { Some(album) };
					}
				});

				ui.columns(3, |columns| {
					columns[0].label("Year:");
					let mut year = self.global_metadata.year.clone().unwrap_or_default();
					if columns[0].text_edit_singleline(&mut year).changed() {
						self.global_metadata.year = if year.is_empty() { None } else { Some(year) };
					}

					columns[1].label("Track:");
					let mut track = self.global_metadata.track.clone().unwrap_or_default();
					if columns[1].text_edit_singleline(&mut track).changed() {
						self.global_metadata.track = if track.is_empty() { None } else { Some(track) };
					}

					columns[2].label("Genre:");
					let mut genre = self.global_metadata.genre.clone().unwrap_or_default();
					if columns[2].text_edit_singleline(&mut genre).changed() {
						self.global_metadata.genre = if genre.is_empty() { None } else { Some(genre) };
					}
				});

				ui.horizontal(|ui| {
					if ui.button("Clear All").clicked() {
						self.global_metadata.clear();
					}
				});
			}
		});
	}

	fn show_controls(&mut self, ui: &mut egui::Ui) {
		ui.separator();

		ui.horizontal(|ui| {
			let is_processing = *self.is_processing.lock().unwrap();
			ui.add_enabled_ui(!is_processing && self.can_process(), |ui| {
				if ui.button("▶ Start Processing").clicked() {
					self.start_processing();
				}
			});

			if is_processing {
				if ui.button("⏹ Stop").clicked() {
					// TODO: Implement stop functionality
				}
			}

			let progress = *self.progress.lock().unwrap();
			ui.add(egui::ProgressBar::new(progress / 100.0)
				.show_percentage()
				.animate(is_processing));
		});

		ui.label(self.status.lock().unwrap().clone());

		ui.horizontal(|ui| {
			if self.tools.ffmpeg_path.is_none() {
				ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠ FFmpeg not found");
			} else {
				ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "✅ FFmpeg");
			}

			if self.tools.ytdlp_path.is_none() {
				ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "⚠ yt-dlp not found");
			} else {
				ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "✅ yt-dlp");
			}
		});
	}

	fn show_console(&self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.set_min_height(ui.available_height());

			ui.strong("📋 Console Output");
			ui.separator();

			let available_height = ui.available_height();

			egui::ScrollArea::vertical()
				.id_salt("console")
				.auto_shrink(false)
				.stick_to_bottom(true)
				.max_height(available_height)
				.show(ui, |ui| {
					ui.set_min_width(ui.available_width() - 10.0);
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

// ============= HELPER FUNCTIONS =============

fn build_ffmpeg_args(
	preset: &EncodingPreset,
	input: &Path,
	output: &Path,
	metadata: &AudioMetadata,
) -> Vec<String> {
	let mut args = vec![
		"-hide_banner".to_string(),
		"-threads".to_string(),
		"0".to_string(),
		"-progress".to_string(),
		"pipe:1".to_string(),
		"-nostats".to_string(),
		"-i".to_string(),
		input.to_string_lossy().to_string(),
	];

	match preset.video_codec {
		VideoCodec::H264 => {
			args.extend(["-c:v".to_string(), "libx264".to_string()]);
			if let Some(crf) = preset.video_crf {
				args.extend(["-crf".to_string(), crf.to_string()]);
			} else if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".to_string(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::H265 => {
			args.extend(["-c:v".to_string(), "libx265".to_string()]);
			if let Some(crf) = preset.video_crf {
				args.extend(["-crf".to_string(), crf.to_string()]);
			} else if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".to_string(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::Vp8 => {
			args.extend(["-c:v".to_string(), "libvpx".to_string()]);
			if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".to_string(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::Vp9 => {
			args.extend(["-c:v".to_string(), "libvpx-vp9".to_string()]);
			if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".to_string(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::Av1 => {
			args.extend(["-c:v".to_string(), "libaom-av1".to_string()]);
			if let Some(crf) = preset.video_crf {
				args.extend(["-crf".to_string(), crf.to_string()]);
			}
		}
		VideoCodec::ProRes => {
			args.extend(["-c:v".to_string(), "prores_ks".to_string()]);
			args.extend(["-profile:v".to_string(), "3".to_string()]);
		}
		VideoCodec::DnxHd => {
			args.extend(["-c:v".to_string(), "dnxhd".to_string()]);
			if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".to_string(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::Copy => {
			args.extend(["-c:v".to_string(), "copy".to_string()]);
		}
		VideoCodec::None => {
			args.extend(["-vn".to_string()]);
		}
	}

	match preset.audio_codec {
		AudioCodec::Aac => {
			args.extend(["-c:a".to_string(), "aac".to_string()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".to_string(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Mp3 => {
			args.extend(["-c:a".to_string(), "libmp3lame".to_string()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".to_string(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Opus => {
			args.extend(["-c:a".to_string(), "libopus".to_string()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".to_string(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Flac => {
			args.extend(["-c:a".to_string(), "flac".to_string()]);
		}
		AudioCodec::Vorbis => {
			args.extend(["-c:a".to_string(), "libvorbis".to_string()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".to_string(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Pcm => {
			args.extend(["-c:a".to_string(), "pcm_s16le".to_string()]);
		}
		AudioCodec::Copy => {
			args.extend(["-c:a".to_string(), "copy".to_string()]);
		}
		AudioCodec::None => {
			args.extend(["-an".to_string()]);
		}
	}

	if let Some(ref resolution) = preset.resolution {
		args.extend(["-vf".to_string(), format!("scale={}", resolution)]);
	}

	if let Some(fps) = preset.fps {
		args.extend(["-r".to_string(), fps.to_string()]);
	}

	if !metadata.is_empty() {
		if let Some(ref title) = metadata.title {
			args.extend(["-metadata".to_string(), format!("title={}", title)]);
		}
		if let Some(ref artist) = metadata.artist {
			args.extend(["-metadata".to_string(), format!("artist={}", artist)]);
		}
		if let Some(ref album) = metadata.album {
			args.extend(["-metadata".to_string(), format!("album={}", album)]);
		}
		if let Some(ref year) = metadata.year {
			args.extend(["-metadata".to_string(), format!("date={}", year)]);
		}
		if let Some(ref genre) = metadata.genre {
			args.extend(["-metadata".to_string(), format!("genre={}", genre)]);
		}
		if let Some(ref track) = metadata.track {
			args.extend(["-metadata".to_string(), format!("track={}", track)]);
		}
		if let Some(ref comment) = metadata.comment {
			args.extend(["-metadata".to_string(), format!("comment={}", comment)]);
		}
	}

	args.extend(["-y".to_string(), output.to_string_lossy().to_string()]);
	args
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

fn download_youtube_video(
	ytdlp: &Path,
	url: &str,
	temp_dir: &Path,
	format: &YtFormat,
	console: &Arc<Mutex<String>>,
	_progress: &Arc<Mutex<f32>>,
) -> Option<PathBuf> {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let output_template = temp_dir.join(format!("yt_{}_{}.%(ext)s", timestamp, "%(title)s"))
		.to_string_lossy()
		.to_string();

	console.lock().unwrap().push_str(&format!("Downloading to temp directory: {}\n", temp_dir.display()));

	let mut args = vec![
		"-o".to_string(), output_template.clone(),
		"--progress".to_string(),
		"--newline".to_string(),
		"--print".to_string(), "after_move:filepath".to_string(),
	];

	match format {
		YtFormat::BestVideo => {
			args.extend(["-f".to_string(), "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".to_string()]);
		}
		YtFormat::Mp4_1080p => {
			args.extend(["-f".to_string(), "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/best[height<=1080][ext=mp4]".to_string()]);
		}
		YtFormat::Mp4_720p => {
			args.extend(["-f".to_string(), "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/best[height<=720][ext=mp4]".to_string()]);
		}
		YtFormat::BestAudio => {
			args.extend(["-x".to_string(), "-f".to_string(), "bestaudio".to_string()]);
		}
		YtFormat::Mp3 => {
			args.extend(["-x".to_string(), "--audio-format".to_string(), "mp3".to_string()]);
		}
	}

	args.push(url.to_string());

	let mut child = Command::new(ytdlp)
		.args(&args)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.ok()?;

	let stdout = child.stdout.take()?;
	let reader = BufReader::new(stdout);

	let mut downloaded_file = None;

	for line in reader.lines() {
		if let Ok(line) = line {
			console.lock().unwrap().push_str(&line);
			console.lock().unwrap().push('\n');

			if line.contains("[download]") && line.contains("%") {
				if let Some(pct_str) = line.split('%').next() {
					if let Some(pct_str) = pct_str.split_whitespace().last() {
						if let Ok(_pct) = pct_str.parse::<f32>() {
						}
					}
				}
			}

			if !line.contains("[") && temp_dir.join(&line).exists() {
				downloaded_file = Some(PathBuf::from(line.trim()));
			} else if line.contains("has already been downloaded") {
				if let Some(path_part) = line.split("has already been downloaded").next() {
					let clean_path = path_part.replace("[download]", "").trim().to_string();
					if temp_dir.join(&clean_path).exists() {
						downloaded_file = Some(temp_dir.join(clean_path));
					}
				}
			} else if line.starts_with(&temp_dir.to_string_lossy().to_string()) {
				downloaded_file = Some(PathBuf::from(line.trim()));
			}
		}
	}

	let status = child.wait().ok()?;
	
	if status.success() {
		if let Some(ref file) = downloaded_file {
			console.lock().unwrap().push_str(&format!("✅ Downloaded: {}\n", file.display()));
		}
		downloaded_file
	} else {
		console.lock().unwrap().push_str("❌ Download failed\n");
		None
	}
}

// ============= APP IMPLEMENTATION =============

impl eframe::App for LoMuxApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		let visuals = if self.dark_mode {
			egui::Visuals::dark()
		} else {
			egui::Visuals::light()
		};
		ctx.set_visuals(visuals);

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
			ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);

			egui::Frame::none()
				.inner_margin(egui::Margin::symmetric(12.0, 8.0))
				.show(ui, |ui| {
					self.show_header(ui);
				});

			ui.separator();

			let available_height = ui.available_height();

			egui::Frame::none()
				.inner_margin(egui::Margin::symmetric(12.0, 8.0))
				.show(ui, |ui| {
					ui.horizontal(|ui| {
						ui.set_height(available_height - 20.0);

						ui.vertical(|ui| {
							ui.set_min_width(400.0);
							ui.set_max_width(500.0);
							ui.set_height(available_height - 20.0);

							egui::ScrollArea::vertical()
								.id_salt("controls")
								.max_height(available_height - 20.0)
								.show(ui, |ui| {
									self.show_files_panel(ui);
									ui.add_space(8.0);
									self.show_presets_panel(ui);
									ui.add_space(8.0);
									self.show_metadata_panel(ui);
									ui.add_space(8.0);

									ui.collapsing("🔧 Advanced", |ui| {
										ui.label("Extra FFmpeg arguments:");
										ui.text_edit_singleline(&mut self.extra_args);
									});

									ui.add_space(8.0);
									self.show_controls(ui);
								});
						});

						ui.separator();

						ui.vertical(|ui| {
							ui.set_min_height(available_height - 20.0);
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
			.with_inner_size([1000.0, 700.0])
			.with_min_inner_size([800.0, 500.0]),
		..Default::default()
	};

	eframe::run_native(
		"LoMux v1.0.2",
		options,
		Box::new(|_cc| Ok(Box::new(LoMuxApp::new()))),
	)
}
