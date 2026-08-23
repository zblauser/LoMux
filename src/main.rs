#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{BufRead, BufReader};

const VERSION: &str = "1.2.0";

const DOWNLOAD_PHASE_SHARE: f32 = 0.5;

const LOUDNORM_FILTER: &str = "loudnorm=I=-16:TP=-1.5:LRA=11";

const CUSTOM_THEME: &str = "Custom";

const FFMPEG_PROGRESS_KEYS: &[&str] = &[
	"progress=", "stream_", "bitrate=", "total_size=",
	"out_time=", "dup_frames=", "drop_frames=",
];

// ============= PRESET SYSTEM =============

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
	Png,
	Jpg,
	Tiff,
	Aiff,
	Ac3,
	Mxf,
	Bmp,
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
			Self::Png => "png",
			Self::Jpg => "jpg",
			Self::Tiff => "tiff",
			Self::Aiff => "aiff",
			Self::Ac3 => "ac3",
			Self::Mxf => "mxf",
			Self::Bmp => "bmp",
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
			Self::Png => "PNG Sequence",
			Self::Jpg => "JPEG Sequence",
			Self::Tiff => "TIFF Sequence",
			Self::Aiff => "AIFF",
			Self::Ac3 => "AC-3",
			Self::Mxf => "MXF OP1a",
			Self::Bmp => "BMP",
		}
	}

	fn is_image_sequence(&self) -> bool {
		matches!(self, Self::Png | Self::Jpg | Self::Tiff | Self::Bmp)
	}

	fn image_encoder(&self) -> Option<&'static str> {
		match self {
			Self::Png => Some("png"),
			Self::Jpg => Some("mjpeg"),
			Self::Tiff => Some("tiff"),
			Self::Bmp => Some("bmp"),
			_ => None,
		}
	}

	fn all() -> &'static [Self] {
		&[
			Self::Mp4, Self::Mkv, Self::Webm, Self::Mov, Self::Avi, Self::Mxf,
			Self::Mp3, Self::Flac, Self::Aac, Self::Opus, Self::Wav, Self::Aiff, Self::Ac3, Self::Gif,
			Self::Png, Self::Jpg, Self::Tiff, Self::Bmp,
		]
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum AudioCodec {
	Aac,
	Mp3,
	Opus,
	Flac,
	Vorbis,
	Pcm,
	Ac3,
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
			Self::Ac3 => "Dolby Digital (AC-3)",
			Self::Copy => "Copy",
			Self::None => "None",
		}
	}

	fn all() -> &'static [Self] {
		&[
			Self::Aac, Self::Mp3, Self::Opus, Self::Flac,
			Self::Vorbis, Self::Pcm, Self::Ac3, Self::Copy, Self::None,
		]
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum PresetCategory {
	WebSocial,
	Device,
	Professional,
	Audio,
	ImageSequence,
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
			Self::ImageSequence => "Images",
			Self::MatchSource => "Match Source",
			Self::Custom => "Custom",
		}
	}

	fn all() -> &'static [Self] {
		&[
			Self::WebSocial, Self::Device, Self::Professional,
			Self::Audio, Self::ImageSequence, Self::MatchSource, Self::Custom,
		]
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
	#[serde(default)]
	single_image: bool,
	#[serde(default)]
	codec_profile: Option<String>,
	#[serde(default)]
	audio_channels: Option<u8>,
	#[serde(default)]
	audio_sample_rate: Option<u32>,
	#[serde(default)]
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
			single_image: false,
			codec_profile: None,
			audio_channels: None,
			audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Animated GIF for web use".into(),
			},

			Self {
				name: "Vimeo 1080p".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(10000),
				audio_bitrate: Some(320),
				video_crf: None,
				fps: None,
				resolution: Some("1920x1080".into()),
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Vimeo's recommended 1080p bitrate".into(),
			},
			Self {
				name: "Vimeo 4K UHD".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H265,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(45000),
				audio_bitrate: Some(320),
				video_crf: None,
				fps: None,
				resolution: Some("3840x2160".into()),
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Vimeo 4K at their recommended ceiling".into(),
			},
			Self {
				name: "Twitch VOD 1080p60".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(6000),
				audio_bitrate: Some(160),
				video_crf: None,
				fps: Some(60),
				resolution: Some("1920x1080".into()),
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Stream archive at Twitch's 6 Mbps cap".into(),
			},
			Self {
				name: "YouTube Shorts".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(8000),
				audio_bitrate: Some(192),
				video_crf: None,
				fps: Some(30),
				resolution: Some("1080x1920".into()),
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Vertical short-form for YouTube".into(),
			},
			Self {
				name: "Facebook / LinkedIn".into(),
				category: PresetCategory::WebSocial,
				container: Container::Mp4,
				video_codec: VideoCodec::H264,
				audio_codec: AudioCodec::Aac,
				video_bitrate: Some(4000),
				audio_bitrate: Some(128),
				video_crf: None,
				fps: Some(30),
				resolution: Some("1920x1080".into()),
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Landscape video for feed platforms".into(),
			},
			Self {
				name: "WebM VP9 1080p".into(),
				category: PresetCategory::WebSocial,
				container: Container::Webm,
				video_codec: VideoCodec::Vp9,
				audio_codec: AudioCodec::Opus,
				video_bitrate: None,
				audio_bitrate: Some(128),
				video_crf: Some(31),
				fps: None,
				resolution: Some("1920x1080".into()),
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Royalty-free VP9 for the open web".into(),
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Chromecast-compatible H.264".into(),
			},

			// Professional
			Self {
				name: "ProRes 422 Proxy".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::ProRes,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("0".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "Smallest ProRes, for offline editing".into(),
			},
			Self {
				name: "ProRes 422 LT".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::ProRes,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("1".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "Light ProRes for delivery and review".into(),
			},
			Self {
				name: "ProRes 422 HQ".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::ProRes,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("3".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "High quality ProRes for finishing".into(),
			},
			Self {
				name: "ProRes 4444".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::ProRes,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("4".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "ProRes 4444 with 12-bit colour depth".into(),
			},
			Self {
				name: "DNxHR LB".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::DnxHd,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("dnxhr_lb".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "Low bandwidth DNxHR proxy, any resolution".into(),
			},
			Self {
				name: "DNxHR SQ".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::DnxHd,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("dnxhr_sq".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "Standard quality DNxHR, any resolution".into(),
			},
			Self {
				name: "DNxHR HQ".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::DnxHd,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("dnxhr_hq".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "High quality DNxHR, any resolution".into(),
			},
			Self {
				name: "DNxHR HQX".into(),
				category: PresetCategory::Professional,
				container: Container::Mov,
				video_codec: VideoCodec::DnxHd,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("dnxhr_hqx".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "10-bit DNxHR HQX for HDR and finishing".into(),
			},
			Self {
				name: "DNxHR HQ (MXF)".into(),
				category: PresetCategory::Professional,
				container: Container::Mxf,
				video_codec: VideoCodec::DnxHd,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
				codec_profile: Some("dnxhr_hq".into()),
				audio_channels: None,
				audio_sample_rate: None,
				description: "DNxHR HQ wrapped as MXF OP1a for broadcast".into(),
			},
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
				single_image: false,
				codec_profile: Some("2".into()),
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Uncompressed PCM audio".into(),
			},

			Self {
				name: "Podcast Mono 128k".into(),
				category: PresetCategory::Audio,
				container: Container::Mp3,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Mp3,
				video_bitrate: None,
				audio_bitrate: Some(128),
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: Some(1),
				audio_sample_rate: Some(44100),
				description: "Spoken word in mono — pair with loudness normalization".into(),
			},
			Self {
				name: "Audiobook AAC 64k".into(),
				category: PresetCategory::Audio,
				container: Container::Aac,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Aac,
				video_bitrate: None,
				audio_bitrate: Some(64),
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: Some(1),
				audio_sample_rate: Some(44100),
				description: "Small mono AAC for long-form speech".into(),
			},
			Self {
				name: "AIFF Uncompressed".into(),
				category: PresetCategory::Audio,
				container: Container::Aiff,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Pcm,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Uncompressed PCM for Logic, Pro Tools, and Final Cut".into(),
			},
			Self {
				name: "Dolby Digital 448k".into(),
				category: PresetCategory::Audio,
				container: Container::Ac3,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::Ac3,
				video_bitrate: None,
				audio_bitrate: Some(448),
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "AC-3 for DVD, Blu-ray, and broadcast delivery".into(),
			},

			// Single images
			Self {
				name: "PNG Image".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Png,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: true,
				codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Single lossless PNG — converts a photo, or grabs one frame from a video".into(),
			},
			Self {
				name: "JPEG Image".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Jpg,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: true,
				codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Single JPEG at high quality".into(),
			},
			Self {
				name: "TIFF Image".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Tiff,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: true,
				codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Single uncompressed TIFF for print and finishing".into(),
			},
			Self {
				name: "BMP Image".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Bmp,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: true,
				codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Single uncompressed bitmap".into(),
			},
			Self {
				name: "JPEG Image 1080p".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Jpg,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: Some("1920x-1".into()),
				single_image: true,
				codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Single JPEG scaled to fit 1920x1080".into(),
			},
			Self {
				name: "Web Thumbnail 640px".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Jpg,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: Some("640x-1".into()),
				single_image: true,
				codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Small JPEG for web thumbnails and contact sheets".into(),
			},

			// Image Sequence
			Self {
				name: "PNG Sequence".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Png,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Lossless PNG frames, one file per frame".into(),
			},
			Self {
				name: "JPEG Sequence".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Jpg,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Compressed JPEG frames for review and contact sheets".into(),
			},
			Self {
				name: "TIFF Sequence".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Tiff,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: None,
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Uncompressed TIFF frames for finishing workflows".into(),
			},
			Self {
				name: "PNG Sequence 24fps".into(),
				category: PresetCategory::ImageSequence,
				container: Container::Png,
				video_codec: VideoCodec::None,
				audio_codec: AudioCodec::None,
				video_bitrate: None,
				audio_bitrate: None,
				video_crf: None,
				fps: Some(24),
				resolution: None,
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "PNG frames resampled to 24fps".into(),
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
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
				single_image: false,
			codec_profile: None,
				audio_channels: None,
				audio_sample_rate: None,
				description: "Copy streams, change container only".into(),
			},
		]
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
enum SubtitleMode {
	#[default]
	None,
	Burn,
	Soft,
}

impl SubtitleMode {
	fn label(&self) -> &str {
		match self {
			Self::None => "Off",
			Self::Burn => "Burn in",
			Self::Soft => "Soft track",
		}
	}
}

#[derive(Debug, Clone, Default)]
struct EncodeOptions {
	extra_args: String,
	normalize_audio: bool,
	trim_start: Option<String>,
	trim_end: Option<String>,
	pass: Option<u8>,
	pass_log: Option<String>,
	subtitle_path: Option<PathBuf>,
	subtitle_mode: SubtitleMode,
}

fn supports_two_pass(preset: &EncodingPreset) -> bool {
	preset.video_crf.is_none()
		&& preset.video_bitrate.is_some()
		&& matches!(
			preset.video_codec,
			VideoCodec::H264 | VideoCodec::H265 | VideoCodec::Vp8 | VideoCodec::Vp9
		)
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
	trim_start: String,
	trim_end: String,
	subtitle_path: Option<PathBuf>,
	subtitle_mode: SubtitleMode,
}

#[derive(Debug, Clone, PartialEq)]
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
			trim_start: String::new(),
			trim_end: String::new(),
			subtitle_path: None,
			subtitle_mode: SubtitleMode::None,
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
			trim_start: String::new(),
			trim_end: String::new(),
			subtitle_path: None,
			subtitle_mode: SubtitleMode::None,
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
	supports_burn_in: bool,
}

impl ToolDetector {
	fn new() -> Self {
		let mut detector = Self {
			ffmpeg_path: None,
			ffprobe_path: None,
			ytdlp_path: None,
			supports_burn_in: false,
		};
		detector.detect_all();
		detector
	}

	fn detect_all(&mut self) {
		self.ffmpeg_path = Self::find_tool("ffmpeg");
		self.ffprobe_path = Self::find_tool("ffprobe");
		self.ytdlp_path = Self::find_tool("yt-dlp")
			.or_else(|| Self::find_tool("youtube-dl"));
		self.supports_burn_in = self.ffmpeg_path
			.as_ref()
			.map(|p| Self::has_subtitles_filter(p))
			.unwrap_or(false);
	}

	fn has_subtitles_filter(ffmpeg: &Path) -> bool {
		Command::new(ffmpeg)
			.args(["-hide_banner", "-filters"])
			.output()
			.map(|out| {
				let listing = String::from_utf8_lossy(&out.stdout);
				listing.lines().any(|line| {
					line.split_whitespace().nth(1) == Some("subtitles")
				})
			})
			.unwrap_or(false)
	}

	fn find_tool(tool_name: &str) -> Option<PathBuf> {
		if let Ok(path) = which::which(tool_name) {
			return Some(path);
		}

		for path in Self::get_search_paths(tool_name) {
			if path.exists() {
				return Some(path);
			}
		}

		#[cfg(target_os = "macos")]
		{
			if let Some(prefix) = Self::brew_prefix() {
				let brew_path = PathBuf::from(format!("{}/bin/{}", prefix, tool_name));
				if brew_path.exists() {
					return Some(brew_path);
				}
			}
		}

		None
	}

	#[cfg(target_os = "macos")]
	fn brew_prefix() -> Option<&'static str> {
		static PREFIX: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
		PREFIX.get_or_init(|| {
			let output = Command::new("brew").args(["--prefix"]).output().ok()?;
			let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
			if prefix.is_empty() { None } else { Some(prefix) }
		}).as_deref()
	}

	fn get_search_paths(tool_name: &str) -> Vec<PathBuf> {
		#[cfg(target_os = "windows")]
		{
			let package = match tool_name {
				"ffmpeg" | "ffprobe" => "ffmpeg\\bin",
				other => other,
			};
			return ["C:\\", "C:\\Program Files\\", "C:\\Program Files (x86)\\"]
				.iter()
				.map(|root| PathBuf::from(format!("{}{}\\{}.exe", root, package, tool_name)))
				.collect();
		}
		#[cfg(not(target_os = "windows"))]
		{
			return Self::search_dirs()
				.iter()
				.map(|dir| PathBuf::from(format!("{}/{}", dir, tool_name)))
				.collect();
		}
	}

	#[cfg(not(target_os = "windows"))]
	fn search_dirs() -> &'static [&'static str] {
		#[cfg(target_os = "macos")]
		return &["/usr/local/bin", "/opt/homebrew/bin", "/opt/local/bin", "/usr/bin"];
		#[cfg(target_os = "linux")]
		return &["/usr/bin", "/usr/local/bin", "/snap/bin"];
		#[cfg(not(any(target_os = "macos", target_os = "linux")))]
		return &["/usr/local/bin", "/usr/bin"];
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
	download_states: Arc<Mutex<Vec<DownloadStatus>>>,

	tools: ToolDetector,

	theme: AppTheme,
	custom_theme: AppTheme,
	theme_editor_open: bool,
	about_open: bool,
	extra_args: String,
	normalize_audio: bool,
	two_pass: bool,
	filename_template: String,
	imported_presets: Vec<EncodingPreset>,
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

fn presets_path() -> Option<PathBuf> {
	config_dir().map(|d| d.join("presets.json"))
}

fn parse_presets_json(text: &str) -> Option<Vec<EncodingPreset>> {
	if let Ok(list) = serde_json::from_str::<Vec<EncodingPreset>>(text) {
		return Some(list);
	}
	serde_json::from_str::<EncodingPreset>(text).ok().map(|p| vec![p])
}

fn save_imported_presets(presets: &[EncodingPreset]) {
	if let Some(path) = presets_path() {
		if let Some(dir) = path.parent() {
			let _ = std::fs::create_dir_all(dir);
		}
		if let Ok(json) = serde_json::to_string_pretty(presets) {
			let _ = std::fs::write(path, json);
		}
	}
}

fn load_imported_presets() -> Vec<EncodingPreset> {
	presets_path()
		.and_then(|path| std::fs::read_to_string(path).ok())
		.and_then(|text| parse_presets_json(&text))
		.unwrap_or_default()
}

#[derive(Serialize, Deserialize)]
struct AppConfig {
	#[serde(flatten)]
	active: ThemeConfig,
	#[serde(default)]
	custom: Option<ThemeConfig>,
}

fn save_theme(theme: &AppTheme, custom: &AppTheme) {
	if let Some(path) = config_path() {
		if let Some(dir) = path.parent() {
			let _ = std::fs::create_dir_all(dir);
		}
		let config = AppConfig {
			active: ThemeConfig::from(theme),
			custom: Some(ThemeConfig::from(custom)),
		};
		if let Ok(json) = serde_json::to_string_pretty(&config) {
			let _ = std::fs::write(path, json);
		}
	}
}

fn load_theme() -> Option<(AppTheme, Option<AppTheme>)> {
	let path = config_path()?;
	let data = std::fs::read_to_string(path).ok()?;
	let config: AppConfig = serde_json::from_str(&data).ok()?;
	let custom = config.custom.map(AppTheme::from);
	Some((AppTheme::from(config.active), custom))
}

fn default_custom_theme() -> AppTheme {
	let mut theme = AppTheme::studio_dark();
	theme.name = CUSTOM_THEME.into();
	theme
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
		visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, self.text_secondary);
		visuals.widgets.noninteractive.rounding = rounding;
		visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
			0.5_f32,
			if self.dark { lighten(self.bg_secondary, 20) } else { darken(self.bg_secondary, 15) }
		);

		visuals.widgets.inactive.bg_fill = if self.dark {
			lighten(self.bg_secondary, 12)
		} else {
			darken(self.bg_secondary, 8)
		};
		visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, self.text_primary);
		visuals.widgets.inactive.rounding = small_rounding;

		visuals.widgets.hovered.bg_fill = alpha_blend(self.accent, 50);
		visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, self.text_primary);
		visuals.widgets.hovered.rounding = small_rounding;

		visuals.widgets.active.bg_fill = alpha_blend(self.accent, 80);
		visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5_f32, self.text_primary);
		visuals.widgets.active.rounding = small_rounding;

		visuals.widgets.open.bg_fill = if self.dark {
			lighten(self.bg_secondary, 16)
		} else {
			darken(self.bg_secondary, 12)
		};
		visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, self.accent);
		visuals.widgets.open.rounding = small_rounding;

		visuals.selection.bg_fill = alpha_blend(self.accent, 60);
		visuals.selection.stroke = egui::Stroke::new(1.0_f32, self.accent);

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
#[allow(dead_code)]
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
		purge_temp_downloads(&temp_dir);

		let (loaded_theme, loaded_custom) = match load_theme() {
			Some((theme, custom)) => (theme, custom.unwrap_or_else(default_custom_theme)),
			None => (AppTheme::studio_dark(), default_custom_theme()),
		};

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
			download_states: Arc::new(Mutex::new(Vec::new())),
			tools: ToolDetector::new(),
			theme: loaded_theme,
			custom_theme: loaded_custom,
			theme_editor_open: false,
			about_open: false,
			extra_args: String::new(),
			normalize_audio: false,
			two_pass: false,
			filename_template: String::new(),
			imported_presets: load_imported_presets(),
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
				"png", "jpg", "jpeg", "tif", "tiff", "bmp", "webp",
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

	fn generate_output_dir(output_dir: &Path, stem: &str) -> PathBuf {
		let candidate = output_dir.join(stem);
		if !candidate.exists() {
			return candidate;
		}
		for i in 1..1000 {
			let numbered = output_dir.join(format!("{}_{}", stem, i));
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
		*self.download_states.lock().unwrap() = vec![DownloadStatus::NotStarted; self.media_files.len()];

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
		let download_states = self.download_states.clone();
		let temp_dir = self.temp_download_dir.clone();
		let metadata_mode = self.metadata_mode.clone();
		let global_metadata = self.global_metadata.clone();
		let youtube_format = self.youtube_format.clone();
		let extra_args = self.extra_args.clone();
		let normalize_audio = self.normalize_audio;
		let two_pass = self.two_pass;
		let supports_burn_in = self.tools.supports_burn_in;
		let filename_template = self.filename_template.clone();

		thread::spawn(move || {
			let total = files.len();
			let mut processed = 0;
			let mut succeeded = 0;
			let mut failed = 0;
			let mut downloaded_temp_files: Vec<PathBuf> = Vec::new();

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
							set_download_state(&download_states, idx, DownloadStatus::Downloading(0.0));

							let downloaded_path = download_youtube_video(
								ytdlp_path,
								url,
								&temp_dir,
								&youtube_format,
								&console,
								&|fraction| {
									set_download_state(&download_states, idx, DownloadStatus::Downloading(fraction));
									*progress.lock().unwrap() =
										(processed as f32 + fraction * DOWNLOAD_PHASE_SHARE) / total as f32 * 100.0;
									*status.lock().unwrap() =
										format!("Downloading {}/{}: {:.0}%", current, total, fraction * 100.0);
								},
								&cancel_flag,
							);

							if let Some(path) = downloaded_path {
								downloaded_temp_files.push(path.clone());
								file.path = path;
								file.download_status = DownloadStatus::Downloaded;
								set_download_state(&download_states, idx, DownloadStatus::Downloaded);
							} else {
								file.download_status = DownloadStatus::Failed;
								set_download_state(&download_states, idx, DownloadStatus::Failed);
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

				let metadata = match metadata_mode {
					MetadataMode::Global => &global_metadata,
					MetadataMode::PerFile if file.apply_metadata => &file.metadata,
					MetadataMode::Selected => {
						if file.apply_metadata { &file.metadata } else { &global_metadata }
					}
					_ => &AudioMetadata::default(),
				};

				let raw_stem = file.path.file_stem().unwrap_or_default().to_string_lossy().to_string();
				let stem = if file.is_youtube { strip_youtube_prefix(&raw_stem) } else { raw_stem };
				let extension = preset.container.extension();
				let stem = if filename_template.trim().is_empty() {
					stem
				} else {
					apply_filename_template(
						&filename_template,
						&stem,
						metadata,
						&preset.name,
						extension,
						idx,
					)
				};
				let is_sequence = preset.container.is_image_sequence() && !preset.single_image;
				let (output, artifact) = if is_sequence {
					let dir = Self::generate_output_dir(&output_dir, &stem);
					if std::fs::create_dir_all(&dir).is_err() {
						console.lock().unwrap().push_str(&format!("❌ Could not create {}\n", dir.display()));
						failed += 1;
						continue;
					}
					let pattern = dir.join(format!("{}_%05d.{}", stem, extension));
					(pattern, dir)
				} else {
					let path = Self::generate_output_path(&output_dir, &stem, extension);
					(path.clone(), path)
				};

				let duration = ffprobe.as_ref()
					.and_then(|p| get_duration(p, &file.path).ok())
					.unwrap_or(0.0);

				let mut options = EncodeOptions {
					extra_args: extra_args.clone(),
					normalize_audio,
					trim_start: some_if_set(&file.trim_start),
					trim_end: some_if_set(&file.trim_end),
					pass: None,
					pass_log: None,
					subtitle_path: file.subtitle_path.clone(),
					subtitle_mode: if file.subtitle_mode == SubtitleMode::Burn && !supports_burn_in {
						console.lock().unwrap().push_str(
							"⚠ This ffmpeg has no subtitles filter (built without libass) — burn-in skipped\n"
						);
						SubtitleMode::None
					} else {
						file.subtitle_mode.clone()
					},
				};

				let duration = trimmed_duration(duration, &options);

				let use_two_pass = two_pass && supports_two_pass(&preset);
				if use_two_pass {
					options.pass_log = Some(
						temp_dir.join(format!("lomux_pass_{}", idx)).to_string_lossy().to_string()
					);
				}
				let passes: Vec<Option<u8>> = if use_two_pass { vec![Some(1), Some(2)] } else { vec![None] };
				let pass_count = passes.len() as f32;

				console.lock().unwrap().push_str(&format!(
					"\n─── Converting {} ({}/{}) ───\n",
					file.display_name(), current, total
				));

				let mut exit_status: std::io::Result<std::process::ExitStatus> =
					Err(std::io::Error::other("no ffmpeg run"));
				let mut spawn_failed = false;

				for (pass_idx, pass) in passes.iter().enumerate() {
					if *cancel_flag.lock().unwrap() {
						break;
					}

					options.pass = *pass;
					if let Some(number) = pass {
						*status.lock().unwrap() = format!(
							"Converting {}/{} (pass {}/2): {}",
							current, total, number, file.display_name()
						);
					}

					let args = build_ffmpeg_args(&preset, &file.path, &output, metadata, &options);

					let child_result = Command::new(&ffmpeg)
						.args(&args)
						.stdout(Stdio::piped())
						.stderr(Stdio::piped())
						.spawn();

					let mut child = match child_result {
						Ok(c) => c,
						Err(e) => {
							console.lock().unwrap().push_str(&format!("❌ Failed to start ffmpeg: {}\n", e));
							spawn_failed = true;
							break;
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
									let phase_offset = if file.is_youtube { DOWNLOAD_PHASE_SHARE } else { 0.0 };
									let phase_span = 1.0 - phase_offset;
									let pass_progress = (pass_idx as f32 + file_progress as f32) / pass_count;
									let total_progress = (processed as f32 + phase_offset + phase_span * pass_progress)
										/ total as f32 * 100.0;
									*progress.lock().unwrap() = total_progress;
								}
							}
						} else if line.starts_with("frame=") || line.starts_with("size=") || line.starts_with("speed=") {
							// progress lines — skip console noise
						} else if !line.trim().is_empty() && !FFMPEG_PROGRESS_KEYS.iter().any(|k| line.starts_with(k)) {
							let mut out = console.lock().unwrap();
							out.push_str(&line);
							out.push('\n');
						}
					}

					*current_child.lock().unwrap() = None;

					exit_status = child.wait();
					if !exit_status.as_ref().map(|s| s.success()).unwrap_or(false) {
						break;
					}
				}

				if use_two_pass {
					if let Some(ref log) = options.pass_log {
						for suffix in ["-0.log", "-0.log.mbtree", ".log", ".log.mbtree"] {
							let _ = std::fs::remove_file(format!("{}{}", log, suffix));
						}
					}
				}

				if spawn_failed {
					failed += 1;
					continue;
				}

				let (produced, size) = if is_sequence {
					sequence_stats(&artifact)
				} else {
					(artifact.exists(), std::fs::metadata(&artifact).map(|m| m.len()).unwrap_or(0))
				};

				if produced && exit_status.map(|s| s.success()).unwrap_or(false) {
					let size_str = format_file_size(size);
					if is_sequence {
						let frames = std::fs::read_dir(&artifact).map(|e| e.count()).unwrap_or(0);
						console.lock().unwrap().push_str(&format!(
							"✅ Created: {} ({} frames, {})\n",
							artifact.display(), frames, size_str
						));
					} else {
						console.lock().unwrap().push_str(&format!("✅ Created: {} ({})\n", artifact.display(), size_str));
					}
					succeeded += 1;
				} else {
					let cancelled = *cancel_flag.lock().unwrap();
					if is_sequence {
						let _ = std::fs::remove_dir_all(&artifact);
					} else if artifact.exists() {
						let _ = std::fs::remove_file(&artifact);
					}
					if !cancelled {
						console.lock().unwrap().push_str("❌ Failed to create output file\n");
						failed += 1;
					}
				}

				processed += 1;
				*progress.lock().unwrap() = (processed as f32 / total as f32) * 100.0;

				if file.is_youtube && file.path.exists() {
					let _ = std::fs::remove_file(&file.path);
				}
			}

			for path in &downloaded_temp_files {
				let _ = std::fs::remove_file(path);
			}
			purge_temp_downloads(&temp_dir);

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
			kill_pid(pid);
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
						let live_states = self.download_states.lock().unwrap().clone();
						for (idx, file) in self.media_files.iter_mut().enumerate() {
							ui.horizontal(|ui| {
								let is_selected = self.selected_file_index == Some(idx);
								let state = live_states.get(idx).unwrap_or(&file.download_status);

								let icon = if file.is_youtube {
									match state {
										DownloadStatus::Downloading(_) => "⏳",
										DownloadStatus::Downloaded => "✅",
										DownloadStatus::Failed => "❌",
										_ => "📺",
									}
								} else {
									"📄"
								};

								let name = file.display_name();
								let label = match state {
									DownloadStatus::Downloading(fraction) if file.is_youtube => {
										format!("{} {} — {:.0}%", icon, truncate_str(&name, 38), fraction * 100.0)
									}
									_ => format!("{} {}", icon, truncate_str(&name, 45)),
								};

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

	fn import_presets(&mut self) {
		let Some(paths) = rfd::FileDialog::new()
			.set_title("Import Presets")
			.add_filter("Preset JSON", &["json"])
			.pick_files()
		else {
			return;
		};

		let mut added = 0;
		for path in paths {
			let Ok(text) = std::fs::read_to_string(&path) else { continue };
			let Some(presets) = parse_presets_json(&text) else { continue };
			for mut preset in presets {
				preset.category = PresetCategory::Custom;
				if !self.imported_presets.iter().any(|p| p.name == preset.name) {
					self.imported_presets.push(preset);
					added += 1;
				}
			}
		}

		if added > 0 {
			save_imported_presets(&self.imported_presets);
		}
	}

	fn export_active_preset(&mut self) {
		let preset = self.active_preset().clone();
		let suggested = format!("{}.json", sanitize_filename(&preset.name));
		if let Some(path) = rfd::FileDialog::new()
			.set_title("Export Preset")
			.add_filter("Preset JSON", &["json"])
			.set_file_name(suggested)
			.save_file()
		{
			if let Ok(json) = serde_json::to_string_pretty(&preset) {
				let _ = std::fs::write(path, json);
			}
		}
	}

	fn show_custom_preset(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			if ui.button("📥 Import…").on_hover_text("Load presets from a JSON file").clicked() {
				self.import_presets();
			}
			if ui.button("📤 Export…").on_hover_text("Save the current preset as JSON").clicked() {
				self.export_active_preset();
			}
		});

		if !self.imported_presets.is_empty() {
			ui.add_space(4.0);
			ui.label(egui::RichText::new("Imported presets").small().weak());

			let mut remove_idx = None;
			let mut load_idx = None;
			egui::ScrollArea::vertical()
				.id_salt("imported_presets")
				.max_height(80.0)
				.show(ui, |ui| {
					ui.set_min_width(ui.available_width() - 10.0);
					for (idx, preset) in self.imported_presets.iter().enumerate() {
						ui.horizontal(|ui| {
							let is_loaded = self.custom_preset.name == preset.name;
							if ui.selectable_label(is_loaded, &preset.name)
								.on_hover_text(preset.info_line())
								.clicked()
							{
								load_idx = Some(idx);
							}
							ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
								if ui.small_button("✕").on_hover_text("Remove").clicked() {
									remove_idx = Some(idx);
								}
							});
						});
					}
				});

			if let Some(idx) = load_idx {
				self.custom_preset = self.imported_presets[idx].clone();
			}
			if let Some(idx) = remove_idx {
				self.imported_presets.remove(idx);
				save_imported_presets(&self.imported_presets);
			}
			ui.separator();
		}

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

	fn show_trim_panel(&mut self, ui: &mut egui::Ui) {
		ui.collapsing("Trim", |ui| {
			let Some(idx) = self.selected_file_index else {
				ui.label(
					egui::RichText::new("Select a file to set in and out points")
						.weak()
				);
				return;
			};
			if idx >= self.media_files.len() {
				return;
			}

			let filename = self.media_files[idx].display_name();
			ui.label(
				egui::RichText::new(format!("Trimming: {}", truncate_str(&filename, 40)))
					.small()
					.weak()
			);

			ui.horizontal(|ui| {
				ui.label("In:");
				ui.add(
					egui::TextEdit::singleline(&mut self.media_files[idx].trim_start)
						.hint_text("00:00:05")
						.desired_width(84.0)
				);
				ui.label("Out:");
				ui.add(
					egui::TextEdit::singleline(&mut self.media_files[idx].trim_end)
						.hint_text("00:01:30")
						.desired_width(84.0)
				);
				if ui.small_button("Clear").clicked() {
					self.media_files[idx].trim_start.clear();
					self.media_files[idx].trim_end.clear();
				}
			});

			let start_text = self.media_files[idx].trim_start.clone();
			let end_text = self.media_files[idx].trim_end.clone();
			if let Some(problem) = trim_problem(&start_text, &end_text) {
				ui.colored_label(egui::Color32::from_rgb(255, 120, 120), problem);
			} else if !start_text.trim().is_empty() || !end_text.trim().is_empty() {
				ui.label(
					egui::RichText::new("Times accept SS, MM:SS, or HH:MM:SS")
						.small()
						.weak()
				);
			}

			if ui.small_button("Apply to All").clicked() {
				for file in &mut self.media_files {
					file.trim_start = start_text.clone();
					file.trim_end = end_text.clone();
				}
			}
		});
	}

	fn show_subtitle_panel(&mut self, ui: &mut egui::Ui) {
		ui.collapsing("Subtitles", |ui| {
			let Some(idx) = self.selected_file_index else {
				ui.label(
					egui::RichText::new("Select a file to attach subtitles")
						.weak()
				);
				return;
			};
			if idx >= self.media_files.len() {
				return;
			}

			ui.horizontal(|ui| {
				ui.label("Mode:");
			let can_burn = self.tools.supports_burn_in;
				for mode in [SubtitleMode::None, SubtitleMode::Burn, SubtitleMode::Soft] {
					let label = mode.label().to_string();
					let enabled = mode != SubtitleMode::Burn || can_burn;
					ui.add_enabled_ui(enabled, |ui| {
						let response = ui.selectable_value(
							&mut self.media_files[idx].subtitle_mode,
							mode,
							label,
						);
						if !enabled {
							response.on_hover_text(
								"This ffmpeg was built without libass, so it has no subtitles filter. Reinstall ffmpeg with libass to burn subtitles in."
							);
						}
					});
				}
			});

			if self.media_files[idx].subtitle_mode == SubtitleMode::None {
				return;
			}

			ui.horizontal(|ui| {
				if ui.button("Choose file…").clicked() {
					if let Some(path) = rfd::FileDialog::new()
						.set_title("Select Subtitle File")
						.add_filter("Subtitles", &["srt", "ass", "ssa", "vtt"])
						.pick_file()
					{
						self.media_files[idx].subtitle_path = Some(path);
					}
				}
				if ui.small_button("Clear").clicked() {
					self.media_files[idx].subtitle_path = None;
					self.media_files[idx].subtitle_mode = SubtitleMode::None;
				}
			});

			match self.media_files[idx].subtitle_path.clone() {
				Some(path) => {
					ui.label(
						egui::RichText::new(truncate_path(&path, 46))
							.small()
							.weak()
					);
				}
				None => {
					ui.colored_label(
						egui::Color32::from_rgb(255, 120, 120),
						"Pick a subtitle file or the setting is ignored"
					);
				}
			}

			if self.media_files[idx].subtitle_mode == SubtitleMode::Soft
				&& subtitle_codec_for(&self.active_preset().container).is_none()
			{
				ui.colored_label(
					egui::Color32::from_rgb(230, 150, 80),
					"This container cannot carry a subtitle track — use MP4, MOV, MKV, WebM, or burn in instead"
				);
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

	fn show_about_window(&mut self, ctx: &egui::Context) {
		let mut open = self.about_open;
		egui::Window::new("About LoMux")
			.open(&mut open)
			.collapsible(false)
			.resizable(false)
			.anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
			.show(ctx, |ui| {
				ui.heading(egui::RichText::new("LoMux").color(self.theme.accent));
				ui.label(format!("Version {}", VERSION));
				ui.add_space(6.0);
				ui.label("Lightweight media converter and YouTube downloader.");
				ui.label("Wraps ffmpeg and yt-dlp in a native interface.");
				ui.add_space(8.0);
				ui.separator();
				ui.add_space(6.0);

				ui.label(egui::RichText::new("Detected tools").small().weak());
				for (name, path) in [
					("ffmpeg", self.tools.ffmpeg_path.clone()),
					("ffprobe", self.tools.ffprobe_path.clone()),
					("yt-dlp", self.tools.ytdlp_path.clone()),
				] {
					match path {
						Some(path) => {
							ui.label(
								egui::RichText::new(format!("{}  {}", name, truncate_path(&path, 42)))
									.small()
							);
						}
						None => {
							ui.label(
								egui::RichText::new(format!("{}  not found", name))
									.small()
									.color(self.theme.text_secondary)
							);
						}
					}
				}
				if !self.tools.supports_burn_in {
					ui.label(
						egui::RichText::new("subtitle burn-in unavailable — ffmpeg built without libass")
							.small()
							.color(self.theme.text_secondary)
					);
				}

				ui.add_space(8.0);
				ui.label(egui::RichText::new("MIT licensed · github.com/zblauser/LoMux").small().weak());
			});
		self.about_open = open;
	}

	fn show_theme_editor(&mut self, ui: &mut egui::Ui) {
		let theme_before = self.theme.clone();
		let custom_before = self.custom_theme.clone();

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

						let is_custom = self.theme.name == CUSTOM_THEME;
						let btn = egui::Button::new(
							egui::RichText::new(CUSTOM_THEME)
								.small()
								.color(if is_custom { self.custom_theme.accent } else { self.theme.text_primary })
						);
						if ui.add(btn).on_hover_text("Your own colours, kept between sessions").clicked() {
							self.theme = self.custom_theme.clone();
						}
					});
				});

			ui.separator();

			let editing_custom = self.theme.name == CUSTOM_THEME;

			if editing_custom {
				ui.label(egui::RichText::new("Colours").small());

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
					}
				});

				ui.horizontal(|ui| {
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
					}

					ui.separator();
					if ui.small_button("Reset").on_hover_text("Back to the default custom colours").clicked() {
						let rounding = self.theme.rounding;
						self.theme = default_custom_theme();
						self.theme.rounding = rounding;
					}
				});

				ui.separator();
			} else {
				ui.horizontal(|ui| {
					ui.label(
						egui::RichText::new("Colours are fixed for curated themes")
							.small()
							.weak()
					);
					if ui.small_button("Copy to Custom").on_hover_text("Start a custom theme from these colours").clicked() {
						let mut copied = self.theme.clone();
						copied.name = CUSTOM_THEME.into();
						self.custom_theme = copied.clone();
						self.theme = copied;
					}
				});
				ui.separator();
			}

			ui.label(egui::RichText::new("Shape").small());
			ui.horizontal(|ui| {
				ui.label("Rounding:");
				ui.add(egui::Slider::new(&mut self.theme.rounding, 0.0..=14.0).suffix("px"));
			});
		});

		if self.theme.name == CUSTOM_THEME {
			self.custom_theme = self.theme.clone();
		}

		if self.theme != theme_before || self.custom_theme != custom_before {
			save_theme(&self.theme, &self.custom_theme);
		}
	}
}

// ============= HELPER FUNCTIONS =============

fn build_ffmpeg_args(
	preset: &EncodingPreset,
	input: &Path,
	output: &Path,
	metadata: &AudioMetadata,
	options: &EncodeOptions,
) -> Vec<String> {
	let mut args = vec![
		"-hide_banner".into(),
		"-loglevel".into(), "warning".into(),
		"-stats".into(),
		"-progress".into(), "pipe:2".into(),
	];

	let start_seconds = options.trim_start.as_deref().and_then(parse_timecode);
	if let Some(start) = start_seconds {
		args.extend(["-ss".into(), format_timecode(start)]);
	}

	args.extend(["-i".into(), input.to_string_lossy().to_string()]);

	let soft_subs = options.subtitle_mode == SubtitleMode::Soft
		&& options.subtitle_path.is_some()
		&& subtitle_codec_for(&preset.container).is_some();
	if soft_subs {
		if let Some(ref subs) = options.subtitle_path {
			args.extend(["-i".into(), subs.to_string_lossy().to_string()]);
		}
	}

	if let Some(end) = options.trim_end.as_deref().and_then(parse_timecode) {
		let duration = end - start_seconds.unwrap_or(0.0);
		if duration > 0.0 {
			args.extend(["-t".into(), format_timecode(duration)]);
		}
	}

	if preset.container.is_image_sequence() {
		if preset.single_image {
			args.extend(["-frames:v".into(), "1".into()]);
			args.extend(["-update".into(), "1".into()]);
		}
		let mut filters = Vec::new();
		if let Some(fps) = preset.fps {
			filters.push(format!("fps={}", fps));
		}
		if let Some(ref resolution) = preset.resolution {
			if resolution.contains("x") || resolution.contains(":") {
				filters.push(format!("scale={}", resolution.replace("x", ":")));
			}
		}
		if !filters.is_empty() {
			args.extend(["-vf".into(), filters.join(",")]);
		}
		if let Some(encoder) = preset.container.image_encoder() {
			args.extend(["-c:v".into(), encoder.into()]);
		}
		if preset.container == Container::Jpg {
			args.extend(["-q:v".into(), "2".into()]);
		}
		if preset.container == Container::Tiff {
			args.extend(["-pix_fmt".into(), "rgb24".into()]);
			args.extend(["-compression_algo".into(), "raw".into()]);
		}
		args.push("-an".into());
	} else if preset.container == Container::Gif {
		let fps = preset.fps.unwrap_or(15);
		let scale = preset.resolution.as_ref()
			.map(|r| format!(",scale={}:flags=lanczos", r.replace("x", ":")))
			.unwrap_or_default();
		args.extend([
			"-filter_complex".into(),
			format!("fps={}{},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse", fps, scale),
		]);
		args.extend(["-loop".into(), "0".into()]);
		args.push("-an".into());
	} else {
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
				let profile = preset.codec_profile.clone().unwrap_or_else(|| "3".to_string());
				args.extend(["-profile:v".into(), profile.clone()]);
				args.extend(["-vendor".into(), "apl0".into()]);
				if profile == "4" || profile == "5" {
					args.extend(["-pix_fmt".into(), "yuv444p10le".into()]);
				}
			}
			VideoCodec::DnxHd => {
				args.extend(["-c:v".into(), "dnxhd".into()]);
				if let Some(ref profile) = preset.codec_profile {
					args.extend(["-profile:v".into(), profile.clone()]);
					let pix_fmt = match profile.as_str() {
						"dnxhr_hqx" => "yuv422p10le",
						"dnxhr_444" => "yuv444p10le",
						_ => "yuv422p",
					};
					args.extend(["-pix_fmt".into(), pix_fmt.into()]);
				} else if let Some(bitrate) = preset.video_bitrate {
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
				let encoder = if preset.container == Container::Aiff { "pcm_s16be" } else { "pcm_s16le" };
				args.extend(["-c:a".into(), encoder.into()]);
			}
			AudioCodec::Ac3 => {
				args.extend(["-c:a".into(), "ac3".into()]);
				if let Some(bitrate) = preset.audio_bitrate {
					args.extend(["-b:a".into(), format!("{}k", bitrate)]);
				}
			}
			AudioCodec::Copy => {
				args.extend(["-c:a".into(), "copy".into()]);
			}
			AudioCodec::None => {
				args.push("-an".into());
			}
		}

		if preset.audio_codec != AudioCodec::Copy && preset.audio_codec != AudioCodec::None {
			if let Some(channels) = preset.audio_channels {
				args.extend(["-ac".into(), channels.to_string()]);
			}
			if let Some(rate) = preset.audio_sample_rate {
				args.extend(["-ar".into(), rate.to_string()]);
			} else if preset.container == Container::Mxf {
				args.extend(["-ar".into(), "48000".into()]);
			}
		}

		if options.normalize_audio
			&& preset.audio_codec != AudioCodec::Copy
			&& preset.audio_codec != AudioCodec::None
		{
			args.extend(["-af".into(), LOUDNORM_FILTER.into()]);
		}

		if preset.video_codec != VideoCodec::Copy && preset.video_codec != VideoCodec::None {
			let mut filters = Vec::new();
			if let Some(ref resolution) = preset.resolution {
				if resolution.contains("x") || resolution.contains(":") {
					filters.push(format!("scale={}", resolution.replace("x", ":")));
				}
			}
			if options.subtitle_mode == SubtitleMode::Burn {
				if let Some(ref subs) = options.subtitle_path {
					filters.push(format!("subtitles='{}'", escape_filter_path(subs)));
				}
			}
			if !filters.is_empty() {
				args.extend(["-vf".into(), filters.join(",")]);
			}
			if let Some(fps) = preset.fps {
				args.extend(["-r".into(), fps.to_string()]);
			}
		}

		if soft_subs {
			if let Some(codec) = subtitle_codec_for(&preset.container) {
				args.extend(["-map".into(), "0".into()]);
				args.extend(["-map".into(), "1".into()]);
				args.extend(["-c:s".into(), codec.into()]);
			}
		}
	}

	if let Some(pass) = options.pass {
		args.extend(["-pass".into(), pass.to_string()]);
		if let Some(ref log) = options.pass_log {
			args.extend(["-passlogfile".into(), log.clone()]);
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

	if !options.extra_args.is_empty() {
		args.extend(
			options.extra_args.split_whitespace()
				.map(|s| s.to_string())
		);
	}

	if options.pass == Some(1) {
		args.extend(["-an".into(), "-f".into(), "null".into(), "-y".into(), "-".into()]);
	} else {
		args.extend(["-y".into(), output.to_string_lossy().to_string()]);
	}
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
	on_progress: &dyn Fn(f32),
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
			if let Some(fraction) = parse_download_percent(&line) {
				on_progress(fraction);
			}
		} else if !line.trim().is_empty() {
			let mut out = console.lock().unwrap();
			out.push_str(&line);
			out.push('\n');
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

fn sanitize_filename(name: &str) -> String {
	let cleaned: String = name
		.chars()
		.map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '_' } else { c })
		.collect();
	cleaned.split_whitespace().collect::<Vec<_>>().join(" ").trim_matches('.').to_string()
}

fn apply_filename_template(
	template: &str,
	stem: &str,
	metadata: &AudioMetadata,
	preset_name: &str,
	extension: &str,
	index: usize,
) -> String {
	let field = |value: &Option<String>| value.clone().unwrap_or_default();
	let rendered = template
		.replace("{name}", stem)
		.replace("{title}", &field(&metadata.title))
		.replace("{artist}", &field(&metadata.artist))
		.replace("{album}", &field(&metadata.album))
		.replace("{year}", &field(&metadata.year))
		.replace("{genre}", &field(&metadata.genre))
		.replace("{track}", &field(&metadata.track))
		.replace("{preset}", preset_name)
		.replace("{index}", &(index + 1).to_string())
		.replace("{ext}", extension);

	let suffix = format!(".{}", extension);
	let trimmed = rendered.strip_suffix(&suffix).unwrap_or(&rendered);
	let cleaned = sanitize_filename(trimmed);

	if cleaned.is_empty() { stem.to_string() } else { cleaned }
}

fn set_download_state(states: &Arc<Mutex<Vec<DownloadStatus>>>, idx: usize, state: DownloadStatus) {
	if let Ok(mut states) = states.lock() {
		if let Some(slot) = states.get_mut(idx) {
			*slot = state;
		}
	}
}

fn parse_download_percent(line: &str) -> Option<f32> {
	let after = line.split("[download]").nth(1)?;
	let idx = after.find('%')?;
	let value = after[..idx].split_whitespace().last()?;
	value.parse::<f32>().ok().map(|p| (p / 100.0).clamp(0.0, 1.0))
}

fn kill_pid(pid: u32) {
	#[cfg(unix)]
	unsafe { libc::kill(pid as i32, libc::SIGTERM); }
	#[cfg(windows)]
	{
		let _ = Command::new("taskkill")
			.args(["/PID", &pid.to_string(), "/F"])
			.output();
	}
	let _ = pid;
}

fn kill_process(child: &Child) {
	kill_pid(child.id());
}

fn trim_problem(start: &str, end: &str) -> Option<&'static str> {
	let start_set = !start.trim().is_empty();
	let end_set = !end.trim().is_empty();

	let parsed_start = parse_timecode(start);
	let parsed_end = parse_timecode(end);

	if start_set && parsed_start.is_none() {
		return Some("In point is not a valid time");
	}
	if end_set && parsed_end.is_none() {
		return Some("Out point is not a valid time");
	}
	if let (Some(s), Some(e)) = (parsed_start, parsed_end) {
		if e <= s {
			return Some("Out point must come after the in point");
		}
	}
	None
}

#[cfg(target_os = "macos")]
thread_local! {
	static VIEW_MENU_ITEMS: std::cell::RefCell<Option<(muda::CheckMenuItem, muda::CheckMenuItem)>> =
		const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "macos")]
fn init_native_menu() {
	use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

	let menu = Menu::new();

	let app_menu = Submenu::new("LoMux", true);
	let _ = app_menu.append_items(&[
		&MenuItem::with_id("about", "About LoMux", true, None),
		&PredefinedMenuItem::separator(),
		&MenuItem::with_id("rescan", "Rescan Tools", true, None),
		&PredefinedMenuItem::separator(),
		&PredefinedMenuItem::services(None),
		&PredefinedMenuItem::separator(),
		&PredefinedMenuItem::hide(None),
		&PredefinedMenuItem::hide_others(None),
		&PredefinedMenuItem::show_all(None),
		&PredefinedMenuItem::separator(),
		&PredefinedMenuItem::quit(None),
	]);

	let file_menu = Submenu::new("File", true);
	let _ = file_menu.append_items(&[
		&MenuItem::with_id("add_files", "Add Files…", true, None),
		&MenuItem::with_id("output_dir", "Set Output Folder…", true, None),
		&PredefinedMenuItem::separator(),
		&MenuItem::with_id("import_presets", "Import Presets…", true, None),
		&MenuItem::with_id("export_preset", "Export Current Preset…", true, None),
		&PredefinedMenuItem::separator(),
		&MenuItem::with_id("clear_queue", "Clear Queue", true, None),
	]);

	let theme_item = CheckMenuItem::with_id("toggle_theme_editor", "Theme Editor", true, false, None);
	let preset_item = CheckMenuItem::with_id("toggle_preset_info", "Preset Details", true, true, None);
	let view_menu = Submenu::new("View", true);
	let _ = view_menu.append_items(&[&theme_item, &preset_item]);
	VIEW_MENU_ITEMS.with(|items| {
		*items.borrow_mut() = Some((theme_item, preset_item));
	});

	let help_menu = Submenu::new("Help", true);
	let _ = help_menu.append_items(&[
		&MenuItem::with_id("about", "Requirements & Detected Tools", true, None),
		&MenuItem::with_id("github", "Project on GitHub", true, None),
	]);

	let _ = menu.append_items(&[&app_menu, &file_menu, &view_menu, &help_menu]);
	menu.init_for_nsapp();
	std::mem::forget(menu);
}

fn strip_youtube_prefix(stem: &str) -> String {
	let Some(rest) = stem.strip_prefix("yt_") else {
		return stem.to_string();
	};
	let Some((timestamp, name)) = rest.split_once('_') else {
		return stem.to_string();
	};
	if !timestamp.is_empty() && timestamp.chars().all(|c| c.is_ascii_digit()) && !name.is_empty() {
		name.to_string()
	} else {
		stem.to_string()
	}
}

fn purge_temp_downloads(temp_dir: &Path) {
	let Ok(entries) = std::fs::read_dir(temp_dir) else {
		return;
	};
	for entry in entries.flatten() {
		let name = entry.file_name().to_string_lossy().to_string();
		if name.starts_with("yt_") || name.starts_with("lomux_pass_") {
			let _ = std::fs::remove_file(entry.path());
		}
	}
}

fn some_if_set(value: &str) -> Option<String> {
	if value.trim().is_empty() { None } else { Some(value.trim().to_string()) }
}

fn trimmed_duration(duration: f64, options: &EncodeOptions) -> f64 {
	if duration <= 0.0 {
		return duration;
	}
	let start = options.trim_start.as_deref().and_then(parse_timecode).unwrap_or(0.0);
	let end = options.trim_end.as_deref().and_then(parse_timecode).unwrap_or(duration);
	let span = end.min(duration) - start;
	if span > 0.0 { span } else { duration }
}

fn escape_filter_path(path: &Path) -> String {
	path.to_string_lossy()
		.replace('\\', "/")
		.replace('\'', "\u{5c}\u{5c}\u{5c}'")
		.replace(':', "\u{5c}\u{5c}:")
}

fn subtitle_codec_for(container: &Container) -> Option<&'static str> {
	match container {
		Container::Mp4 | Container::Mov => Some("mov_text"),
		Container::Mkv | Container::Webm => Some("srt"),
		_ => None,
	}
}

fn parse_timecode(value: &str) -> Option<f64> {
	let value = value.trim();
	if value.is_empty() {
		return None;
	}

	let mut seconds = 0.0;
	for part in value.split(':') {
		let parsed: f64 = part.parse().ok()?;
		if parsed < 0.0 {
			return None;
		}
		seconds = seconds * 60.0 + parsed;
	}

	if value.split(':').count() > 3 { None } else { Some(seconds) }
}

fn format_timecode(seconds: f64) -> String {
	let total = seconds.max(0.0);
	let hours = (total / 3600.0).floor();
	let minutes = ((total - hours * 3600.0) / 60.0).floor();
	let secs = total - hours * 3600.0 - minutes * 60.0;
	format!("{:02}:{:02}:{:06.3}", hours as u64, minutes as u64, secs)
}

fn sequence_stats(dir: &Path) -> (bool, u64) {
	let Ok(entries) = std::fs::read_dir(dir) else {
		return (false, 0);
	};
	let mut count = 0usize;
	let mut size = 0u64;
	for entry in entries.flatten() {
		if let Ok(meta) = entry.metadata() {
			if meta.is_file() {
				count += 1;
				size += meta.len();
			}
		}
	}
	(count > 0, size)
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
	if s.chars().count() <= max {
		s.to_string()
	} else {
		let kept: String = s.chars().take(max.saturating_sub(1)).collect();
		format!("{}…", kept)
	}
}

fn truncate_path(path: &Path, max: usize) -> String {
	let s = path.to_string_lossy();
	if s.chars().count() <= max {
		s.to_string()
	} else {
		let filename = path.file_name().unwrap_or_default().to_string_lossy();
		if filename.chars().count() + 4 > max {
			truncate_str(&s, max)
		} else {
			format!("…/{}", filename)
		}
	}
}

// ============= APP IMPLEMENTATION =============

impl LoMuxApp {
	#[cfg(target_os = "macos")]
	fn handle_native_menu(&mut self, ctx: &egui::Context) {
		while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
			match event.id().0.as_str() {
				"about" => self.about_open = true,
				"rescan" => self.tools.detect_all(),
				"add_files" => self.select_input_files(),
				"output_dir" => self.select_output_dir(),
				"import_presets" => self.import_presets(),
				"export_preset" => self.export_active_preset(),
				"clear_queue" => {
					self.media_files.clear();
					self.selected_file_index = None;
				}
				"toggle_theme_editor" => self.theme_editor_open = !self.theme_editor_open,
				"toggle_preset_info" => self.show_preset_info = !self.show_preset_info,
				"github" => ctx.open_url(egui::OpenUrl::new_tab("https://github.com/zblauser/LoMux")),
				_ => {}
			}
		}

		self.sync_native_menu_state();
	}

	#[cfg(target_os = "macos")]
	fn sync_native_menu_state(&self) {
		VIEW_MENU_ITEMS.with(|items| {
			if let Some((theme_item, preset_item)) = items.borrow().as_ref() {
				if theme_item.is_checked() != self.theme_editor_open {
					theme_item.set_checked(self.theme_editor_open);
				}
				if preset_item.is_checked() != self.show_preset_info {
					preset_item.set_checked(self.show_preset_info);
				}
			}
		});
	}
}

impl eframe::App for LoMuxApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		self.theme.apply(ctx);

		#[cfg(target_os = "macos")]
		self.handle_native_menu(ctx);

		// Handle drag and drop
		ctx.input(|i| {
			for file in &i.raw.dropped_files {
				if let Some(ref path) = file.path {
					self.media_files.push(MediaFile::new(path.clone()));
				}
			}
		});

		#[cfg(not(target_os = "macos"))]
		egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
			egui::menu::bar(ui, |ui| {
				ui.menu_button("LoMux", |ui| {
					if ui.button("About LoMux").clicked() {
						self.about_open = true;
						ui.close_menu();
					}
					if ui.button("Rescan Tools").clicked() {
						self.tools.detect_all();
						ui.close_menu();
					}
					ui.separator();
					if ui.button("Quit").clicked() {
						ctx.send_viewport_cmd(egui::ViewportCommand::Close);
					}
				});

				ui.menu_button("File", |ui| {
					if ui.button("Add Files…").clicked() {
						self.select_input_files();
						ui.close_menu();
					}
					if ui.button("Set Output Folder…").clicked() {
						self.select_output_dir();
						ui.close_menu();
					}
					ui.separator();
					if ui.button("Import Presets…").clicked() {
						self.import_presets();
						ui.close_menu();
					}
					if ui.button("Export Current Preset…").clicked() {
						self.export_active_preset();
						ui.close_menu();
					}
					ui.separator();
					let has_files = !self.media_files.is_empty();
					if ui.add_enabled(has_files, egui::Button::new("Clear Queue")).clicked() {
						self.media_files.clear();
						self.selected_file_index = None;
						ui.close_menu();
					}
				});

				ui.menu_button("View", |ui| {
					ui.checkbox(&mut self.theme_editor_open, "Theme editor");
					ui.checkbox(&mut self.show_preset_info, "Preset details");
				});

				ui.menu_button("Help", |ui| {
					if ui.button("Requirements & detected tools").clicked() {
						self.about_open = true;
						ui.close_menu();
					}
					if ui.button("Project on GitHub").clicked() {
						ctx.open_url(egui::OpenUrl::new_tab("https://github.com/zblauser/LoMux"));
						ui.close_menu();
					}
				});
			});
		});

		self.show_about_window(ctx);

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
									self.show_trim_panel(ui);
									ui.add_space(4.0);
									self.show_subtitle_panel(ui);
									ui.add_space(4.0);

									ui.collapsing("Advanced", |ui| {
										ui.checkbox(&mut self.normalize_audio, "Normalize loudness (EBU R128)")
											.on_hover_text("Applies loudnorm=I=-16:TP=-1.5:LRA=11. Skipped when audio is copied or disabled.");
										ui.checkbox(&mut self.two_pass, "Two-pass encoding")
											.on_hover_text("Analyses the file first, then encodes. Better quality at the same bitrate, roughly twice the time. Only applies to bitrate presets using H.264, H.265, VP8, or VP9.");
										ui.add_space(4.0);
										ui.label(
											egui::RichText::new("Output filename template:")
												.small()
										);
										ui.add(
											egui::TextEdit::singleline(&mut self.filename_template)
												.hint_text("{artist} - {title}")
												.desired_width(ui.available_width())
										).on_hover_text(
											"Tokens: {name} {title} {artist} {album} {year} {genre} {track} {preset} {index} {ext}. \
											Empty keeps the original filename. Metadata tokens use whatever metadata this file is set to write."
										);
										ui.add_space(4.0);
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

fn print_cli_help() {
	println!("LoMux {}", VERSION);
	println!("Lightweight media converter and YouTube downloader.");
	println!();
	println!("Usage: lomux [OPTIONS]");
	println!();
	println!("Options:");
	println!("  -h, --help       Print this help and exit");
	println!("  -V, --version    Print the version and exit");
	println!();
	println!("Run without options to open the application.");
	println!("Requires ffmpeg on PATH; yt-dlp is optional and enables YouTube downloads.");
}

fn main() -> eframe::Result {
	for arg in std::env::args().skip(1) {
		match arg.as_str() {
			"-h" | "--help" => {
				print_cli_help();
				return Ok(());
			}
			"-V" | "--version" => {
				println!("LoMux {}", VERSION);
				return Ok(());
			}
			_ => {}
		}
	}

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
		"LoMux",
		options,
		Box::new(|_cc| {
			#[cfg(target_os = "macos")]
			init_native_menu();
			Ok(Box::new(LoMuxApp::new()))
		}),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn preset(
		name: &str,
		container: Container,
		video_codec: VideoCodec,
		audio_codec: AudioCodec,
		video_bitrate: Option<u32>,
		audio_bitrate: Option<u32>,
		video_crf: Option<u8>,
		fps: Option<u8>,
		resolution: Option<&str>,
	) -> EncodingPreset {
		EncodingPreset {
			name: name.into(),
			category: PresetCategory::Custom,
			container,
			video_codec,
			audio_codec,
			video_bitrate,
			audio_bitrate,
			video_crf,
			fps,
			resolution: resolution.map(String::from),
			single_image: false,
			codec_profile: None,
			audio_channels: None,
			audio_sample_rate: None,
			description: String::new(),
		}
	}

	fn in_out() -> (PathBuf, PathBuf) {
		(PathBuf::from("/in.mp4"), PathBuf::from("/out.mp4"))
	}

	#[test]
	fn youtube_1080p_args_stable() {
		let p = preset(
			"YouTube 1080p HD", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(8000), Some(320), None, None, Some("1920x1080"),
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert_eq!(args, vec![
			"-hide_banner", "-loglevel", "warning",
			"-stats", "-progress", "pipe:2",
			"-i", "/in.mp4",
			"-c:v", "libx264", "-preset", "medium",
			"-b:v", "8000k",
			"-pix_fmt", "yuv420p",
			"-movflags", "+faststart",
			"-c:a", "aac", "-b:a", "320k",
			"-vf", "scale=1920:1080",
			"-threads", "0",
			"-y", "/out.mp4",
		]);
	}

	#[test]
	fn youtube_4k_uses_hevc_with_hvc1_tag() {
		let p = preset(
			"YouTube 4K UHD", Container::Mp4,
			VideoCodec::H265, AudioCodec::Aac,
			Some(35000), Some(320), None, None, Some("3840x2160"),
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-c:v", "libx265"]));
		assert!(args.windows(2).any(|w| w == ["-tag:v", "hvc1"]));
		assert!(args.windows(2).any(|w| w == ["-vf", "scale=3840:2160"]));
	}

	#[test]
	fn mp3_audio_only_skips_video() {
		let p = preset(
			"MP3 320", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.contains(&"-vn".to_string()));
		assert!(args.windows(2).any(|w| w == ["-c:a", "libmp3lame"]));
		assert!(args.windows(2).any(|w| w == ["-b:a", "320k"]));
		assert!(!args.iter().any(|a| a == "-vf"));
	}

	#[test]
	fn prores_uses_prores_ks() {
		let p = preset(
			"ProRes 422", Container::Mov,
			VideoCodec::ProRes, AudioCodec::Pcm,
			None, None, None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-c:v", "prores_ks"]));
		assert!(args.windows(2).any(|w| w == ["-profile:v", "3"]));
		assert!(args.windows(2).any(|w| w == ["-c:a", "pcm_s16le"]));
	}

	#[test]
	fn discord_preset_applies_fps_and_scale() {
		let p = preset(
			"Discord", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(2500), Some(128), None, Some(30), Some("1280x720"),
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-vf", "scale=1280:720"]));
		assert!(args.windows(2).any(|w| w == ["-r", "30"]));
		assert!(args.windows(2).any(|w| w == ["-b:v", "2500k"]));
	}

	#[test]
	fn crf_takes_precedence_over_bitrate_when_set() {
		let p = preset(
			"H264 CRF", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(9999), Some(128), Some(20), None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-crf", "20"]));
		assert!(!args.windows(2).any(|w| w == ["-b:v", "9999k"]));
	}

	#[test]
	fn video_copy_skips_scale_and_pixfmt() {
		let p = preset(
			"Remux", Container::Mp4,
			VideoCodec::Copy, AudioCodec::Copy,
			None, None, None, None, Some("1920x1080"),
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
		assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
		assert!(!args.iter().any(|a| a == "-vf"));
		assert!(!args.iter().any(|a| a == "-pix_fmt"));
	}

	#[test]
	fn metadata_emits_metadata_flags() {
		let p = preset(
			"MP3", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(192), None, None, None,
		);
		let meta = AudioMetadata {
			title: Some("Song".into()),
			artist: Some("Band".into()),
			album: None,
			year: Some("2026".into()),
			genre: None,
			track: None,
			comment: None,
		};
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &meta, &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-metadata", "title=Song"]));
		assert!(args.windows(2).any(|w| w == ["-metadata", "artist=Band"]));
		assert!(args.windows(2).any(|w| w == ["-metadata", "date=2026"]));
		assert!(!args.iter().any(|a| a == "album="));
	}

	#[test]
	fn extra_args_are_split_on_whitespace() {
		let p = preset(
			"Custom", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(5000), Some(128), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions { extra_args: "-tune film -profile:v high".into(), ..Default::default() });
		assert!(args.windows(2).any(|w| w == ["-tune", "film"]));
		assert!(args.windows(2).any(|w| w == ["-profile:v", "high"]));
	}

	#[test]
	fn output_is_always_last_with_y_flag() {
		let p = preset(
			"Any", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(5000), Some(128), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		let n = args.len();
		assert_eq!(args[n - 2], "-y");
		assert_eq!(args[n - 1], "/out.mp4");
	}

	#[test]
	fn all_builtin_presets_produce_nonempty_args() {
		let (i, o) = in_out();
		for p in EncodingPreset::get_all_presets() {
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
			assert!(args.len() > 6, "preset {} produced too few args", p.name);
			assert_eq!(args[args.len() - 1], "/out.mp4", "preset {} did not end at output", p.name);
		}
	}

	fn meta(artist: &str, title: &str) -> AudioMetadata {
		AudioMetadata {
			title: Some(title.into()),
			artist: Some(artist.into()),
			album: Some("Album".into()),
			year: Some("2026".into()),
			genre: None,
			track: Some("3".into()),
			comment: None,
		}
	}

	#[test]
	#[ignore = "prints args for the ffmpeg smoke script"]
	fn dump_preset_args() {
		let input = PathBuf::from(std::env::var("LOMUX_DUMP_INPUT").unwrap_or_else(|_| "/in.mp4".into()));
		let out_dir = std::env::var("LOMUX_DUMP_OUTDIR").unwrap_or_else(|_| "/tmp".into());
		for p in EncodingPreset::get_all_presets() {
			let stem = sanitize_filename(&p.name).replace(' ', "_");
			let output = if p.container.is_image_sequence() && !p.single_image {
				PathBuf::from(format!("{}/{}_%05d.{}", out_dir, stem, p.container.extension()))
			} else {
				PathBuf::from(format!("{}/{}.{}", out_dir, stem, p.container.extension()))
			};
			let args = build_ffmpeg_args(&p, &input, &output, &AudioMetadata::default(), &EncodeOptions::default());
			println!("PRESET\t{}\t{}", p.name, args.join("\t"));
		}
	}

	#[test]
	fn professional_presets_carry_valid_profiles() {
		let (i, o) = in_out();
		for p in EncodingPreset::get_all_presets() {
			let Some(ref profile) = p.codec_profile else { continue };
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
			assert!(
				args.windows(2).any(|w| w == ["-profile:v", profile.as_str()]),
				"{} did not emit its profile",
				p.name
			);
			match p.video_codec {
				VideoCodec::ProRes => assert!(
					["0", "1", "2", "3", "4", "5"].contains(&profile.as_str()),
					"{} has an invalid ProRes profile",
					p.name
				),
				VideoCodec::DnxHd => assert!(
					profile.starts_with("dnxhr_"),
					"{} has an invalid DNxHR profile",
					p.name
				),
				_ => {}
			}
		}
	}

	#[test]
	fn prores_422_preset_is_actually_422() {
		let p = EncodingPreset::get_all_presets()
			.into_iter()
			.find(|p| p.name == "ProRes 422")
			.expect("preset missing");
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-profile:v", "2"]));
	}

	#[test]
	fn dnxhr_profiles_pick_matching_pixel_formats() {
		let (i, o) = in_out();
		for (profile, expected) in [
			("dnxhr_lb", "yuv422p"),
			("dnxhr_sq", "yuv422p"),
			("dnxhr_hq", "yuv422p"),
			("dnxhr_hqx", "yuv422p10le"),
			("dnxhr_444", "yuv444p10le"),
		] {
			let mut p = preset(
				"DNxHR", Container::Mov,
				VideoCodec::DnxHd, AudioCodec::Pcm,
				None, None, None, None, None,
			);
			p.codec_profile = Some(profile.into());
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
			assert!(args.windows(2).any(|w| w == ["-pix_fmt", expected]), "{} pix_fmt", profile);
		}
	}

	#[test]
	fn mxf_forces_48khz_audio() {
		let (i, o) = in_out();
		let mut p = preset(
			"DNxHR MXF", Container::Mxf,
			VideoCodec::DnxHd, AudioCodec::Pcm,
			None, None, None, None, None,
		);
		p.codec_profile = Some("dnxhr_hq".into());
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-ar", "48000"]), "mxf muxer only accepts 48kHz");
	}

	#[test]
	fn audio_channel_and_rate_controls_emit_flags() {
		let (i, o) = in_out();
		let mut p = preset(
			"Mono podcast", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(128), None, None, None,
		);
		p.audio_channels = Some(1);
		p.audio_sample_rate = Some(44100);
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-ac", "1"]));
		assert!(args.windows(2).any(|w| w == ["-ar", "44100"]));

		let copied = preset(
			"Remux", Container::Mp4,
			VideoCodec::Copy, AudioCodec::Copy,
			None, None, None, None, None,
		);
		let args = build_ffmpeg_args(&copied, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(!args.iter().any(|a| a == "-ac" || a == "-ar"));
	}

	#[test]
	fn burn_in_joins_the_existing_filter_chain() {
		let p = preset(
			"Discord", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(2500), Some(128), None, Some(30), Some("1280x720"),
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			subtitle_mode: SubtitleMode::Burn,
			subtitle_path: Some(PathBuf::from("/subs/show.srt")),
			..Default::default()
		});
		let filter = args.iter()
			.position(|a| a == "-vf")
			.map(|idx| args[idx + 1].clone())
			.expect("no -vf");
		assert_eq!(filter, "scale=1280:720,subtitles='/subs/show.srt'");
		assert_eq!(args.iter().filter(|a| *a == "-vf").count(), 1, "only one -vf may be emitted");
		assert_eq!(args.iter().filter(|a| *a == "-i").count(), 1, "burn-in must not add an input");
	}

	#[test]
	fn soft_subs_add_an_input_and_map_both_streams() {
		let p = preset(
			"YouTube 1080p HD", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(8000), Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			subtitle_mode: SubtitleMode::Soft,
			subtitle_path: Some(PathBuf::from("/subs/show.srt")),
			..Default::default()
		});
		assert_eq!(args.iter().filter(|a| *a == "-i").count(), 2);
		assert!(args.windows(2).any(|w| w == ["-c:s", "mov_text"]));
		assert!(args.windows(2).any(|w| w == ["-map", "0"]));
		assert!(args.windows(2).any(|w| w == ["-map", "1"]));
		assert!(!args.iter().any(|a| a.contains("subtitles=")), "soft mux must not burn");
	}

	#[test]
	fn soft_subs_pick_the_container_codec() {
		assert_eq!(subtitle_codec_for(&Container::Mp4), Some("mov_text"));
		assert_eq!(subtitle_codec_for(&Container::Mov), Some("mov_text"));
		assert_eq!(subtitle_codec_for(&Container::Mkv), Some("srt"));
		assert_eq!(subtitle_codec_for(&Container::Webm), Some("srt"));
		assert_eq!(subtitle_codec_for(&Container::Mp3), None);
		assert_eq!(subtitle_codec_for(&Container::Png), None);
	}

	#[test]
	fn soft_subs_skipped_for_containers_without_subtitle_support() {
		let p = preset(
			"MP3", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			subtitle_mode: SubtitleMode::Soft,
			subtitle_path: Some(PathBuf::from("/subs/show.srt")),
			..Default::default()
		});
		assert_eq!(args.iter().filter(|a| *a == "-i").count(), 1, "no second input for a container that cannot carry subs");
		assert!(!args.iter().any(|a| a == "-c:s"));
	}

	#[test]
	fn subtitles_absent_when_no_file_is_chosen() {
		let p = preset(
			"YouTube 1080p HD", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(8000), Some(320), None, None, None,
		);
		let (i, o) = in_out();
		for mode in [SubtitleMode::Burn, SubtitleMode::Soft] {
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
				subtitle_mode: mode,
				subtitle_path: None,
				..Default::default()
			});
			assert_eq!(args.iter().filter(|a| *a == "-i").count(), 1);
			assert!(!args.iter().any(|a| a.contains("subtitles=") || a == "-c:s"));
		}
	}

	#[test]
	fn filter_path_escaping_protects_colons() {
		let escaped = escape_filter_path(Path::new("C:/subs/show.srt"));
		assert!(escaped.contains("\\:"), "windows drive colons must be escaped: {}", escaped);
		assert!(!escaped.contains('\\') || escaped.contains("\\:"));
	}

	#[test]
	fn two_pass_first_pass_discards_output() {
		let p = preset(
			"YouTube 1080p HD", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(8000), Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			pass: Some(1),
			pass_log: Some("/tmp/lomux_pass_0".into()),
			..Default::default()
		});
		assert!(args.windows(2).any(|w| w == ["-pass", "1"]));
		assert!(args.windows(2).any(|w| w == ["-passlogfile", "/tmp/lomux_pass_0"]));
		assert!(args.windows(2).any(|w| w == ["-f", "null"]));
		assert!(args.iter().any(|a| a == "-an"));
		assert_eq!(args[args.len() - 1], "-");
	}

	#[test]
	fn two_pass_second_pass_writes_the_real_output() {
		let p = preset(
			"YouTube 1080p HD", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(8000), Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			pass: Some(2),
			pass_log: Some("/tmp/lomux_pass_0".into()),
			..Default::default()
		});
		assert!(args.windows(2).any(|w| w == ["-pass", "2"]));
		assert!(!args.windows(2).any(|w| w == ["-f", "null"]));
		assert_eq!(args[args.len() - 1], "/out.mp4");
		assert_eq!(args[args.len() - 2], "-y");
	}

	#[test]
	fn two_pass_only_offered_for_bitrate_video_presets() {
		let bitrate = preset(
			"H264 bitrate", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(8000), Some(320), None, None, None,
		);
		assert!(supports_two_pass(&bitrate));

		let crf = preset(
			"H264 crf", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			None, Some(320), Some(23), None, None,
		);
		assert!(!supports_two_pass(&crf), "CRF is already single-pass quality-targeted");

		let audio = preset(
			"MP3", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(320), None, None, None,
		);
		assert!(!supports_two_pass(&audio));

		let prores = preset(
			"ProRes", Container::Mov,
			VideoCodec::ProRes, AudioCodec::Pcm,
			Some(100000), None, None, None, None,
		);
		assert!(!supports_two_pass(&prores), "intra codecs gain nothing from two passes");
	}

	#[test]
	fn aiff_uses_big_endian_pcm_and_ac3_carries_bitrate() {
		let (i, o) = in_out();

		let aiff = preset(
			"AIFF", Container::Aiff,
			VideoCodec::None, AudioCodec::Pcm,
			None, None, None, None, None,
		);
		let args = build_ffmpeg_args(&aiff, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-c:a", "pcm_s16be"]));

		let wav = preset(
			"WAV", Container::Wav,
			VideoCodec::None, AudioCodec::Pcm,
			None, None, None, None, None,
		);
		let args = build_ffmpeg_args(&wav, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-c:a", "pcm_s16le"]));

		let ac3 = preset(
			"AC-3", Container::Ac3,
			VideoCodec::None, AudioCodec::Ac3,
			None, Some(448), None, None, None,
		);
		let args = build_ffmpeg_args(&ac3, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-c:a", "ac3"]));
		assert!(args.windows(2).any(|w| w == ["-b:a", "448k"]));
	}

	#[test]
	fn timecode_parses_all_supported_shapes() {
		assert_eq!(parse_timecode("12"), Some(12.0));
		assert_eq!(parse_timecode("1:30"), Some(90.0));
		assert_eq!(parse_timecode("01:02:03"), Some(3723.0));
		assert_eq!(parse_timecode(" 2.5 "), Some(2.5));
		assert_eq!(parse_timecode(""), None);
		assert_eq!(parse_timecode("abc"), None);
		assert_eq!(parse_timecode("-5"), None);
		assert_eq!(parse_timecode("1:2:3:4"), None);
	}

	#[test]
	fn timecode_formats_for_ffmpeg() {
		assert_eq!(format_timecode(0.0), "00:00:00.000");
		assert_eq!(format_timecode(90.5), "00:01:30.500");
		assert_eq!(format_timecode(3723.0), "01:02:03.000");
	}

	#[test]
	fn trim_emits_seek_before_input_and_duration_after() {
		let p = preset(
			"YouTube 1080p HD", Container::Mp4,
			VideoCodec::H264, AudioCodec::Aac,
			Some(8000), Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			trim_start: Some("00:00:10".into()),
			trim_end: Some("00:00:25".into()),
			..Default::default()
		});

		let ss = args.iter().position(|a| a == "-ss").expect("no -ss");
		let input = args.iter().position(|a| a == "-i").expect("no -i");
		let t = args.iter().position(|a| a == "-t").expect("no -t");
		assert!(ss < input, "-ss must precede -i for fast seek");
		assert!(t > input, "-t must follow -i");
		assert_eq!(args[ss + 1], "00:00:10.000");
		assert_eq!(args[t + 1], "00:00:15.000");
	}

	#[test]
	fn trim_end_alone_becomes_plain_duration() {
		let p = preset(
			"MP3", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			trim_end: Some("30".into()),
			..Default::default()
		});
		assert!(!args.iter().any(|a| a == "-ss"));
		let t = args.iter().position(|a| a == "-t").expect("no -t");
		assert_eq!(args[t + 1], "00:00:30.000");
	}

	#[test]
	fn trim_ignores_invalid_or_inverted_ranges() {
		let p = preset(
			"MP3", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(320), None, None, None,
		);
		let (i, o) = in_out();

		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			trim_start: Some("nonsense".into()),
			trim_end: Some("also bad".into()),
			..Default::default()
		});
		assert!(!args.iter().any(|a| a == "-ss" || a == "-t"));

		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions {
			trim_start: Some("30".into()),
			trim_end: Some("10".into()),
			..Default::default()
		});
		assert!(!args.iter().any(|a| a == "-t"), "inverted range must not emit a negative duration");
	}

	#[test]
	fn trim_problem_reports_bad_input() {
		assert_eq!(trim_problem("", ""), None);
		assert_eq!(trim_problem("10", "20"), None);
		assert!(trim_problem("x", "").is_some());
		assert!(trim_problem("", "y").is_some());
		assert!(trim_problem("20", "10").is_some());
	}

	#[test]
	fn trimmed_duration_tracks_the_selected_range() {
		let full = 120.0;
		assert_eq!(trimmed_duration(full, &EncodeOptions::default()), 120.0);
		assert_eq!(trimmed_duration(full, &EncodeOptions {
			trim_start: Some("00:00:30".into()),
			trim_end: Some("00:01:30".into()),
			..Default::default()
		}), 60.0);
		assert_eq!(trimmed_duration(full, &EncodeOptions {
			trim_start: Some("30".into()),
			..Default::default()
		}), 90.0);
		assert_eq!(trimmed_duration(0.0, &EncodeOptions {
			trim_start: Some("30".into()),
			..Default::default()
		}), 0.0);
	}

	#[test]
	fn youtube_prefix_stripped_from_output_name() {
		assert_eq!(strip_youtube_prefix("yt_1755734400_Some Song Title"), "Some Song Title");
		assert_eq!(strip_youtube_prefix("yt_1_A"), "A");
		assert_eq!(strip_youtube_prefix("yt_1755734400_Artist - Track_remix"), "Artist - Track_remix");
	}

	#[test]
	fn youtube_prefix_strip_leaves_other_names_alone() {
		assert_eq!(strip_youtube_prefix("holiday_video"), "holiday_video");
		assert_eq!(strip_youtube_prefix("yt_notatimestamp_clip"), "yt_notatimestamp_clip");
		assert_eq!(strip_youtube_prefix("yt_12345"), "yt_12345");
		assert_eq!(strip_youtube_prefix("yt_12345_"), "yt_12345_");
		assert_eq!(strip_youtube_prefix("my yt_123_file"), "my yt_123_file");
	}

	#[test]
	fn config_round_trips_active_and_custom_themes() {
		let mut custom = AppTheme::studio_dark();
		custom.name = CUSTOM_THEME.into();
		custom.accent = egui::Color32::from_rgb(200, 40, 90);
		custom.rounding = 11.0;

		let config = AppConfig {
			active: ThemeConfig::from(&AppTheme::emerald()),
			custom: Some(ThemeConfig::from(&custom)),
		};
		let json = serde_json::to_string(&config).expect("serialize");
		let parsed: AppConfig = serde_json::from_str(&json).expect("parse");

		assert_eq!(parsed.active.name, "Emerald");
		let restored = AppTheme::from(parsed.custom.expect("custom slot missing"));
		assert_eq!(restored.name, CUSTOM_THEME);
		assert_eq!(restored.accent, egui::Color32::from_rgb(200, 40, 90));
		assert_eq!(restored.rounding, 11.0);
	}

	#[test]
	fn old_config_files_without_a_custom_slot_still_load() {
		let legacy = serde_json::to_string(&ThemeConfig::from(&AppTheme::midnight())).expect("serialize");
		let parsed: AppConfig = serde_json::from_str(&legacy).expect("legacy config must still parse");
		assert_eq!(parsed.active.name, "Midnight");
		assert!(parsed.custom.is_none());
	}

	#[test]
	fn single_image_presets_write_one_frame() {
		let (i, o) = in_out();
		let singles: Vec<_> = EncodingPreset::get_all_presets()
			.into_iter()
			.filter(|p| p.single_image)
			.collect();
		assert!(!singles.is_empty(), "no single-image presets found");

		for p in singles {
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
			assert!(args.windows(2).any(|w| w == ["-frames:v", "1"]), "{} should write one frame", p.name);
			assert!(args.windows(2).any(|w| w == ["-update", "1"]), "{} needs -update for a fixed filename", p.name);
			assert!(args.iter().any(|a| a == "-an"), "{} should drop audio", p.name);
			let encoder = p.container.image_encoder().expect("no image encoder");
			assert!(args.windows(2).any(|w| w == ["-c:v", encoder]), "{} missing encoder", p.name);
		}
	}

	#[test]
	fn sequence_presets_do_not_limit_frames() {
		let (i, o) = in_out();
		for p in EncodingPreset::get_all_presets() {
			if !p.container.is_image_sequence() || p.single_image {
				continue;
			}
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
			assert!(!args.iter().any(|a| a == "-frames:v"), "{} must keep every frame", p.name);
		}
	}

	#[test]
	fn single_image_scaling_presets_keep_aspect() {
		let p = EncodingPreset::get_all_presets()
			.into_iter()
			.find(|p| p.name == "Web Thumbnail 640px")
			.expect("preset missing");
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		let filter = args.iter()
			.position(|a| a == "-vf")
			.map(|idx| args[idx + 1].clone())
			.expect("no -vf");
		assert_eq!(filter, "scale=640:-1");
	}

	#[test]
	fn image_sequence_presets_emit_frame_args() {
		let (i, o) = in_out();
		for p in EncodingPreset::get_all_presets() {
			if !p.container.is_image_sequence() {
				continue;
			}
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
			let encoder = p.container.image_encoder().expect("no encoder");
			assert!(args.windows(2).any(|w| w == ["-c:v", encoder]), "{} missing encoder", p.name);
			assert!(args.iter().any(|a| a == "-an"), "{} should drop audio", p.name);
			assert!(!args.iter().any(|a| a == "-vn"), "{} must not strip video", p.name);
			assert!(!args.iter().any(|a| a == "-filter_complex"), "{} should not use the gif chain", p.name);
		}
	}

	#[test]
	fn jpeg_sequence_sets_quality_and_tiff_is_raw_rgb() {
		let (i, o) = in_out();

		let jpg = preset(
			"JPEG Sequence", Container::Jpg,
			VideoCodec::None, AudioCodec::None,
			None, None, None, None, None,
		);
		let args = build_ffmpeg_args(&jpg, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-q:v", "2"]));

		let tiff = preset(
			"TIFF Sequence", Container::Tiff,
			VideoCodec::None, AudioCodec::None,
			None, None, None, None, None,
		);
		let args = build_ffmpeg_args(&tiff, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(args.windows(2).any(|w| w == ["-pix_fmt", "rgb24"]));
		assert!(args.windows(2).any(|w| w == ["-compression_algo", "raw"]));
	}

	#[test]
	fn image_sequence_applies_fps_and_scale_filters() {
		let p = preset(
			"PNG Sequence", Container::Png,
			VideoCodec::None, AudioCodec::None,
			None, None, None, Some(24), Some("1280x720"),
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		let filter = args.iter()
			.position(|a| a == "-vf")
			.map(|idx| args[idx + 1].clone())
			.expect("no -vf");
		assert_eq!(filter, "fps=24,scale=1280:720");
	}

	#[test]
	fn preset_json_round_trips() {
		let original = EncodingPreset::get_all_presets()
			.into_iter()
			.find(|p| p.name == "YouTube 1080p HD")
			.expect("preset missing");
		let json = serde_json::to_string_pretty(&original).expect("serialize");
		let parsed = parse_presets_json(&json).expect("parse");
		assert_eq!(parsed.len(), 1);
		assert_eq!(parsed[0].name, original.name);
		assert_eq!(parsed[0].container, original.container);
		assert_eq!(parsed[0].video_codec, original.video_codec);
		assert_eq!(parsed[0].audio_codec, original.audio_codec);
		assert_eq!(parsed[0].video_bitrate, original.video_bitrate);
		assert_eq!(parsed[0].resolution, original.resolution);

		let (i, o) = in_out();
		assert_eq!(
			build_ffmpeg_args(&parsed[0], &i, &o, &AudioMetadata::default(), &EncodeOptions::default()),
			build_ffmpeg_args(&original, &i, &o, &AudioMetadata::default(), &EncodeOptions::default()),
		);
	}

	#[test]
	fn preset_json_accepts_arrays_and_single_objects() {
		let all = EncodingPreset::get_all_presets();
		let json = serde_json::to_string(&all).expect("serialize");
		assert_eq!(parse_presets_json(&json).expect("array").len(), all.len());

		let single = serde_json::to_string(&all[0]).expect("serialize");
		assert_eq!(parse_presets_json(&single).expect("single").len(), 1);
	}

	#[test]
	fn preset_json_tolerates_missing_description() {
		let json = r#"{
			"name": "Minimal",
			"category": "Custom",
			"container": "Mp4",
			"video_codec": "H264",
			"audio_codec": "Aac",
			"video_bitrate": 5000,
			"audio_bitrate": 192,
			"video_crf": null,
			"fps": null,
			"resolution": null
		}"#;
		let parsed = parse_presets_json(json).expect("parse");
		assert_eq!(parsed[0].name, "Minimal");
		assert_eq!(parsed[0].description, "");
	}

	#[test]
	fn preset_json_rejects_garbage() {
		assert!(parse_presets_json("not json").is_none());
		assert!(parse_presets_json("{\"name\": \"broken\"}").is_none());
	}

	#[test]
	fn template_substitutes_metadata_tokens() {
		let out = apply_filename_template(
			"{artist} - {title}", "original", &meta("Band", "Song"), "MP3 320kbps", "mp3", 0,
		);
		assert_eq!(out, "Band - Song");
	}

	#[test]
	fn template_supports_name_preset_index_and_ext() {
		let out = apply_filename_template(
			"{index}. {name} [{preset}].{ext}", "clip", &AudioMetadata::default(), "YouTube 1080p HD", "mp4", 4,
		);
		assert_eq!(out, "5. clip [YouTube 1080p HD]");
	}

	#[test]
	fn template_strips_path_separators_and_illegal_chars() {
		let out = apply_filename_template(
			"{artist}/{title}", "original", &meta("AC/DC", "Back: In*Black?"), "MP3", "mp3", 0,
		);
		assert!(!out.contains('/'));
		assert!(!out.contains(':'));
		assert!(!out.contains('*'));
		assert!(!out.contains('?'));
	}

	#[test]
	fn template_falls_back_to_stem_when_result_is_empty() {
		let out = apply_filename_template(
			"{artist} {title}", "original", &AudioMetadata::default(), "MP3", "mp3", 0,
		);
		assert_eq!(out, "original");
	}

	#[test]
	fn template_keeps_unicode_intact() {
		let out = apply_filename_template(
			"{artist} - {title}", "original", &meta("Café Münster", "Übersetzung 🎵"), "MP3", "mp3", 0,
		);
		assert_eq!(out, "Café Münster - Übersetzung 🎵");
	}

	#[test]
	fn loudnorm_applied_when_enabled() {
		let p = preset(
			"MP3", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions { normalize_audio: true, ..Default::default() });
		assert!(args.windows(2).any(|w| w == ["-af", LOUDNORM_FILTER]));
	}

	#[test]
	fn loudnorm_absent_when_disabled() {
		let p = preset(
			"MP3", Container::Mp3,
			VideoCodec::None, AudioCodec::Mp3,
			None, Some(320), None, None, None,
		);
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		assert!(!args.iter().any(|a| a == "-af"));
	}

	#[test]
	fn loudnorm_skipped_for_copied_or_absent_audio() {
		let (i, o) = in_out();

		let remux = preset(
			"Remux", Container::Mp4,
			VideoCodec::Copy, AudioCodec::Copy,
			None, None, None, None, None,
		);
		let args = build_ffmpeg_args(&remux, &i, &o, &AudioMetadata::default(), &EncodeOptions { normalize_audio: true, ..Default::default() });
		assert!(!args.iter().any(|a| a == "-af"), "loudnorm cannot run on a copied stream");

		let silent = preset(
			"Silent", Container::Mp4,
			VideoCodec::H264, AudioCodec::None,
			Some(5000), None, None, None, None,
		);
		let args = build_ffmpeg_args(&silent, &i, &o, &AudioMetadata::default(), &EncodeOptions { normalize_audio: true, ..Default::default() });
		assert!(!args.iter().any(|a| a == "-af"));
	}

	#[test]
	fn loudnorm_never_reaches_gif() {
		let p = EncodingPreset::get_all_presets()
			.into_iter()
			.find(|p| p.name == "Web GIF")
			.expect("Web GIF preset missing");
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions { normalize_audio: true, ..Default::default() });
		assert!(!args.iter().any(|a| a == "-af"));
	}

	#[test]
	fn download_percent_parses_ytdlp_lines() {
		let cases = [
			("[download]   0.0% of 12.34MiB at 1.00MiB/s ETA 00:12", 0.0),
			("[download]  42.5% of ~ 98.76MiB at  5.43MiB/s ETA 00:09", 0.425),
			("[download] 100% of 12.34MiB in 00:03", 1.0),
			("[download] 100.0% of 12.34MiB", 1.0),
		];
		for (line, expected) in cases {
			let got = parse_download_percent(line).expect(line);
			assert!((got - expected).abs() < 0.001, "{} gave {}", line, got);
		}
	}

	#[test]
	fn download_percent_ignores_other_lines() {
		assert_eq!(parse_download_percent("[download] Destination: /tmp/yt_1_video.mp4"), None);
		assert_eq!(parse_download_percent("[youtube] Extracting URL"), None);
		assert_eq!(parse_download_percent(""), None);
	}

	#[test]
	fn download_percent_clamps_out_of_range() {
		assert_eq!(parse_download_percent("[download] 250.0% of 1MiB"), Some(1.0));
	}

	#[test]
	fn gif_preset_uses_palette_filter() {
		let p = EncodingPreset::get_all_presets()
			.into_iter()
			.find(|p| p.name == "Web GIF")
			.expect("Web GIF preset missing");
		let (i, o) = in_out();
		let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
		let filter = args.iter()
			.position(|a| a == "-filter_complex")
			.map(|idx| args[idx + 1].clone())
			.expect("no -filter_complex");
		assert!(filter.contains("palettegen"));
		assert!(filter.contains("paletteuse"));
		assert!(filter.contains("scale=480:-1:flags=lanczos"));
		assert!(filter.starts_with("fps=15,"));
		assert!(args.windows(2).any(|w| w == ["-loop", "0"]));
		assert!(args.iter().any(|a| a == "-an"));
		assert!(!args.iter().any(|a| a == "-vn"));
	}

	#[test]
	fn no_gif_preset_strips_its_own_video() {
		let (i, o) = in_out();
		for p in EncodingPreset::get_all_presets() {
			if p.container != Container::Gif {
				continue;
			}
			let args = build_ffmpeg_args(&p, &i, &o, &AudioMetadata::default(), &EncodeOptions::default());
			assert!(!args.iter().any(|a| a == "-vn"), "preset {} strips video from a GIF", p.name);
			assert!(args.iter().any(|a| a == "-filter_complex"), "preset {} has no gif filter", p.name);
		}
	}

	#[test]
	fn truncate_str_handles_multibyte() {
		let name = "Café Münster – Übersetzung 🎵.mp3";
		for max in [1usize, 4, 7, 12, 20] {
			let out = truncate_str(name, max);
			assert!(out.chars().count() <= max, "max {} produced {} chars", max, out.chars().count());
		}
	}

	#[test]
	fn truncate_str_leaves_short_strings_alone() {
		assert_eq!(truncate_str("short.mp4", 45), "short.mp4");
		assert_eq!(truncate_str("Übersetzung 🎵.mp3", 45), "Übersetzung 🎵.mp3");
	}

	#[test]
	fn truncate_path_handles_multibyte() {
		let path = PathBuf::from("/Users/zachary/Música/Café Münster – Übersetzung 🎵.mp3");
		for max in [5usize, 10, 30, 50] {
			let out = truncate_path(&path, max);
			assert!(!out.is_empty());
		}
	}
}
