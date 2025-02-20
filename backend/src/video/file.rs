use std::fmt::Display;
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use rocket_db_pools::mongodb::bson::{doc, oid::ObjectId};
use serde::{Serialize, Deserialize};

use crate::config::CONFIG;

use super::UploadError;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(super) enum AudioCodec {
    Mp3,
    Aac,
    #[serde(untagged)]
    Unk(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(super) enum VideoCodec {
    H264,
    Hevc,
    #[serde(untagged)]
    Unk(String),
}

impl From<&str> for AudioCodec {
    fn from(s: &str) -> Self {
        match s {
            "mp3" => Self::Mp3,
            "aac" => Self::Aac,
            _ => Self::Unk(s.to_string()),
        }
    }
}

impl From<&str> for VideoCodec {
    fn from(s: &str) -> Self {
        match s {
            "h264" => Self::H264,
            "hevc" => Self::Hevc,
            _ => Self::Unk(s.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(super) enum Format {
    Mkv,
    Mp4,
    Unk(String),
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mkv => write!(f, "mkv"),
            Self::Mp4 => write!(f, "mp4"),
            Self::Unk(s) => write!(f, "{}", s),
        }
    }
}

impl From<&str> for Format {
    fn from(s: &str) -> Self {
        match s {
            "matroska,webm" => Self::Mkv,
            "mov,mp4,m4a,3gp,3g2,mj2" => Self::Mp4,
            _ => Self::Unk(s.to_string()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
enum CodecType {
    Video,
    Audio,
    Subtitle,
    Data,
}

#[derive(Deserialize, Serialize, Debug)]
struct ProbedStream {
    index: u32,
    codec_name: String,
    profile: Option<String>,
    codec_type: CodecType,
    #[serde(rename = "codec_tag_string")]
    codec_tag: String,
    // #[serde(deserialize_with = "deserialize_string_float")]
    duration: Option<String>,
    // save all tags into a hashmap
    #[serde(flatten)]
    tags: Option<HashMap<String, serde_json::Value>>
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ProbedFormat {
    format_name: String,
    #[serde(deserialize_with = "deserialize_string_usize")]
    size: usize,
    // #[serde(deserialize_with = "deserialize_string_float")]
    duration: Option<String>,
}

fn deserialize_string_usize<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    Ok(s.parse().unwrap())
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct VideoFile {
    #[serde(rename = "_id")]
    pub id: String,
    duration: Option<f64>,
    size: Option<usize>,
    audio_codec: AudioCodec,
    video_codec: VideoCodec,
    pub(super) format: Format,
    converted: Option<String>,
}


impl VideoFile {
    pub(super) async fn from_path(path: &Path) -> Result<VideoFile, UploadError> {
        println!("{:?}", path);
        let proc = std::process::Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-show_streams")
            .arg("-show_format")
            .arg("-of")
            .arg("json")
            .arg(path)
            .output()
            .map_err(|_| UploadError::ProbeError("ffprobe process"))?;
        if proc.status.success() {
            let probed = String::from_utf8(proc.stdout)
                .map_err(|_| UploadError::ProbeError("ffprobe output format"))?;
            println!("{}", probed);

            #[derive(Deserialize)]
            struct Probed {
                streams: Vec<ProbedStream>,
                format: ProbedFormat,
            }
            let probed: Probed = serde_json::from_str(&probed).map_err(|e| { log::error!("error while probing video: {:?}", e); UploadError::ProbeError("deserializing ffprobe output") })?;
            let streams = probed.streams;
            // this purposelly keeps the first audio and video stream
            let a_stream = streams.iter()
                .find(|s| matches!(s.codec_type, CodecType::Audio))
                .map(|s| AudioCodec::from(s.codec_name.as_str()));
            let v_stream = streams.iter()
                .find(|s| matches!(s.codec_type, CodecType::Video))
                .map(|s| VideoCodec::from(s.codec_name.as_str()));

            if a_stream.is_none() && v_stream.is_none() {
                return Err(UploadError::FormatError("uploaded file doens't contain audio either video nor audio streams"));
            }

            let fduration = probed.format.duration.as_ref().map(|v| v.parse::<f64>().unwrap());
            let duration = streams.iter()
                .map(|s| s.duration
                    .as_ref()
                    .map(|v| v.parse::<f64>().unwrap())
                    .or(s.tags.as_ref().and_then(|t| t.get("DURATION").and_then(|v| v.as_f64()))))
                .fold(fduration, |acc, d| {
                    match (acc, d) {
                        (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) },
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                });

            let id = ObjectId::new().to_hex();
            // if there is video then create thumbnail
            if v_stream.is_some() {
                // get video stream length
                let vlength = streams.iter()
                    .find(|s| matches!(s.codec_type, CodecType::Video))
                    .and_then(|s| s.duration.as_ref().map(|v| v.parse::<f64>().unwrap()))
                    .unwrap_or(1.);

                let proc = std::process::Command::new("ffmpeg")
                    .arg("-y")
                    .arg("-ss")
                    .arg((vlength * 0.2).to_string())
                    .arg("-i")
                    .arg(path)
                    .arg("-frames:v")
                    .arg("1")
                    .arg(Path::new(&CONFIG.video_storage).join("thumbs").join(format!("{}.jpg", id)))
                    .output();
                match proc {
                    Err(e) => log::error!("failed to create thumbnail for video {}: {}", id, e),
                    Ok(proc) => {
                        if !proc.status.success() {
                            log::error!("failed to create thumbnail for video {}: {}", id, String::from_utf8_lossy(&proc.stderr));
                        }
                    }
                }
            }

            Ok(VideoFile {
                id,
                duration,
                size: Some(probed.format.size),
                audio_codec: a_stream.unwrap_or(AudioCodec::Unk("unknown".to_string())),
                video_codec: v_stream.unwrap_or(VideoCodec::Unk("unknown".to_string())),
                format: Format::from(probed.format.format_name.as_str()),
                converted: None,
            })
        } else {
            let err = String::from_utf8(proc.stderr)
                .expect("format error on ffprobe output");
            println!("{}", err);
            Err(UploadError::ProbeError("ffprobe process"))
        }
    }

    pub(crate) fn path(&self) -> PathBuf {
        PathBuf::from(&CONFIG.video_storage).join(&self.id)
    }

    pub(crate) fn thumb(id: &str) -> Option<PathBuf> {
        let thumb = Path::new(&CONFIG.video_storage).join("thumbs").join(format!("{}.jpg", id));
        if thumb.exists() {
            Some(thumb)
        } else {
            None
        }
    }

    pub(crate) fn delete(&self) -> Result<(), std::io::Error> {
        std::fs::remove_file(self.path())
    }
}
