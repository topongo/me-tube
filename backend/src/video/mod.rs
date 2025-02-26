mod token;
mod file;
pub mod share;

use std::path::Path;

use chrono::{DateTime, Utc};
use file::VideoFile;
use rand::Rng;
use rocket::fs::NamedFile;
use rocket::futures::{TryStreamExt, StreamExt};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::{form::Form, fs::TempFile};
use rocket_db_pools::mongodb;
use rocket_db_pools::mongodb::bson::Document;
use rocket_db_pools::mongodb::bson::doc;
use serde::{Serialize, Deserialize};
use token::VideoToken;

use crate::user::{ExpiringToken, User};
use crate::{authentication::{AuthenticationError, UserGuard}, config::CONFIG, db::DBWrapper, response::{ApiErrorType, ApiResponder, ApiResponse}, user::Permissions};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

#[allow(dead_code)]
impl<L, R> Either<L, R> {
    pub fn unwrap_left(self) -> L {
        match self {
            Self::Left(l) => l,
            _ => panic!("called `Either::unwrap_left` on a `Right` value"),
        }
    }

    pub fn unwrap_right(self) -> R {
        match self {
            Self::Right(r) => r,
            _ => panic!("called `Either::unwrap_right` on a `Left` value"),
        }
    }

    pub fn as_ref(&self) -> Either<&L, &R> {
        match self {
            Self::Left(l) => Either::Left(l),
            Self::Right(r) => Either::Right(r),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Video {
    #[serde(rename = "_id")]
    pub id: String,
    pub file: Either<String, VideoFile>,
    name: Option<String>,
    game: String,
    public: bool,
    owner: String,
    added: DateTime<Utc>,
}

static CODE_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

impl Video {
    fn random_code() -> String {
        let mut rng = rand::thread_rng();
        // get 6 random chars from CODE_CHARS
        let code: String = (0..11).map(|_| {
            let idx = rng.gen_range(0..CODE_CHARS.len());
            CODE_CHARS[idx] as char
        }).collect();
        code
    }

    pub(crate) fn download_name(&self) -> String {
        let f = self.file.as_ref().unwrap_right();
        match self.name {
            Some(ref n) => format!("{}.{}", n, f.format),
            None => format!("{}.{}", self.id, f.format),
        }
    }

    pub(crate) fn generate_token(&self) -> VideoToken {
        VideoToken::new(&self.id)
    }

    pub(crate) async fn user_authorized(&self, user: Option<&User>, db: &DBWrapper) -> Result<bool, mongodb::error::Error> {
        Ok(self.public ||
            match user {
                Some(user) => user.allowed(Permissions::WATCH_VIDEO) || db.is_user_in_game(&self.game, &user.username).await?,
                None => false,
            }
        )
    }

    // fails if file is Either::Left
    pub(crate) async fn resolve_converted(&mut self, db: &DBWrapper) -> Result<(), mongodb::error::Error> {
        if let Some(conv) = self.file.as_ref().unwrap_right().converted.clone() {
            self.file = Either::Right(db.get_video_file(&conv).await?.unwrap());
        }
        Ok(())
    }
}

impl DBWrapper {
    pub(crate) async fn check_video_code(&self, code: &str) -> Result<bool, mongodb::error::Error> {
        Ok(self
            .collection::<Video>(Self::VIDEO_FILES)
            .find_one(doc! { "id": code }, None)
            .await?
            .is_none())
    }

    pub(super) async fn insert_video_file(&self, video: VideoFile) -> Result<(), mongodb::error::Error> {
        self
            .collection::<VideoFile>(Self::VIDEO_FILES)
            .insert_one(video, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_video(&self, video: &Video) -> Result<(), mongodb::error::Error> {
        self
            .collection::<Video>(Self::VIDEOS)
            .insert_one(video, None)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn get_videos(&self, ids: Vec<String>) -> Result<Vec<Video>, mongodb::error::Error> {
        let d= if ids.is_empty() {
            doc! {}
        } else {
            doc! { "_id": { "$in": ids } }
        };
        self
            .collection::<Video>(Self::VIDEOS)
            .find(d, None)
            .await?
            .try_collect()
            .await
    }

    pub(crate) async fn get_user_videos(&self, user: &User, sort: bool, skip: Option<u32>, limit: Option<u32>) -> Result<(usize, Vec<Video>), mongodb::error::Error> {
        let mut pipeline = vec![];
        let m = if user.allowed(Permissions::ADMIN) {
            doc! { }
        } else {
            let user_games = self.get_user_games_ids(user).await?;
            doc! { "$or": [{ "owner": &user.username }, { "public": true }, { "game": { "$in": user_games.into_iter().collect::<Vec<_>>() } }]}
        };
        pipeline.push(doc! {"$match": m.clone()});
        if sort {
            pipeline.push(doc! {"$sort": {"added": -1}});
        }
        if let Some(skip) = skip {
            pipeline.push(doc!{"$skip": skip as i64});
        }
        if let Some(limit) = limit {
            pipeline.push(doc!{"$limit": limit as i64});
        }
        let count = self
            .collection::<Document>(Self::VIDEOS)
            .count_documents(m, None)
            .await?;

        self
            .collection::<Document>(Self::VIDEOS)
            .aggregate(pipeline, None)
            .await?
            .map(|d| d.map(|d| mongodb::bson::from_document::<Video>(d).unwrap()))
            .try_collect()
            .await
            .map(|v| (count as usize, v))
    }

    pub(super) async fn get_video_files(&self, ids: Vec<String>) -> Result<Vec<VideoFile>, mongodb::error::Error> {
        let d= if ids.is_empty() {
            doc! {}
        } else {
            doc! { "_id": { "$in": ids } }
        };
        self
            .collection::<VideoFile>(Self::VIDEO_FILES)
            .find(d, None)
            .await?
            .try_collect()
            .await
    }

    pub(crate) async fn get_video_file(&self, id: &str) -> Result<Option<VideoFile>, mongodb::error::Error> {
        self
            .collection::<VideoFile>(Self::VIDEO_FILES)
            .find_one(doc! { "_id": id }, None)
            .await
    }

    pub(crate) async fn get_video(&self, id: &str) -> Result<Option<Video>, mongodb::error::Error> {
        self
            .collection::<Video>(Self::VIDEOS)
            .find_one(doc! { "_id": id }, None)
            .await
    }

    pub(crate) async fn get_video_resolved(&self, id: &str) -> Result<Option<Video>, mongodb::error::Error> {
        let res = self
            .collection::<Video>(Self::VIDEOS)
            .aggregate(vec![
                doc! { "$match": { "_id": id } },
                // join with video_files
                doc! { "$lookup": {
                    "from": Self::VIDEO_FILES,
                    "localField": "file",
                    "foreignField": "_id",
                    "as": "file"
                } },
                doc! { "$unwind": "$file" },
            ], None)
            .await?
            .map(|v| v.map(|v| mongodb::bson::from_document::<Video>(v).unwrap() ))
            .try_collect::<Vec<Video>>()
            .await?;
        Ok(res.into_iter().next())
    }

    pub(super) async fn delete_video(&self, video: &Video) -> Result<(), mongodb::error::Error> {
        self
            .collection::<Video>(Self::VIDEOS)
            .delete_one(doc! { "_id": &video.id }, None)
            .await?;
        // delete referenced video file
        self.delete_video_file(&video.file.as_ref().unwrap_right().id).await?;
        // delete referenced likes
        self
            .collection::<()>("likes")
            .delete_many(doc! { "video": &video.id }, None)
            .await?;
        Ok(())
    }

    pub(super) async fn delete_video_file(&self, id: &str) -> Result<(), mongodb::error::Error> {
        self
            .collection::<VideoFile>(Self::VIDEO_FILES)
            .delete_one(doc! { "_id": id }, None)
            .await?;
        Ok(())
    }

    pub(super) async fn update_video(&self, video: &Video) -> Result<(), mongodb::error::Error> {
        self
            .collection::<Video>(Self::VIDEOS)
            .replace_one(doc! { "_id": video.id.clone() }, video, None)
            .await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct UploadResponse {
    inner: Vec<Video>,
}

impl ApiResponse for UploadResponse {}

#[derive(FromForm, Debug)]
struct FileWrapper<'r> {
    name: Option<String>,
    public: bool,
    file: TempFile<'r>,
}

#[derive(FromForm, Debug)]
pub(crate) struct UploadForm<'r> {
    game: String,
    files: Vec<FileWrapper<'r>>,
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
pub(crate) async fn upload(mut form: Form<UploadForm<'_>>, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<UploadResponse> {
    let user = user?.user;
    // check if user has permission to upload
    if !user.allowed(Permissions::ADD_VIDEOS) {
        return AuthenticationError::InsufficientPermissions(Permissions::ADD_VIDEOS).into();
    }
    // check if game exists
    //    this also checks if user is in the game group
    let games = db.get_user_games_ids(&user).await?;
    if !games.contains(&form.game) {
        return ApiResponder::Err(UploadError::GameNotFound.into());
    }
    let game = form.game.clone();

    // get video metadata
    let mut videos = vec![];
    for file in form.files.iter_mut() {
        let vfile = VideoFile::from_path(file.file.path().unwrap()).await?;

        // generate random code: https://github.com/topongo/movieStore/blob/master/video_share/models.py#L25
        let mut code;
        // check if code isn't clashing
        loop {
            code = Video::random_code();
            if db.check_video_code(&code).await? { break }
            log::warn!("code clashes: {}", code);
        }
        // insert video file in db
        let fid = vfile.id.clone();
        db.insert_video_file(vfile).await?;
        // insert video in db
        let video = Video {
            id: code,
            file: Either::Left(fid.clone()),
            name: file.name.clone(),
            game: game.clone(),
            public: file.public,
            owner: user.username.clone(),
            added: Utc::now(),
        };

        db.insert_video(&video).await?;
        // move file to storage only if everything is successful
        let target = Path::new(&CONFIG.video_storage).join(&fid);
        file.file.move_copy_to(target).await.expect("Failed to move file to storage");
        videos.push(video);
    }
    UploadResponse { inner: videos }.into()
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct GetResponse {
    inner: Vec<Video>,
}

impl ApiResponse for GetResponse {}

#[get("/?<limit>&<skip>")]
pub(crate) async fn list(
    user: Result<UserGuard<()>, AuthenticationError>, 
    db: DBWrapper,
    limit: Option<u32>,
    skip: Option<u32>,
    ) -> ApiResponder<GetResponse> {
    let user = user?.user;
    if !user.allowed(Permissions::VIEW_VIDEOS) {
        return AuthenticationError::InsufficientPermissions(Permissions::VIEW_VIDEOS).into();
    }

    let limit = match limit {
        Some(l) => l.min(50),
        None => 50,
    };

    let (count, videos) = db.get_user_videos(&user, true, skip, Some(limit)).await?;

    ApiResponder::OkWithHeaders(GetResponse { inner: videos }, vec![("X-Total-Count", count.to_string())])
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct GetFileResponse {
    inner: Vec<VideoFile>,
}

impl ApiResponse for GetFileResponse {}


#[get("/file")]
pub(crate) async fn list_file(user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<GetFileResponse> {
    let user = user?.user;
    if !user.allowed(Permissions::VIEW_VIDEOS) {
        return AuthenticationError::InsufficientPermissions(Permissions::VIEW_VIDEOS).into();
    }
    GetFileResponse { inner: db.get_video_files(vec![]).await? }.into()
}

pub(crate) struct ThumbResponder(Option<NamedFile>);

impl<'r, 'o: 'r> rocket::response::Responder<'r, 'o> for ThumbResponder {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        match self.0 {
            Some(file) => file.respond_to(request),
            None => Redirect::to("/static/placeholder.png").respond_to(request),
        }
    }
}

// TODO: add authentication to this route
//  - not that simple: flutter's image caching wont work with auth.
//  - we may use the video token to get the thumb
//  - what the hell, we can just keep this public.
#[get("/<id>/thumb")]
pub(crate) async fn thumb(id: &str) -> ThumbResponder {
    match VideoFile::thumb(id) {
        Some(f) => ThumbResponder(Some(NamedFile::open(f).await.unwrap())),
        None => ThumbResponder(None),
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct TokenResponse {
    inner: ExpiringToken,
}

impl ApiResponse for TokenResponse {}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum TokenError {
    VideoNotFound,
}

impl ApiErrorType for TokenError {
    fn ty(&self) -> &'static str {
        match self {
            Self::VideoNotFound => "video_not_found",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            Self::VideoNotFound => rocket::http::Status::NotFound,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::VideoNotFound => "Video not found".to_string(),
        }
    }
}

#[get("/<video>/token")]
pub(crate) async fn get_token(video: &str, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<TokenResponse> {
    let video = match db.get_video(video).await? {
        Some(v) => v,
        None => return ApiResponder::Err(TokenError::VideoNotFound.into()),
    };

    if video.user_authorized(user.as_ref().ok().map(|u| &u.user), &db).await? {
        let token = video.generate_token();
        db.add_video_token(&token).await?;
        TokenResponse { inner: token.token }.into()
    } else {
        match user {
            Ok(_) => AuthenticationError::InsufficientPermissions(Permissions::WATCH_VIDEO).into(),
            Err(e) => e.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct DeleteResponse {
    inner: String,
}

impl ApiResponse for DeleteResponse {}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum DeleteError {
    VideoNotFound,
    DeletionError,
}

impl ApiErrorType for DeleteError {
    fn ty(&self) -> &'static str {
        match self {
            Self::VideoNotFound => "video_not_found",
            Self::DeletionError => "deletion_error",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            Self::VideoNotFound => rocket::http::Status::NotFound,
            Self::DeletionError => rocket::http::Status::InternalServerError,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::VideoNotFound => "Video not found".to_string(),
            Self::DeletionError => "Error while deleting video".to_string(),
        }
    }
}

#[delete("/<video>")]
pub(crate) async fn delete(video: &str, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<DeleteResponse> {
    let video = match db.get_video_resolved(video).await? {
        Some(v) => v,
        None => return ApiResponder::Err(DeleteError::VideoNotFound.into()),
    };
    let user = user?.user;
    if video.owner != user.username && !user.allowed(Permissions::ADMIN) {
        AuthenticationError::InsufficientPermissions(Permissions::ADMIN).into()
    } else {
        db.delete_video(&video).await?;
        if video.file.unwrap_right().delete().is_err() {
            ApiResponder::Err(DeleteError::DeletionError.into())
        } else {
            DeleteResponse { inner: video.id }.into()
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct UpdateForm {
    name: Option<String>,
    public: Option<bool>,
    game: Option<String>,
}

impl UpdateForm {
    fn apply_to(self, video: &mut Video) {
        let Self { name, public, game } = self;
        if let Some(name) = name {
            video.name = Some(name.clone());
        }
        if let Some(public) = public {
            video.public = public;
        }
        if let Some(game) = game {
            video.game = game.clone();
        }
    }
}

#[derive(Serialize)]
pub(crate) struct UpdateResponse {
    inner: Video,
}

impl ApiResponse for UpdateResponse {}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum UpdateError {
    VideoNotFound,
    // UpdateError,
}

impl ApiErrorType for UpdateError {
    fn ty(&self) -> &'static str {
        match self {
            Self::VideoNotFound => "video_not_found",
            // Self::UpdateError => "update_error",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            Self::VideoNotFound => rocket::http::Status::NotFound,
            // Self::UpdateError => rocket::http::Status::InternalServerError,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::VideoNotFound => "Video not found".to_string(),
            // Self::UpdateError => "Error while updating video".to_string(),
        }
    }
}

#[post("/<video>", data = "<form>", format = "json")]
pub(crate) async fn update(video: &str, form: Json<UpdateForm>, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<UpdateResponse> {
    let user = user?.user;
    let mut video = match db.get_video(video).await? {
        Some(v) => v,
        None => return ApiResponder::Err(UpdateError::VideoNotFound.into()),
    };
    let user_games = db.get_user_games_ids(&user).await?;
    if let Some(ref game) = form.game {
        // check if user is part of target game
        //   if the user is admin, it's automatically in all games.
        if !user_games.contains(game) {
            return AuthenticationError::GameNotAllowed.into();
        }
    }
    // check if user owns the video or has permission to modify others' videos.
    //   if the user can modify others' videos, it will also need to be in the source game.
    if video.owner == user.username || (user.allowed(Permissions::MODIFY_VIDEO_OTHERS) && user_games.contains(&video.game)){
        form.into_inner().apply_to(&mut video);
        db.update_video(&video).await?;
        UpdateResponse { inner: video }.into()
    } else {
        AuthenticationError::InsufficientPermissions(Permissions::MODIFY_VIDEO_OTHERS).into()
    }
}
