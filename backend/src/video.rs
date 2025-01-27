use rocket::{form::Form, fs::TempFile};
use rocket_db_pools::Connection;
use serde::{Serialize, Deserialize};

use crate::{authentication::{AuthenticationError, UserGuard}, config::CONFIG, db::{DBWrapper, Db}, response::{ApiResponder, ApiResponse}};

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

#[derive(Serialize, Deserialize)]
struct VideoUploadResponse;

impl ApiResponse for VideoUploadResponse {}

#[derive(FromForm, Debug)]
struct VideoUploadForm<'r> {
    file: TempFile<'r>,
}

#[post("/upload", data = "<form>")]
pub(crate) async fn upload(mut form: Form<VideoUploadForm<'_>>, user: Result<UserGuard, AuthenticationError>, db: Connection<Db>) -> ApiResponder<VideoUploadResponse> {
    let user = user?;
    let db = DBWrapper::new(db.into_inner());
    // TODO: check if user has permission to upload
    //       generate random code: https://github.com/topongo/movieStore/blob/master/video_share/models.py#L25
    //       check if code isn't clashing
    //       get video metadata
    //       insert in db
    //       move file to storage only if everything is successful
    form.file.persist_to(CONFIG.video_storage.clone()).await;

    todo!()
}

// #[post("/upload", data = "<video>")]
// async fn upload(video: Form<VideoUploadForm>, user: UserGuard)
