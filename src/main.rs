#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{BufRead, BufReader};

const VERSION: &str = "1.1.0";

// ============= PRESET SYSTEM =============

#[derive(Debug, Clone, PartialEq)]
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

impl Container {
	fn extension(&self) -> &str {
		match self {
			Self::Mp4 => "mp4",
			Self::Mkv => "mkv",
			Self::Webm => "webm",
			Self::Mp3 => "mp3",
			Self::Flac => "flac",
			Self::Aac => "aac",
			Self::Opus => "opus",
			Self::Wav => "wav",
			Self::Gif => "gif",
			Self::Avi => "avi",
			Self::Mov => "mov",
		}
	}

	fn label(&self) -> &str {
		match self {
			Self::Mp4 => "MP4",
			Self::Mkv => "MKV",
			Self::Webm => "WebM",
			Self::Mp3 => "MP3",
			Self::Flac => "FLAC",
			Self::Aac => "AAC",
			Self::Opus => "Opus",
			Self::Wav => "WAV",
			Self::Gif => "GIF",
			Self::Avi => "AVI",
			Self::Mov => "MOV",
		}
	}

	fn all() -> &'static [Self] {
		&[
			Self::Mp4, Self::Mkv, Self::Webm, Self::Mov, Self::Avi,
			Self::Mp3, Self::Flac, Self::Aac, Self::Opus, Self::Wav, Self::Gif,
		]
	}
}

#[derive(Debug, Clone, PartialEq)]
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

impl VideoCodec {
	fn label(&self) -> &str {
		match self {
			Self::H264 => "H.264",
			Self::H265 => "H.265/HEVC",
			Self::Vp8 => "VP8",
			Self::Vp9 => "VP9",
			Self::Av1 => "AV1",
			Self::ProRes => "ProRes",
			Self::DnxHd => "DNxHD",
			Self::Copy => "Copy",
			Self::None => "None",
		}
	}

	fn all() -> &'static [Self] {
		&[
			Self::H264, Self::H265, Self::Vp9, Self::Av1,
			Self::ProRes, Self::DnxHd, Self::Vp8, Self::Copy, Self::None,
		]
	}
}

#[derive(Debug, Clone, PartialEq)]
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

impl AudioCodec {
	fn label(&self) -> &str {
		match self {
			Self::Aac => "AAC",
			Self::Mp3 => "MP3",
			Self::Opus => "Opus",
			Self::Flac => "FLAC",
			Self::Vorbis => "Vorbis",
			Self::Pcm => "PCM",
			Self::Copy => "Copy",
			Self::None => "None",
		}
	}

	fn all() -> &'static [Self] {
		&[
			Self::Aac, Self::Mp3, Self::Opus, Self::Flac,
			Self::Vorbis, Self::Pcm, Self::Copy, Self::None,
		]
	}
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

impl PresetCategory {
	fn label(&self) -> &str {
		match self {
			Self::WebSocial => "Web & Social",
			Self::Device => "Devices",
			Self::Professional => "Professional",
			Self::Audio => "Audio",
			Self::MatchSource => "Match Source",
			Self::Custom => "Custom",
		}
	}

	fn all() -> &'static [Self] {
		&[
			Self::WebSocial, Self::Device, Self::Professional,
			Self::Audio, Self::MatchSource, Self::Custom,
		]
	}
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
	fn info_line(&self) -> String {
		let mut parts = Vec::new();
		parts.push(self.container.label().to_string());
		if self.video_codec != VideoCodec::None {
			parts.push(self.video_codec.label().to_string());
		}
		if let Some(ref res) = self.resolution {
			parts.push(res.replace("x", "×"));
		}
		if let Some(vb) = self.video_bitrate {
			parts.push(format!("{}Mbps", vb / 1000));
		}
		if let Some(crf) = self.video_crf {
			parts.push(format!("CRF {}", crf));
		}
		if let Some(ab) = self.audio_bitrate {
			parts.push(format!("{}kbps audio", ab));
		}
		parts.join(" · ")
	}

	fn custom_default() -> Self {
		Self {
			name: "Custom".to_string(),
			category: PresetCategory::Custom,
			container: Container::Mp4,
			video_codec: VideoCodec::H264,
			audio_codec: AudioCodec::Aac,
			video_bitrate: Some(5000),
			audio_bitrate: Some(192),
			video_crf: None,
			fps: None,
			resolution: None,
			description: "User-defined settings".to_string(),
		}
	}

	fn get_all_presets() -> Vec<Self> {
		vec![
			// Web & Social
			Self {
				name: "YouTube 1080p HD".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(8000),
				audio_bitrate: Some(320),
				video_crf: None,
				fps: None,
				resolution: Some("1920x1080".into()),
				description: "Optimized for YouTube 1080p uploads".into(),
			},
			Self {
				name: "YouTube 4K UHD".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H265,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(35000),
				audio_bitrate: Some(320),
				video_crf: None,
				fps: None,
				resolution: Some("3840x2160".into()),
				description: "High quality 4K for YouTube".into(),
			},
			Self {
				name: "Instagram Feed".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(3500),
				audio_bitrate: Some(128),
				video_crf: None,
				fps: Some(30),
				resolution: Some("1080x1080".into()),
				description: "Square video for Instagram feed".into(),
			},
			Self {
				name: "Instagram Reels".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(5000),
				audio_bitrate: Some(128),
				video_crf: None,
				fps: Some(30),
				resolution: Some("1080x1920".into()),
				description: "Vertical video for Instagram Reels".into(),
			},
			Self {
				name: "TikTok".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(6000),
				audio_bitrate: Some(128),
				video_crf: None,
				fps: Some(30),
				resolution: Some("1080x1920".into()),
				description: "Vertical video for TikTok".into(),
			},
			Self {
				name: "Twitter/X".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(5000),
				audio_bitrate: Some(128),
				video_crf: None,
				fps: Some(30),
				resolution: Some("1280x720".into()),
				description: "Optimized for Twitter/X video".into(),
			},
			Self {
				name: "Discord".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(2500),
				audio_bitrate: Some(128),
				video_crf: None,
				fps: Some(30),
				resolution: Some("1280x720".into()),
				description: "Under 25MB for free Discord uploads".into(),
			},
			Self {
				name: "Web GIF".into(),
				category: PresetCategory::WebSocial,
				container: Container::Gif,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: Some(15),
				resolution: Some("480x-1".into()),
				description: "Animated GIF for web use".into(),
			},

			// Devices
			Self {
				name: "iPhone/iPad".into(),
				category: PresetCategory::Device,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(5000),
				audio_bitrate: Some(160),
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Compatible with all iOS devices".into(),
			},
			Self {
				name: "Android".into(),
				category: PresetCategory::Device,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(4000),
				audio_bitrate: Some(128),
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Compatible with Android devices".into(),
			},
			Self {
				name: "Apple TV 4K".into(),
				category: PresetCategory::Device,
				container: Container::Mp4,
				video_codec: VideoCodec::H265,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(20000),
				audio_bitrate: Some(256),
				video_crf: None,
				fps: None,
				resolution: Some("3840x2160".into()),
				description: "4K HEVC for Apple TV".into(),
			},
			Self {
				name: "Chromecast".into(),
				category: PresetCategory::Device,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(8000),
				audio_bitrate: Some(192),
				video_crf: None,
				fps: None,
				resolution: Some("1920x1080".into()),
				description: "Chromecast-compatible H.264".into(),
			},

			// Professional
			Self {
				name: "ProRes 422".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::ProRes,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Apple ProRes 422 for editing".into(),
			},
			Self {
				name: "DNxHD 1080p".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::DnxHd,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: Some(185000),
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: Some("1920x1080".into()),
				description: "Avid DNxHD for broadcast".into(),
			},
			Self {
				name: "Archive H.265".into(),
				category: PresetCategory::Professional,
				container: Container::Mkv,
				video_codec: VideoCodec::H265,
				audio_codec: AudioCodec::Flac,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: Some(16),
				fps: None,
				resolution: None,
				description: "High quality archival with lossless audio".into(),
			},

			// Audio
			Self {
				name: "MP3 320kbps".into(),
				category: PresetCategory::Audio,
				container: Container::Mp3,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Mp3,
				video_bitrate: None,
				audio_bitrate: Some(320),
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Highest quality MP3".into(),
			},
			Self {
				name: "MP3 192kbps".into(),
				category: PresetCategory::Audio,
				container: Container::Mp3,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Mp3,
				video_bitrate: None,
				audio_bitrate: Some(192),
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Standard quality MP3".into(),
			},
			Self {
				name: "FLAC Lossless".into(),
				category: PresetCategory::Audio,
				container: Container::Flac,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Flac,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Lossless FLAC audio".into(),
			},
			Self {
				name: "AAC 256kbps".into(),
				category: PresetCategory::Audio,
				container: Container::Aac,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Aac,
				video_bitrate: None,
				audio_bitrate: Some(256),
				video_crf: None,
				fps: None,
				resolution: None,
				description: "High quality AAC audio".into(),
			},
			Self {
				name: "Opus 128kbps".into(),
				category: PresetCategory::Audio,
				container: Container::Opus,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Opus,
				video_bitrate: None,
				audio_bitrate: Some(128),
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Efficient Opus audio".into(),
			},
			Self {
				name: "WAV Uncompressed".into(),
				category: PresetCategory::Audio,
				container: Container::Wav,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Uncompressed PCM audio".into(),
			},

			// Match Source
			Self {
				name: "Match Source - High".into(),
				category: PresetCategory::MatchSource,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: None,
				audio_bitrate: Some(256),
				video_crf: Some(18),
				fps: None,
				resolution: None,
				description: "High quality, larger file".into(),
			},
			Self {
				name: "Match Source - Medium".into(),
				category: PresetCategory::MatchSource,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: None,
				audio_bitrate: Some(192),
				video_crf: Some(23),
				fps: None,
				resolution: None,
				description: "Balanced quality and size".into(),
			},
			Self {
				name: "Match Source - Low".into(),
				category: PresetCategory::MatchSource,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: None,
				audio_bitrate: Some(128),
				video_crf: Some(28),
				fps: None,
				resolution: None,
				description: "Lower quality, smaller file".into(),
			},
			Self {
				name: "Remux (No Re-encode)".into(),
				category: PresetCategory::MatchSource,
				container: Container::Mp4,
				video_codec: VideoCodec::Copy,
				audio_codec: AudioCodec::Copy,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				description: "Copy streams, change container only".into(),
			},
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
#[allow(dead_code)]
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
			self.youtube_url.as_deref().unwrap_or("YouTube").to_string()
		} else {
			self.path.file_name()
				.unwrap_or_default()
				.to_string_lossy()
				.to_string()
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
		let stem = filename.rsplit_once('.').map(|(s, _)| s).unwrap_or(filename);
		let mut metadata = Self::default();

		if let Some((artist, title)) = stem.split_once(" - ") {
			metadata.artist = Some(artist.trim().to_string());
			metadata.title = Some(title.trim().to_string());
		} else {
			metadata.title = Some(stem.to_string());
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
		self.ytdlp_path = Self::find_tool("yt-dlp")
			.or_else(|| Self::find_tool("youtube-dl"));
	}

	fn find_tool(tool_name: &str) -> Option<PathBuf> {
		if let Ok(path) = which::which(tool_name) {
			return Some(path);
		}

		for path_str in Self::get_search_paths(tool_name) {
			let path = PathBuf::from(path_str);
			if path.exists() {
				return Some(path);
			}
		}

		#[cfg(target_os = "macos")]
		{
			if let Ok(output) = Command::new("brew").args(["--prefix"]).output() {
				let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
				let brew_path = PathBuf::from(format!("{}/bin/{}", prefix, tool_name));
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
	custom_preset: EncodingPreset,
	presets: Vec<EncodingPreset>,
	preset_filter: PresetCategory,

	metadata_mode: MetadataMode,
	global_metadata: AudioMetadata,

	youtube_url_input: String,
	youtube_format: YtFormat,
	temp_download_dir: PathBuf,

	console_output: Arc<Mutex<String>>,
	is_processing: Arc<Mutex<bool>>,
	cancel_flag: Arc<Mutex<bool>>,
	progress: Arc<Mutex<f32>>,
	status: Arc<Mutex<String>>,
	current_child: Arc<Mutex<Option<u32>>>,

	tools: ToolDetector,

	theme: AppTheme,
	theme_editor_open: bool,
	extra_args: String,
	show_preset_info: bool,
}

// ============= THEME SYSTEM =============

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ThemeConfig {
	name: String,
	dark: bool,
	accent: [u8; 3],
	bg_primary: [u8; 3],
	bg_secondary: [u8; 3],
	text_primary: [u8; 3],
	text_secondary: [u8; 3],
	rounding: f32,
}

impl From<&AppTheme> for ThemeConfig {
	fn from(t: &AppTheme) -> Self {
		Self {
			name: t.name.clone(),
			dark: t.dark,
			accent: [t.accent.r(), t.accent.g(), t.accent.b()],
			bg_primary: [t.bg_primary.r(), t.bg_primary.g(), t.bg_primary.b()],
			bg_secondary: [t.bg_secondary.r(), t.bg_secondary.g(), t.bg_secondary.b()],
			text_primary: [t.text_primary.r(), t.text_primary.g(), t.text_primary.b()],
			text_secondary: [t.text_secondary.r(), t.text_secondary.g(), t.text_secondary.b()],
			rounding: t.rounding,
		}
	}
}

impl From<ThemeConfig> for AppTheme {
	fn from(c: ThemeConfig) -> Self {
		Self {
			name: c.name,
			dark: c.dark,
			accent: egui::Color32::from_rgb(c.accent[0], c.accent[1], c.accent[2]),
			bg_primary: egui::Color32::from_rgb(c.bg_primary[0], c.bg_primary[1], c.bg_primary[2]),
			bg_secondary: egui::Color32::from_rgb(c.bg_secondary[0], c.bg_secondary[1], c.bg_secondary[2]),
			text_primary: egui::Color32::from_rgb(c.text_primary[0], c.text_primary[1], c.text_primary[2]),
			text_secondary: egui::Color32::from_rgb(c.text_secondary[0], c.text_secondary[1], c.text_secondary[2]),
			rounding: c.rounding,
		}
	}
}

fn config_dir() -> Option<PathBuf> {
	#[cfg(target_os = "macos")]
	{
		return dirs_next().map(|d| d.join("lomux"));
	}
	#[cfg(target_os = "windows")]
	{
		return std::env::var("APPDATA").ok().map(|d| PathBuf::from(d).join("lomux"));
	}
	#[cfg(target_os = "linux")]
	{
		return std::env::var("XDG_CONFIG_HOME")
			.ok()
			.map(PathBuf::from)
			.or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
			.map(|d| d.join("lomux"));
	}
	#[allow(unreachable_code)]
	None
}

#[cfg(target_os = "macos")]
fn dirs_next() -> Option<PathBuf> {
	std::env::var("HOME")
		.ok()
		.map(|h| PathBuf::from(h).join("Library").join("Application Support"))
}

fn config_path() -> Option<PathBuf> {
	config_dir().map(|d| d.join("config.json"))
}

fn save_theme(theme: &AppTheme) {
	if let Some(path) = config_path() {
		if let Some(dir) = path.parent() {
			let _ = std::fs::create_dir_all(dir);
		}
		let config = ThemeConfig::from(theme);
		if let Ok(json) = serde_json::to_string_pretty(&config) {
			let _ = std::fs::write(path, json);
		}
	}
}

fn load_theme() -> Option<AppTheme> {
	let path = config_path()?;
	let data = std::fs::read_to_string(path).ok()?;
	let config: ThemeConfig = serde_json::from_str(&data).ok()?;
	Some(AppTheme::from(config))
}

#[derive(Debug, Clone, PartialEq)]
struct AppTheme {
	name: String,
	dark: bool,
	accent: egui::Color32,
	bg_primary: egui::Color32,
	bg_secondary: egui::Color32,
	text_primary: egui::Color32,
	text_secondary: egui::Color32,
	rounding: f32,
}

impl AppTheme {
	fn studio_dark() -> Self {
		Self {
			name: "Studio Dark".into(),
			dark: true,
			accent: egui::Color32::from_rgb(110, 160, 255),
			bg_primary: egui::Color32::from_rgb(22, 22, 28),
			bg_secondary: egui::Color32::from_rgb(32, 32, 40),
			text_primary: egui::Color32::from_rgb(220, 220, 230),
			text_secondary: egui::Color32::from_rgb(130, 130, 150),
			rounding: 6.0,
		}
	}

	fn midnight() -> Self {
		Self {
			name: "Midnight".into(),
			dark: true,
			accent: egui::Color32::from_rgb(200, 130, 255),
			bg_primary: egui::Color32::from_rgb(18, 15, 28),
			bg_secondary: egui::Color32::from_rgb(28, 24, 42),
			text_primary: egui::Color32::from_rgb(225, 220, 240),
			text_secondary: egui::Color32::from_rgb(140, 130, 160),
			rounding: 8.0,
		}
	}

	fn warm_dark() -> Self {
		Self {
			name: "Warm Dark".into(),
			dark: true,
			accent: egui::Color32::from_rgb(255, 160, 90),
			bg_primary: egui::Color32::from_rgb(28, 24, 20),
			bg_secondary: egui::Color32::from_rgb(40, 34, 28),
			text_primary: egui::Color32::from_rgb(235, 225, 215),
			text_secondary: egui::Color32::from_rgb(160, 145, 130),
			rounding: 6.0,
		}
	}

	fn emerald() -> Self {
		Self {
			name: "Emerald".into(),
			dark: true,
			accent: egui::Color32::from_rgb(80, 220, 160),
			bg_primary: egui::Color32::from_rgb(18, 24, 22),
			bg_secondary: egui::Color32::from_rgb(26, 36, 32),
			text_primary: egui::Color32::from_rgb(215, 230, 225),
			text_secondary: egui::Color32::from_rgb(120, 150, 140),
			rounding: 6.0,
		}
	}

	fn clean_light() -> Self {
		Self {
			name: "Clean Light".into(),
			dark: false,
			accent: egui::Color32::from_rgb(50, 110, 220),
			bg_primary: egui::Color32::from_rgb(248, 248, 252),
			bg_secondary: egui::Color32::from_rgb(238, 238, 244),
			text_primary: egui::Color32::from_rgb(30, 30, 40),
			text_secondary: egui::Color32::from_rgb(110, 110, 130),
			rounding: 6.0,
		}
	}

	fn cream() -> Self {
		Self {
			name: "Cream".into(),
			dark: false,
			accent: egui::Color32::from_rgb(180, 100, 60),
			bg_primary: egui::Color32::from_rgb(252, 248, 240),
			bg_secondary: egui::Color32::from_rgb(242, 236, 224),
			text_primary: egui::Color32::from_rgb(50, 40, 30),
			text_secondary: egui::Color32::from_rgb(130, 115, 100),
			rounding: 8.0,
		}
	}

	fn all_presets() -> Vec<Self> {
		vec![
			Self::studio_dark(),
			Self::midnight(),
			Self::warm_dark(),
			Self::emerald(),
			Self::clean_light(),
			Self::cream(),
		]
	}

	fn apply(&self, ctx: &egui::Context) {
		let mut visuals = if self.dark {
			egui::Visuals::dark()
		} else {
			egui::Visuals::light()
		};

		visuals.override_text_color = Some(self.text_primary);
		visuals.panel_fill = self.bg_primary;
		visuals.window_fill = self.bg_secondary;
		visuals.extreme_bg_color = self.bg_secondary;
		visuals.faint_bg_color = if self.dark {
			lighten(self.bg_secondary, 8)
		} else {
			darken(self.bg_secondary, 6)
		};
		visuals.code_bg_color = self.bg_secondary;

		let rounding = egui::Rounding::same(self.rounding);
		let small_rounding = egui::Rounding::same((self.rounding * 0.5).max(2.0));

		visuals.widgets.noninteractive.bg_fill = self.bg_secondary;
		visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, self.text_secondary);
		visuals.widgets.noninteractive.rounding = rounding;
		visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
			0.5,
			if self.dark { lighten(self.bg_secondary, 20) } else { darken(self.bg_secondary, 15) }
		);

		visuals.widgets.inactive.bg_fill = if self.dark {
			lighten(self.bg_secondary, 12)
		} else {
			darken(self.bg_secondary, 8)
		};
		visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, self.text_primary);
		visuals.widgets.inactive.rounding = small_rounding;

		visuals.widgets.hovered.bg_fill = alpha_blend(self.accent, 50);
		visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, self.text_primary);
		visuals.widgets.hovered.rounding = small_rounding;

		visuals.widgets.active.bg_fill = alpha_blend(self.accent, 80);
		visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, self.text_primary);
		visuals.widgets.active.rounding = small_rounding;

		visuals.widgets.open.bg_fill = if self.dark {
			lighten(self.bg_secondary, 16)
		} else {
			darken(self.bg_secondary, 12)
		};
		visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, self.accent);
		visuals.widgets.open.rounding = small_rounding;

		visuals.selection.bg_fill = alpha_blend(self.accent, 60);
		visuals.selection.stroke = egui::Stroke::new(1.0, self.accent);

		visuals.hyperlink_color = self.accent;
		visuals.window_rounding = rounding;

		ctx.set_visuals(visuals);
	}
}

fn lighten(color: egui::Color32, amount: u8) -> egui::Color32 {
	egui::Color32::from_rgb(
		color.r().saturating_add(amount),
		color.g().saturating_add(amount),
		color.b().saturating_add(amount),
	)
}

fn darken(color: egui::Color32, amount: u8) -> egui::Color32 {
	egui::Color32::from_rgb(
		color.r().saturating_sub(amount),
		color.g().saturating_sub(amount),
		color.b().saturating_sub(amount),
	)
}

fn alpha_blend(color: egui::Color32, alpha: u8) -> egui::Color32 {
	egui::Color32::from_rgba_premultiplied(
		(color.r() as u16 * alpha as u16 / 255) as u8,
		(color.g() as u16 * alpha as u16 / 255) as u8,
		(color.b() as u16 * alpha as u16 / 255) as u8,
		alpha,
	)
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
			selected_preset: presets[0].clone(),
			custom_preset: EncodingPreset::custom_default(),
			presets,
			preset_filter: PresetCategory::WebSocial,
			metadata_mode: MetadataMode::None,
			global_metadata: AudioMetadata::default(),
			youtube_url_input: String::new(),
			youtube_format: YtFormat::BestVideo,
			temp_download_dir: temp_dir,
			console_output: Arc::new(Mutex::new(String::new())),
			is_processing: Arc::new(Mutex::new(false)),
			cancel_flag: Arc::new(Mutex::new(false)),
			progress: Arc::new(Mutex::new(0.0)),
			status: Arc::new(Mutex::new("Ready".to_string())),
			current_child: Arc::new(Mutex::new(None)),
			tools: ToolDetector::new(),
			theme: load_theme().unwrap_or_else(AppTheme::studio_dark),
			theme_editor_open: false,
			extra_args: String::new(),
			show_preset_info: true,
		}
	}

	fn active_preset(&self) -> &EncodingPreset {
		if self.preset_filter == PresetCategory::Custom {
			&self.custom_preset
		} else {
			&self.selected_preset
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
			.add_filter("Media Files", &[
				"mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "m4v",
				"mp3", "flac", "wav", "aac", "ogg", "opus", "m4a", "wma",
				"gif", "ts", "mts", "m2ts",
			])
			.add_filter("All Files", &["*"])
			.pick_files()
		{
			for file in files {
				self.media_files.push(MediaFile::new(file));
			}
		}
	}

	fn add_youtube_url(&mut self) {
		let url = self.youtube_url_input.trim().to_string();
		if !url.is_empty() && self.tools.ytdlp_path.is_some() {
			self.media_files.push(MediaFile::new_youtube(url));
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

	fn generate_output_path(output_dir: &Path, stem: &str, extension: &str) -> PathBuf {
		let candidate = output_dir.join(format!("{}.{}", stem, extension));
		if !candidate.exists() {
			return candidate;
		}
		for i in 1..1000 {
			let numbered = output_dir.join(format!("{}_{}.{}", stem, i, extension));
			if !numbered.exists() {
				return numbered;
			}
		}
		candidate
	}

	fn start_processing(&mut self) {
		if !self.can_process() {
			return;
		}

		*self.progress.lock().unwrap() = 0.0;
		*self.is_processing.lock().unwrap() = true;
		*self.cancel_flag.lock().unwrap() = false;
		*self.console_output.lock().unwrap() = String::new();

		let files = self.media_files.clone();
		let output_dir = self.output_dir.clone().unwrap();
		let preset = self.active_preset().clone();
		let ffmpeg = self.tools.ffmpeg_path.clone().unwrap();
		let ffprobe = self.tools.ffprobe_path.clone();
		let ytdlp = self.tools.ytdlp_path.clone();
		let console = self.console_output.clone();
		let progress = self.progress.clone();
		let status = self.status.clone();
		let is_processing = self.is_processing.clone();
		let cancel_flag = self.cancel_flag.clone();
		let current_child = self.current_child.clone();
		let temp_dir = self.temp_download_dir.clone();
		let metadata_mode = self.metadata_mode.clone();
		let global_metadata = self.global_metadata.clone();
		let youtube_format = self.youtube_format.clone();
		let extra_args = self.extra_args.clone();

		thread::spawn(move || {
			let total = files.len();
			let mut processed = 0;
			let mut succeeded = 0;
			let mut failed = 0;

			for (idx, mut file) in files.into_iter().enumerate() {
				if *cancel_flag.lock().unwrap() {
					console.lock().unwrap().push_str("\n⏹ Processing cancelled by user\n");
					break;
				}

				let current = idx + 1;

				if file.is_youtube {
					if let Some(ref ytdlp_path) = ytdlp {
						*status.lock().unwrap() = format!("Downloading {}/{}...", current, total);

						if let Some(ref url) = file.youtube_url {
							let downloaded_path = download_youtube_video(
								ytdlp_path,
								url,
								&temp_dir,
								&youtube_format,
								&console,
								&progress,
								&cancel_flag,
							);

							if let Some(path) = downloaded_path {
								file.path = path;
								file.download_status = DownloadStatus::Downloaded;
							} else {
								file.download_status = DownloadStatus::Failed;
								console.lock().unwrap().push_str(&format!("❌ Failed to download: {}\n", url));
								failed += 1;
								continue;
							}
						}
					} else {
						console.lock().unwrap().push_str("❌ yt-dlp not found, skipping YouTube URL\n");
						failed += 1;
						continue;
					}
				}

				if !file.path.exists() {
					console.lock().unwrap().push_str(&format!("❌ File not found: {}\n", file.path.display()));
					failed += 1;
					continue;
				}

				*status.lock().unwrap() = format!(
					"Converting {}/{}: {}",
					current, total, file.display_name()
				);

				let stem = file.path.file_stem().unwrap_or_default().to_string_lossy().to_string();
				let output = Self::generate_output_path(
					&output_dir,
					&stem,
					preset.container.extension(),
				);

				let metadata = match metadata_mode {
					MetadataMode::Global => &global_metadata,
					MetadataMode::PerFile if file.apply_metadata => &file.metadata,
					MetadataMode::Selected => {
						if file.apply_metadata { &file.metadata } else { &global_metadata }
					}
					_ => &AudioMetadata::default(),
				};

				let duration = ffprobe.as_ref()
					.and_then(|p| get_duration(p, &file.path).ok())
					.unwrap_or(0.0);

				let args = build_ffmpeg_args(&preset, &file.path, &output, metadata, &extra_args);

				console.lock().unwrap().push_str(&format!(
					"\n─── Converting {} ({}/{}) ───\n",
					file.display_name(), current, total
				));

				let child_result = Command::new(&ffmpeg)
					.args(&args)
					.stdout(Stdio::piped())
					.stderr(Stdio::piped())
					.spawn();

				let mut child = match child_result {
					Ok(c) => c,
					Err(e) => {
						console.lock().unwrap().push_str(&format!("❌ Failed to start ffmpeg: {}\n", e));
						failed += 1;
						continue;
					}
				};

				*current_child.lock().unwrap() = Some(child.id());

				let stderr = child.stderr.take().unwrap();
				let reader = BufReader::new(stderr);

				for line in reader.lines().flatten() {
					if *cancel_flag.lock().unwrap() {
						kill_process(&child);
						break;
					}

					if line.contains("out_time_ms=") && duration > 0.0 {
						if let Some(ms_str) = line.split("out_time_ms=").nth(1) {
							if let Ok(ms) = ms_str.trim().parse::<i64>() {
								let file_progress = (ms as f64 / 1_000_000.0 / duration).min(1.0);
								let total_progress = (processed as f32 + file_progress as f32) / total as f32 * 100.0;
								*progress.lock().unwrap() = total_progress;
							}
						}
					} else if line.starts_with("frame=") || line.starts_with("size=") || line.starts_with("speed=") {
						// progress lines — skip console noise
					} else if !line.trim().is_empty() && !line.starts_with("progress=") && !line.starts_with("stream_") && !line.starts_with("bitrate=") && !line.starts_with("total_size=") && !line.starts_with("out_time=") && !line.starts_with("dup_frames=") && !line.starts_with("drop_frames=") {
						console.lock().unwrap().push_str(&line);
						console.lock().unwrap().push('\n');
					}
				}

				*current_child.lock().unwrap() = None;

				let exit_status = child.wait();

				if output.exists() && exit_status.map(|s| s.success()).unwrap_or(false) {
					let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
					let size_str = format_file_size(size);
					console.lock().unwrap().push_str(&format!("✅ Created: {} ({})\n", output.display(), size_str));
					succeeded += 1;
				} else if !*cancel_flag.lock().unwrap() {
					console.lock().unwrap().push_str("❌ Failed to create output file\n");
					failed += 1;
				}

				processed += 1;
				*progress.lock().unwrap() = (processed as f32 / total as f32) * 100.0;

				if file.is_youtube && file.path.exists() {
					let _ = std::fs::remove_file(&file.path);
				}
			}

			let summary = format!(
				"\n═══ Complete: {} succeeded, {} failed ═══\n",
				succeeded, failed
			);
			console.lock().unwrap().push_str(&summary);
			*status.lock().unwrap() = format!("Done — {} succeeded, {} failed", succeeded, failed);
			*progress.lock().unwrap() = 100.0;
			*is_processing.lock().unwrap() = false;
		});
	}

	fn cancel_processing(&mut self) {
		*self.cancel_flag.lock().unwrap() = true;
		if let Some(pid) = *self.current_child.lock().unwrap() {
			#[cfg(unix)]
			{
				unsafe { libc::kill(pid as i32, libc::SIGTERM); }
			}
			#[cfg(windows)]
			{
				let _ = Command::new("taskkill")
					.args(["/PID", &pid.to_string(), "/F"])
					.output();
			}
		}
	}

	// ============= UI PANELS =============

	fn show_header(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			ui.heading(
				egui::RichText::new("LoMux")
					.strong()
					.color(self.theme.accent)
			);
			ui.label(
				egui::RichText::new(format!("v{}", VERSION))
					.small()
					.weak()
			);

			ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
				if ui.button("🎨").on_hover_text("Theme editor").clicked() {
					self.theme_editor_open = !self.theme_editor_open;
				}
				if ui.button("🔄").on_hover_text("Rescan tools").clicked() {
					self.tools.detect_all();
				}
			});
		});
	}

	fn show_files_panel(&mut self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.horizontal(|ui| {
				ui.strong("Files & Sources");
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					let count = self.media_files.len();
					if count > 0 {
						ui.label(
							egui::RichText::new(format!("{} file{}", count, if count == 1 { "" } else { "s" }))
								.small()
								.weak()
						);
					}
				});
			});
			ui.separator();

			ui.horizontal(|ui| {
				if ui.button("➕ Add Files").clicked() {
					self.select_input_files();
				}
				if ui.button("📂 Output").clicked() {
					self.select_output_dir();
				}
			});

			if self.tools.ytdlp_path.is_some() {
				ui.horizontal(|ui| {
					let response = ui.add(
						egui::TextEdit::singleline(&mut self.youtube_url_input)
							.desired_width(ui.available_width() - 130.0)
							.hint_text("YouTube / video URL")
					);

					egui::ComboBox::from_id_salt("yt_format")
						.width(60.0)
						.selected_text(match self.youtube_format {
							YtFormat::BestVideo => "Best",
							YtFormat::BestAudio => "Audio",
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

					let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
					if ui.button("➕").clicked() || enter_pressed {
						self.add_youtube_url();
					}
				});
			}

			if let Some(ref dir) = self.output_dir {
				ui.horizontal(|ui| {
					ui.label(egui::RichText::new("→").weak());
					ui.label(
						egui::RichText::new(truncate_path(dir, 50))
							.small()
					);
				});
			}

			if !self.media_files.is_empty() {
				ui.separator();

				egui::ScrollArea::vertical()
					.id_salt("files")
					.max_height(140.0)
					.show(ui, |ui| {
						ui.set_min_width(ui.available_width() - 10.0);

						let mut remove_idx = None;
						for (idx, file) in self.media_files.iter_mut().enumerate() {
							ui.horizontal(|ui| {
								let is_selected = self.selected_file_index == Some(idx);

								let icon = if file.is_youtube {
									match file.download_status {
										DownloadStatus::Downloading(_) => "⏳",
										DownloadStatus::Downloaded => "✅",
										DownloadStatus::Failed => "❌",
										_ => "📺",
									}
								} else {
									"📄"
								};

								let name = file.display_name();
								let label = format!("{} {}", icon, truncate_str(&name, 45));

								if ui.selectable_label(is_selected, label).clicked() {
									self.selected_file_index = Some(idx);
								}

								ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
									if ui.small_button("✕").on_hover_text("Remove").clicked() {
										remove_idx = Some(idx);
									}
									if ui.small_button("📝").on_hover_text("Edit metadata").clicked() {
										self.selected_file_index = Some(idx);
										self.metadata_mode = MetadataMode::PerFile;
									}
								});
							});
						}

						if let Some(idx) = remove_idx {
							self.media_files.remove(idx);
							if self.selected_file_index == Some(idx) {
								self.selected_file_index = None;
							} else if let Some(ref mut sel) = self.selected_file_index {
								if *sel > idx && *sel > 0 {
									*sel -= 1;
								}
							}
						}
					});

				ui.horizontal(|ui| {
					if ui.small_button("Clear All").clicked() {
						self.media_files.clear();
						self.selected_file_index = None;
					}
				});
			} else {
				ui.add_space(8.0);
				ui.colored_label(
					self.theme.text_secondary,
					"Drop files here or click Add Files"
				);
				ui.add_space(4.0);
			}
		});
	}

	fn show_presets_panel(&mut self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.strong("Encoding Preset");
			ui.separator();

			ui.horizontal_wrapped(|ui| {
				for category in PresetCategory::all() {
					if ui.selectable_label(
						self.preset_filter == *category,
						category.label(),
					).clicked() {
						self.preset_filter = category.clone();
					}
				}
			});

			ui.separator();

			if self.preset_filter == PresetCategory::Custom {
				self.show_custom_preset(ui);
			} else {
				egui::ScrollArea::vertical()
					.id_salt("presets")
					.max_height(120.0)
					.show(ui, |ui| {
						ui.set_min_width(ui.available_width() - 10.0);
						for preset in &self.presets {
							if preset.category == self.preset_filter {
								let is_selected = self.selected_preset.name == preset.name;
								ui.horizontal(|ui| {
									if ui.selectable_label(is_selected, &preset.name).clicked() {
										self.selected_preset = preset.clone();
									}
								});
								if is_selected && self.show_preset_info {
									ui.horizontal(|ui| {
										ui.add_space(12.0);
										ui.label(
											egui::RichText::new(format!(
												"{} — {}",
												preset.description,
												preset.info_line()
											))
												.small()
												.weak()
										);
									});
								}
							}
						}
					});
			}
		});
	}

	fn show_custom_preset(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			ui.label("Container:");
			egui::ComboBox::from_id_salt("custom_container")
				.selected_text(self.custom_preset.container.label())
				.show_ui(ui, |ui| {
					for c in Container::all() {
						ui.selectable_value(
							&mut self.custom_preset.container,
							c.clone(),
							c.label(),
						);
					}
				});

			ui.separator();

			ui.label("Video:");
			egui::ComboBox::from_id_salt("custom_vcodec")
				.selected_text(self.custom_preset.video_codec.label())
				.show_ui(ui, |ui| {
					for c in VideoCodec::all() {
						ui.selectable_value(
							&mut self.custom_preset.video_codec,
							c.clone(),
							c.label(),
						);
					}
				});
		});

		ui.horizontal(|ui| {
			ui.label("Audio:");
			egui::ComboBox::from_id_salt("custom_acodec")
				.selected_text(self.custom_preset.audio_codec.label())
				.show_ui(ui, |ui| {
					for c in AudioCodec::all() {
						ui.selectable_value(
							&mut self.custom_preset.audio_codec,
							c.clone(),
							c.label(),
						);
					}
				});

			if self.custom_preset.video_codec != VideoCodec::None
				&& self.custom_preset.video_codec != VideoCodec::Copy
			{
				ui.separator();
				ui.label("V.Bitrate:");
				let mut vb = self.custom_preset.video_bitrate.unwrap_or(5000);
				if ui.add(egui::DragValue::new(&mut vb).suffix("k").range(100..=100000)).changed() {
					self.custom_preset.video_bitrate = Some(vb);
				}
			}
		});

		ui.horizontal(|ui| {
			if self.custom_preset.video_codec != VideoCodec::None
				&& self.custom_preset.video_codec != VideoCodec::Copy
			{
				ui.label("CRF:");
				let mut use_crf = self.custom_preset.video_crf.is_some();
				if ui.checkbox(&mut use_crf, "").changed() {
					self.custom_preset.video_crf = if use_crf { Some(23) } else { None };
				}
				if let Some(ref mut crf) = self.custom_preset.video_crf {
					ui.add(egui::DragValue::new(crf).range(0..=51));
				}
				ui.separator();
			}

			if self.custom_preset.audio_codec != AudioCodec::None
				&& self.custom_preset.audio_codec != AudioCodec::Copy
				&& self.custom_preset.audio_codec != AudioCodec::Flac
				&& self.custom_preset.audio_codec != AudioCodec::Pcm
			{
				ui.label("A.Bitrate:");
				let mut ab = self.custom_preset.audio_bitrate.unwrap_or(192);
				if ui.add(egui::DragValue::new(&mut ab).suffix("k").range(32..=512)).changed() {
					self.custom_preset.audio_bitrate = Some(ab);
				}
			}
		});

		ui.horizontal(|ui| {
			ui.label("Resolution:");
			let mut res = self.custom_preset.resolution.clone().unwrap_or_default();
			if ui.add(
				egui::TextEdit::singleline(&mut res)
					.desired_width(100.0)
					.hint_text("e.g. 1920x1080")
			).changed() {
				self.custom_preset.resolution = if res.is_empty() { None } else { Some(res) };
			}

			ui.separator();
			ui.label("FPS:");
			let mut use_fps = self.custom_preset.fps.is_some();
			if ui.checkbox(&mut use_fps, "").changed() {
				self.custom_preset.fps = if use_fps { Some(30) } else { None };
			}
			if let Some(ref mut fps) = self.custom_preset.fps {
				ui.add(egui::DragValue::new(fps).range(1..=120));
			}
		});
	}

	fn show_metadata_panel(&mut self, ui: &mut egui::Ui) {
		ui.collapsing("Metadata Editor", |ui| {
			ui.horizontal(|ui| {
				ui.label("Mode:");
				ui.selectable_value(&mut self.metadata_mode, MetadataMode::None, "Off");
				ui.selectable_value(&mut self.metadata_mode, MetadataMode::Global, "All Files");
				ui.selectable_value(&mut self.metadata_mode, MetadataMode::Selected, "Checked");
				ui.selectable_value(&mut self.metadata_mode, MetadataMode::PerFile, "Per File");
			});

			if self.metadata_mode == MetadataMode::None {
				return;
			}

			ui.separator();

			if self.metadata_mode == MetadataMode::PerFile {
				if let Some(idx) = self.selected_file_index {
					if idx < self.media_files.len() {
						let filename = self.media_files[idx].display_name();
						ui.label(
							egui::RichText::new(format!("Editing: {}", truncate_str(&filename, 40)))
								.small()
								.weak()
						);
						Self::show_metadata_fields(ui, &mut self.media_files[idx].metadata, Some(&filename));

						ui.horizontal(|ui| {
							if ui.small_button("Clear").clicked() {
								self.media_files[idx].metadata.clear();
							}
							if ui.small_button("Copy to All").clicked() {
								let metadata = self.media_files[idx].metadata.clone();
								for file in &mut self.media_files {
									file.metadata = metadata.clone();
									file.apply_metadata = true;
								}
							}
						});
					} else {
						ui.label(
							egui::RichText::new("Select a file to edit its metadata")
								.weak()
						);
					}
				} else {
					ui.label(
						egui::RichText::new("Select a file to edit its metadata")
							.weak()
					);
				}
			} else {
				Self::show_metadata_fields(ui, &mut self.global_metadata, None);
				if ui.small_button("Clear").clicked() {
					self.global_metadata.clear();
				}
			}
		});
	}

	fn show_metadata_fields(ui: &mut egui::Ui, meta: &mut AudioMetadata, filename: Option<&str>) {
		let field_width = ui.available_width() - 60.0;

		ui.horizontal(|ui| {
			ui.label("Title:");
			let mut val = meta.title.clone().unwrap_or_default();
			if ui.add(egui::TextEdit::singleline(&mut val).desired_width(field_width - 30.0)).changed() {
				meta.title = if val.is_empty() { None } else { Some(val) };
			}
			if let Some(fname) = filename {
				if ui.small_button("📋").on_hover_text("Parse from filename").clicked() {
					*meta = AudioMetadata::from_filename(fname);
				}
			}
		});

		for (label, field) in [
			("Artist:", &mut meta.artist),
			("Album:", &mut meta.album),
		] {
			ui.horizontal(|ui| {
				ui.label(label);
				let mut val = field.clone().unwrap_or_default();
				if ui.add(egui::TextEdit::singleline(&mut val).desired_width(field_width)).changed() {
					*field = if val.is_empty() { None } else { Some(val) };
				}
			});
		}

		ui.horizontal(|ui| {
			for (label, field) in [
				("Year:", &mut meta.year),
				("Track:", &mut meta.track),
				("Genre:", &mut meta.genre),
			] {
				ui.label(label);
				let mut val = field.clone().unwrap_or_default();
				if ui.add(egui::TextEdit::singleline(&mut val).desired_width(60.0)).changed() {
					*field = if val.is_empty() { None } else { Some(val) };
				}
			}
		});
	}

	fn show_controls(&mut self, ui: &mut egui::Ui) {
		ui.separator();

		let is_processing = *self.is_processing.lock().unwrap();
		let progress_val = *self.progress.lock().unwrap();

		ui.horizontal(|ui| {
			if is_processing {
				if ui.button(
					egui::RichText::new("⏹ Cancel").color(egui::Color32::from_rgb(255, 120, 120))
				).clicked() {
					self.cancel_processing();
				}
			} else {
				ui.add_enabled_ui(self.can_process(), |ui| {
					if ui.button(
						egui::RichText::new("▶ Convert").strong().color(self.theme.accent)
					).clicked() {
						self.start_processing();
					}
				});
			}

			ui.add(
				egui::ProgressBar::new(progress_val / 100.0)
					.show_percentage()
					.animate(is_processing)
			);
		});

		ui.horizontal(|ui| {
			ui.label(
				egui::RichText::new(self.status.lock().unwrap().clone())
					.small()
			);

			ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
				if self.tools.ytdlp_path.is_some() {
					ui.label(egui::RichText::new("yt-dlp ✓").small().color(egui::Color32::from_rgb(100, 200, 100)));
				} else {
					ui.label(egui::RichText::new("yt-dlp ✗").small().color(egui::Color32::from_rgb(200, 150, 80)));
				}

				if self.tools.ffmpeg_path.is_some() {
					ui.label(egui::RichText::new("ffmpeg ✓").small().color(egui::Color32::from_rgb(100, 200, 100)));
				} else {
					ui.label(egui::RichText::new("ffmpeg ✗").small().color(egui::Color32::from_rgb(255, 100, 100)));
				}
			});
		});

		if self.tools.ffmpeg_path.is_none() {
			ui.colored_label(
				egui::Color32::from_rgb(255, 120, 120),
				"FFmpeg is required. Install it from ffmpeg.org"
			);
		}
	}

	fn show_console(&self, ui: &mut egui::Ui) {
		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.set_min_height(ui.available_height());

			ui.horizontal(|ui| {
				ui.strong("Console");
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					if ui.small_button("Clear").clicked() {
						self.console_output.lock().unwrap().clear();
					}
				});
			});
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
						ui.colored_label(
							self.theme.text_secondary,
							"Output will appear here..."
						);
					} else {
						ui.monospace(&*output);
					}
				});
		});
	}

	fn show_theme_editor(&mut self, ui: &mut egui::Ui) {
		let theme_before = self.theme.clone();

		ui.group(|ui| {
			ui.set_width(ui.available_width());
			ui.strong("Theme");
			ui.separator();

			ui.label(egui::RichText::new("Presets").small());
			egui::ScrollArea::horizontal()
				.id_salt("theme_presets")
				.show(ui, |ui| {
					ui.horizontal(|ui| {
						for preset in AppTheme::all_presets() {
							let is_active = self.theme.name == preset.name;
							let btn = egui::Button::new(
								egui::RichText::new(&preset.name)
									.small()
									.color(if is_active { preset.accent } else { self.theme.text_primary })
							);
							if ui.add(btn).clicked() {
								self.theme = preset;
							}
						}
					});
				});

			ui.separator();
			ui.label(egui::RichText::new("Customize").small());

			ui.horizontal(|ui| {
				ui.label("Accent:");
				let mut rgb = [
					self.theme.accent.r() as f32 / 255.0,
					self.theme.accent.g() as f32 / 255.0,
					self.theme.accent.b() as f32 / 255.0,
				];
				if ui.color_edit_button_rgb(&mut rgb).changed() {
					self.theme.accent = egui::Color32::from_rgb(
						(rgb[0] * 255.0) as u8,
						(rgb[1] * 255.0) as u8,
						(rgb[2] * 255.0) as u8,
					);
					self.theme.name = "Custom".into();
				}

				ui.separator();
				ui.label("Background:");
				let mut bg = [
					self.theme.bg_primary.r() as f32 / 255.0,
					self.theme.bg_primary.g() as f32 / 255.0,
					self.theme.bg_primary.b() as f32 / 255.0,
				];
				if ui.color_edit_button_rgb(&mut bg).changed() {
					self.theme.bg_primary = egui::Color32::from_rgb(
						(bg[0] * 255.0) as u8,
						(bg[1] * 255.0) as u8,
						(bg[2] * 255.0) as u8,
					);
					self.theme.bg_secondary = if self.theme.dark {
						lighten(self.theme.bg_primary, 10)
					} else {
						darken(self.theme.bg_primary, 10)
					};
					self.theme.name = "Custom".into();
				}
			});

			ui.horizontal(|ui| {
				ui.label("Rounding:");
				if ui.add(egui::Slider::new(&mut self.theme.rounding, 0.0..=14.0).suffix("px")).changed() {
					self.theme.name = "Custom".into();
				}

				ui.separator();
				let mut is_dark = self.theme.dark;
				if ui.checkbox(&mut is_dark, "Dark mode").changed() {
					self.theme.dark = is_dark;
					if is_dark {
						self.theme.text_primary = egui::Color32::from_rgb(220, 220, 230);
						self.theme.text_secondary = egui::Color32::from_rgb(130, 130, 150);
					} else {
						self.theme.text_primary = egui::Color32::from_rgb(30, 30, 40);
						self.theme.text_secondary = egui::Color32::from_rgb(110, 110, 130);
					}
					self.theme.name = "Custom".into();
				}
			});
		});

		if self.theme != theme_before {
			save_theme(&self.theme);
		}
	}
}

// ============= HELPER FUNCTIONS =============

fn build_ffmpeg_args(
	preset: &EncodingPreset,
	input: &Path,
	output: &Path,
	metadata: &AudioMetadata,
	extra_args: &str,
) -> Vec<String> {
	let mut args = vec![
		"-hide_banner".into(),
		"-loglevel".into(), "warning".into(),
		"-stats".into(),
		"-progress".into(), "pipe:2".into(),
		"-i".into(),
		input.to_string_lossy().to_string(),
	];

	match preset.video_codec {
		VideoCodec::H264 => {
			args.extend(["-c:v".into(), "libx264".into()]);
			args.extend(["-preset".into(), "medium".into()]);
			if let Some(crf) = preset.video_crf {
				args.extend(["-crf".into(), crf.to_string()]);
			} else if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".into(), format!("{}k", bitrate)]);
			}
			args.extend(["-pix_fmt".into(), "yuv420p".into()]);
			args.extend(["-movflags".into(), "+faststart".into()]);
		}
		VideoCodec::H265 => {
			args.extend(["-c:v".into(), "libx265".into()]);
			args.extend(["-preset".into(), "medium".into()]);
			if let Some(crf) = preset.video_crf {
				args.extend(["-crf".into(), crf.to_string()]);
			} else if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".into(), format!("{}k", bitrate)]);
			}
			args.extend(["-tag:v".into(), "hvc1".into()]);
			args.extend(["-pix_fmt".into(), "yuv420p".into()]);
			args.extend(["-movflags".into(), "+faststart".into()]);
		}
		VideoCodec::Vp8 => {
			args.extend(["-c:v".into(), "libvpx".into()]);
			if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".into(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::Vp9 => {
			args.extend(["-c:v".into(), "libvpx-vp9".into()]);
			if let Some(crf) = preset.video_crf {
				args.extend(["-crf".into(), crf.to_string()]);
				args.extend(["-b:v".into(), "0".into()]);
			} else if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".into(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::Av1 => {
			args.extend(["-c:v".into(), "libaom-av1".into()]);
			args.extend(["-cpu-used".into(), "4".into()]);
			if let Some(crf) = preset.video_crf {
				args.extend(["-crf".into(), crf.to_string()]);
			}
		}
		VideoCodec::ProRes => {
			args.extend(["-c:v".into(), "prores_ks".into()]);
			args.extend(["-profile:v".into(), "3".into()]);
			args.extend(["-vendor".into(), "apl0".into()]);
		}
		VideoCodec::DnxHd => {
			args.extend(["-c:v".into(), "dnxhd".into()]);
			if let Some(bitrate) = preset.video_bitrate {
				args.extend(["-b:v".into(), format!("{}k", bitrate)]);
			}
		}
		VideoCodec::Copy => {
			args.extend(["-c:v".into(), "copy".into()]);
		}
		VideoCodec::None => {
			args.push("-vn".into());
		}
	}

	match preset.audio_codec {
		AudioCodec::Aac => {
			args.extend(["-c:a".into(), "aac".into()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".into(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Mp3 => {
			args.extend(["-c:a".into(), "libmp3lame".into()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".into(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Opus => {
			args.extend(["-c:a".into(), "libopus".into()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".into(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Flac => {
			args.extend(["-c:a".into(), "flac".into()]);
		}
		AudioCodec::Vorbis => {
			args.extend(["-c:a".into(), "libvorbis".into()]);
			if let Some(bitrate) = preset.audio_bitrate {
				args.extend(["-b:a".into(), format!("{}k", bitrate)]);
			}
		}
		AudioCodec::Pcm => {
			args.extend(["-c:a".into(), "pcm_s16le".into()]);
		}
		AudioCodec::Copy => {
			args.extend(["-c:a".into(), "copy".into()]);
		}
		AudioCodec::None => {
			args.push("-an".into());
		}
	}

	if preset.video_codec != VideoCodec::Copy && preset.video_codec != VideoCodec::None {
		if let Some(ref resolution) = preset.resolution {
			if resolution.contains("x") || resolution.contains(":") {
				args.extend(["-vf".into(), format!("scale={}", resolution.replace("x", ":"))]);
			}
		}
		if let Some(fps) = preset.fps {
			args.extend(["-r".into(), fps.to_string()]);
		}
	}

	args.push("-threads".into());
	args.push("0".into());

	if !metadata.is_empty() {
		for (key, val) in [
			("title", &metadata.title),
			("artist", &metadata.artist),
			("album", &metadata.album),
			("date", &metadata.year),
			("genre", &metadata.genre),
			("track", &metadata.track),
			("comment", &metadata.comment),
		] {
			if let Some(ref v) = val {
				args.extend(["-metadata".into(), format!("{}={}", key, v)]);
			}
		}
	}

	if !extra_args.is_empty() {
		args.extend(
			extra_args.split_whitespace()
				.map(|s| s.to_string())
		);
	}

	args.extend(["-y".into(), output.to_string_lossy().to_string()]);
	args
}

fn get_duration(ffprobe: &Path, input: &Path) -> Result<f64, Box<dyn std::error::Error>> {
	let output = Command::new(ffprobe)
		.args([
			"-v", "error",
			"-show_entries", "format=duration",
			"-of", "csv=p=0",
		])
		.arg(input)
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
	cancel_flag: &Arc<Mutex<bool>>,
) -> Option<PathBuf> {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();

	let output_template = temp_dir.join(format!("yt_{}_%(title).80s.%(ext)s", timestamp))
		.to_string_lossy()
		.to_string();

	let mut args = vec![
		"-o".to_string(), output_template,
		"--progress".to_string(),
		"--newline".to_string(),
		"--print".to_string(), "after_move:filepath".to_string(),
		"--no-playlist".to_string(),
	];

	match format {
		YtFormat::BestVideo => {
			args.extend(["-f".into(), "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".into()]);
		}
		YtFormat::Mp4_1080p => {
			args.extend(["-f".into(), "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/best[height<=1080]".into()]);
		}
		YtFormat::Mp4_720p => {
			args.extend(["-f".into(), "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/best[height<=720]".into()]);
		}
		YtFormat::BestAudio => {
			args.extend(["-x".into(), "-f".into(), "bestaudio".into()]);
		}
		YtFormat::Mp3 => {
			args.extend(["-x".into(), "--audio-format".into(), "mp3".into()]);
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
	let mut downloaded_file: Option<PathBuf> = None;
	let temp_dir_str = temp_dir.to_string_lossy().to_string();

	for line in reader.lines().flatten() {
		if *cancel_flag.lock().unwrap() {
			kill_process(&child);
			return None;
		}

		if line.starts_with(&temp_dir_str) && !line.contains("[") {
			let path = PathBuf::from(line.trim());
			if path.exists() {
				downloaded_file = Some(path);
			}
		} else if line.contains("[download]") && line.contains("%") {
			// progress line — could update progress bar here
		} else if !line.trim().is_empty() {
			console.lock().unwrap().push_str(&line);
			console.lock().unwrap().push('\n');
		}
	}

	let status = child.wait().ok()?;
	if status.success() {
		if downloaded_file.is_none() {
			if let Ok(entries) = std::fs::read_dir(temp_dir) {
				let prefix = format!("yt_{}_", timestamp);
				for entry in entries.flatten() {
					let name = entry.file_name().to_string_lossy().to_string();
					if name.starts_with(&prefix) {
						downloaded_file = Some(entry.path());
						break;
					}
				}
			}
		}
		if let Some(ref path) = downloaded_file {
			console.lock().unwrap().push_str(&format!("✅ Downloaded: {}\n", path.display()));
		}
		downloaded_file
	} else {
		console.lock().unwrap().push_str("❌ Download failed\n");
		None
	}
}

fn kill_process(child: &Child) {
	let id = child.id();
	#[cfg(unix)]
	unsafe { libc::kill(id as i32, libc::SIGTERM); }
	#[cfg(windows)]
	{
		let _ = Command::new("taskkill")
			.args(["/PID", &id.to_string(), "/F"])
			.output();
	}
	let _ = id;
}

fn format_file_size(bytes: u64) -> String {
	if bytes < 1024 {
		format!("{} B", bytes)
	} else if bytes < 1024 * 1024 {
		format!("{:.1} KB", bytes as f64 / 1024.0)
	} else if bytes < 1024 * 1024 * 1024 {
		format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
	} else {
		format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
	}
}

fn truncate_str(s: &str, max: usize) -> String {
	if s.len() <= max {
		s.to_string()
	} else {
		format!("{}…", &s[..max.saturating_sub(1)])
	}
}

fn truncate_path(path: &Path, max: usize) -> String {
	let s = path.to_string_lossy();
	if s.len() <= max {
		s.to_string()
	} else {
		let filename = path.file_name().unwrap_or_default().to_string_lossy();
		if filename.len() + 4 > max {
			truncate_str(&s, max)
		} else {
			format!("…/{}", filename)
		}
	}
}

// ============= APP IMPLEMENTATION =============

impl eframe::App for LoMuxApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		self.theme.apply(ctx);

		// Handle drag and drop
		ctx.input(|i| {
			for file in &i.raw.dropped_files {
				if let Some(ref path) = file.path {
					self.media_files.push(MediaFile::new(path.clone()));
				}
			}
		});

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);
			ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);

			egui::Frame::none()
				.inner_margin(egui::Margin::symmetric(12.0, 6.0))
				.show(ui, |ui| {
					self.show_header(ui);
				});

			ui.separator();

			if self.theme_editor_open {
				egui::Frame::none()
					.inner_margin(egui::Margin::symmetric(12.0, 4.0))
					.show(ui, |ui| {
						self.show_theme_editor(ui);
					});
				ui.separator();
			}

			let available_height = ui.available_height();

			egui::Frame::none()
				.inner_margin(egui::Margin::symmetric(12.0, 4.0))
				.show(ui, |ui| {
					ui.horizontal(|ui| {
						ui.set_height(available_height - 16.0);

						ui.vertical(|ui| {
							ui.set_min_width(380.0);
							ui.set_max_width(480.0);
							ui.set_height(available_height - 16.0);

							egui::ScrollArea::vertical()
								.id_salt("controls")
								.max_height(available_height - 16.0)
								.show(ui, |ui| {
									self.show_files_panel(ui);
									ui.add_space(4.0);
									self.show_presets_panel(ui);
									ui.add_space(4.0);
									self.show_metadata_panel(ui);
									ui.add_space(4.0);

									ui.collapsing("Advanced", |ui| {
										ui.label(
											egui::RichText::new("Extra FFmpeg arguments:")
												.small()
										);
										ui.add(
											egui::TextEdit::singleline(&mut self.extra_args)
												.hint_text("-preset slow -tune film")
												.desired_width(ui.available_width())
										);
									});

									ui.add_space(4.0);
									self.show_controls(ui);
								});
						});

						ui.separator();

						ui.vertical(|ui| {
							ui.set_min_height(available_height - 16.0);
							self.show_console(ui);
						});
					});
				});

			if *self.is_processing.lock().unwrap() {
				ctx.request_repaint_after(std::time::Duration::from_millis(50));
			} else {
				ctx.request_repaint_after(std::time::Duration::from_millis(250));
			}
		});
	}
}

#[cfg(not(target_arch = "wasm32"))]
fn load_icon() -> Option<egui::IconData> {
	use image::GenericImageView;
	let png_data = include_bytes!("../assets/LoMux.png");
	let img = image::load_from_memory(png_data).ok()?;
	let rgba = img.to_rgba8();
	let (width, height) = img.dimensions();
	Some(egui::IconData {
		rgba: rgba.into_raw(),
		width,
		height,
	})
}

fn main() -> eframe::Result {
	let mut viewport = egui::ViewportBuilder::default()
		.with_inner_size([1000.0, 700.0])
		.with_min_inner_size([800.0, 500.0])
		.with_drag_and_drop(true);

	#[cfg(not(target_arch = "wasm32"))]
	if let Some(icon) = load_icon() {
		viewport = viewport.with_icon(Arc::new(icon));
	}

	let options = eframe::NativeOptions {
		viewport,
		..Default::default()
	};

	eframe::run_native(
		&format!("LoMux v{}", VERSION),
		options,
		Box::new(|_cc| Ok(Box::new(LoMuxApp::new()))),
	)
}
