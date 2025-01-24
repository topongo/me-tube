use rocket::form::Form;
use serde::{Serialize, Deserialize};

use crate::db::DBWrapper;

#[derive(Serialize, Deserialize)]
enum AudioCodec {
    Mp3,
    Aac,
    Unk,
}

#[derive(Serialize, Deserialize)]
enum VideoCodec {
    H264,
    H265,
    Unk,
}

#[derive(Serialize, Deserialize)]
enum Format {
    Mkv,
    Mp4,
    Unk,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Video {
    id: String,
    duration: f32,
    name: Option<String>,
    audio_codec: AudioCodec,
    video_codec: VideoCodec,
    format: Format,
    parent: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct VideoUpload {
    id: String,
    metadata: Option<Video>
}

impl DBWrapper {
    // pub(crate) async fn add_video()
}

// #[post("/upload", data = "<video>")]
// async fn upload(video: Form<VideoUploadForm>, user: UserGuard)
