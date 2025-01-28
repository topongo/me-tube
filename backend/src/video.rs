use std::{cmp::max_by, collections::HashMap, path::Path};

use rand::Rng;
use rocket::futures::{TryStreamExt, StreamExt};
use rocket::{form::Form, fs::TempFile};
use rocket_db_pools::mongodb;
use rocket_db_pools::{mongodb::bson::{doc, oid::ObjectId}, Connection};
use serde::{Serialize, Deserialize};

use crate::{authentication::{AuthenticationError, UserGuard}, config::CONFIG, db::{DBWrapper, Db}, response::{ApiErrorType, ApiResponder, ApiResponse}, user::Permissions};

#[derive(Serialize, Deserialize, Debug)]
enum AudioCodec {
    Mp3,
    Aac,
    Unk(String),
}

#[derive(Serialize, Deserialize, Debug)]
enum VideoCodec {
    H264,
    H265,
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
            "h265" => Self::H265,
            _ => Self::Unk(s.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Format {
    Mkv,
    Mp4,
    Unk(String),
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Video {
    #[serde(rename = "_id")]
    id: String,
    file: Either<String, VideoFile>,
    name: Option<String>,
    game: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct VideoFile {
    #[serde(rename = "_id")]
    id: String,
    duration: Option<f64>,
    size: usize,
    audio_codec: AudioCodec,
    video_codec: VideoCodec,
    format: Format,
    parent: Option<String>,
}

static CODE_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

impl Video {
    fn random_code() -> String {
        let mut rng = rand::thread_rng();
        // get 6 random chars from CODE_CHARS
        let code: String = (0..6).map(|_| {
            let idx = rng.gen_range(0..CODE_CHARS.len());
            CODE_CHARS[idx] as char
        }).collect();
        code
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
enum CodecType {
    Video,
    Audio,
}

fn deserialize_string_float<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => Ok(Some(s.parse().unwrap())),
        None => Ok(None),
    }
}

fn deserialize_string_usize<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    Ok(s.parse().unwrap())
}

#[derive(Deserialize, Serialize, Debug)]
struct ProbedStream {
    index: u32,
    codec_name: String,
    profile: String,
    codec_type: CodecType,
    #[serde(rename = "codec_tag_string")]
    codec_tag: String,
    #[serde(deserialize_with = "deserialize_string_float")]
    duration: Option<f64>,
    // save all tags into a hashmap
    #[serde(flatten)]
    tags: Option<HashMap<String, serde_json::Value>>
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ProbedFormat {
    format_name: String,
    #[serde(deserialize_with = "deserialize_string_usize")]
    size: usize,
    #[serde(deserialize_with = "deserialize_string_float")]
    duration: Option<f64>,
}

impl VideoFile {
    async fn from_path(path: &Path) -> Result<VideoFile, UploadError> {
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
            let probed: Probed = serde_json::from_str(&probed).map_err(|_| UploadError::ProbeError("deserializing ffprobe output"))?;
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

            let duration = streams.iter()
                .map(|s| s.duration.or(s.tags.as_ref().and_then(|t| t.get("DURATION").and_then(|v| v.as_f64()))))
                .fold(probed.format.duration, |acc, d| {
                    match (acc, d) {
                        (Some(a), Some(b)) => if a > b { Some(a) } else { Some(b) },
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                });

            Ok(VideoFile {
                id: ObjectId::new().to_hex(),
                duration,
                size: probed.format.size,
                audio_codec: a_stream.unwrap_or(AudioCodec::Unk("unknown".to_string())),
                video_codec: v_stream.unwrap_or(VideoCodec::Unk("unknown".to_string())),
                format: Format::from(probed.format.format_name.as_str()),
                parent: None,
            })
        } else {
            let err = String::from_utf8(proc.stderr)
                .expect("format error on ffprobe output");
            println!("{}", err);
            Err(UploadError::ProbeError("ffprobe process"))
        }
    }
}

impl DBWrapper {
    pub(crate) async fn check_video_code(&self, code: &str) -> Result<bool, mongodb::error::Error> {
        Ok(self.database()
            .collection::<Video>("video_files")
            .find_one(doc! { "id": code }, None)
            .await?
            .is_none())
    }

    pub(crate) async fn insert_video_file(&self, video: VideoFile) -> Result<(), mongodb::error::Error> {
        self.database()
            .collection::<VideoFile>("video_files")
            .insert_one(video, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_video(&self, video: &Video) -> Result<(), mongodb::error::Error> {
        self.database()
            .collection::<Video>("videos")
            .insert_one(video, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_videos(&self, ids: Vec<String>) -> Result<Vec<Video>, mongodb::error::Error> {
        let d= if ids.is_empty() {
            doc! {}
        } else {
            doc! { "_id": { "$in": ids } }
        };
        self.database()
            .collection::<Video>("videos")
            .find(d, None)
            .await?
            .try_collect()
            .await
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct UploadResponse {
    inner: Video,
}

impl ApiResponse for UploadResponse {}

#[derive(FromForm, Debug)]
struct UploadForm<'r> {
    name: Option<String>,
    game: String,
    file: TempFile<'r>,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum UploadError { 
    GameNotFound,
    VideoAlreadyExists(String),
    ProbeError(&'static str),
    FormatError(&'static str),
}

impl ApiErrorType for UploadError {
    fn ty(&self) -> &'static str {
        match self {
            Self::GameNotFound => "game_not_found",
            Self::VideoAlreadyExists(_) => "video_already_exists",
            Self::ProbeError(_) => "probe_error",
            Self::FormatError(_) => "format_error",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            Self::GameNotFound => rocket::http::Status::NotFound,
            Self::VideoAlreadyExists(_) => rocket::http::Status::Conflict,
            Self::ProbeError(_) => rocket::http::Status::InternalServerError,
            Self::FormatError(_) => rocket::http::Status::BadRequest,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::GameNotFound => "Game not found".to_string(),
            Self::VideoAlreadyExists(_) => "Video already exists".to_string(),
            Self::ProbeError(s) => format!("Error while probing video for metadata: {}", s),
            Self::FormatError(s) => format!("Uploaded file has some format errors: {}", s),
        }
    }
}

#[post("/upload", data = "<form>")]
pub(crate) async fn upload(mut form: Form<UploadForm<'_>>, user: Result<UserGuard, AuthenticationError>, db: Connection<Db>) -> ApiResponder<UploadResponse> {
    let user = user?.user;
    let db = DBWrapper::new(db.into_inner());
    //       check if user has permission to upload
    if !user.allowed(Permissions::ADD_VIDEO) {
        return AuthenticationError::InsufficientPermissions.into();
    }
    //       check if game exists
    let games = db.get_games(vec![form.game.clone()]).await?;
    if games.is_empty() {
        return ApiResponder::Err(UploadError::GameNotFound.into());
    }

    // TODO: check if user has access to the game

    //       get video metadata
    let file = VideoFile::from_path(form.file.path().unwrap()).await?;
    println!("{:?}", file);

    //       generate random code: https://github.com/topongo/movieStore/blob/master/video_share/models.py#L25
    let mut code;
    //       check if code isn't clashing
    loop {
        code = Video::random_code();
        if db.check_video_code(&code).await? { break }
        println!("code clashes: {}", code);
    }
    //       insert video file in db
    let fid = file.id.clone();
    db.insert_video_file(file).await?;
    //       insert video in db
    let video = Video {
        id: code,
        file: Either::Left(fid),
        name: form.name.clone(),
        game: form.game.clone(),
    };

    db.insert_video(&video).await?;
    //       move file to storage only if everything is successful
    let target = Path::new(&CONFIG.video_storage).join(&video.id);
    form.file.move_copy_to(target).await.expect("Failed to move file to storage");

    UploadResponse { inner: video }.into()
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct GetResponse {
    inner: Vec<Video>,
}

impl ApiResponse for GetResponse {}

#[get("/")]
pub(crate) async fn list(user: Result<UserGuard, AuthenticationError>, db: Connection<Db>) -> ApiResponder<GetResponse> {
    let user = user?.user;
    let db = DBWrapper::new(db.into_inner());
    if !user.allowed(Permissions::VIEW_VIDEO) {
        return AuthenticationError::InsufficientPermissions.into();
    }

    // TODO: check if user has access to the game

    GetResponse { inner: db.get_videos(vec![]).await.unwrap() }.into()
}

