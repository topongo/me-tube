use std::collections::HashMap;
use rocket::futures::{StreamExt, TryStreamExt};

use rocket_db_pools::mongodb::{self, bson::doc};
use serde::{Deserialize, Serialize};

use crate::{authentication::{AuthenticationError, UserGuard}, db::DBWrapper, response::{ApiErrorType, ApiResponder, ApiResponse}, user::User, video::Video};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Like {
    user: String,
    video: String,
}

impl DBWrapper {
    pub(crate) async fn get_user_likes(&self, user: &User) -> Result<Vec<String>, mongodb::error::Error> {
        self
            .collection::<()>(Self::LIKES)
            .aggregate(vec![
                doc! {"$match": {"user": &user.username}},
                doc! {"$project": {"video": 1, "_id": 0}},
            ], None)
            .await?
            .map(|d| d.map(|d| d.get_str("video").unwrap().to_string()))
            .try_collect::<Vec<String>>()
            .await
    }

    pub(crate) async fn get_videos_likes(&self, user: &User) -> Result<HashMap<String, u16>, mongodb::error::Error> {
        let videos = self.get_user_videos(user, false, None, None)
            .await?
            .1
            .into_iter()
            .map(|v| v.id)
            .collect::<Vec<_>>();
        let likes = self
            .collection::<()>(Self::LIKES)
            .aggregate(vec![
                doc! {"$match": {"video": {"$in": videos.as_slice()} }},
                doc! {"$group": {"_id": "$video", "count": {"$sum": 1}}},
            ], None)
            .await?
            .map(|d| d.map(|d| (d.get_str("_id").unwrap().to_string(), d.get_i32("count").unwrap() as u16)))
            .try_collect::<HashMap<String, u16>>()
            .await?;
        Ok(videos
            .into_iter()
            .map(|v| {
                let n = *likes.get(&v).unwrap_or(&0);
                (v, n)
            })
            .collect::<HashMap<_, _>>()
        )
    }

    pub(crate) async fn add_like(&self, user: &User, video: &Video) -> Result<(), mongodb::error::Error> {
        self
            .collection(Self::LIKES)
            .replace_one(
                doc! {"user": &user.username, "video": &video.id},
                doc! {"user": &user.username, "video": &video.id},
                mongodb::options::ReplaceOptions::builder().upsert(true).build(),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn remove_like(&self, user: &User, video: &Video) -> Result<(), mongodb::error::Error> {
        self
            .collection::<()>(Self::LIKES)
            .delete_one(doc! {"user": &user.username, "video": &video.id}, None)
            .await
            .map(|_| ())
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum LikeError {
    VideoNotFound,
}

impl ApiErrorType for LikeError {
    fn ty(&self) -> &'static str {
        match self {
            LikeError::VideoNotFound => "video_not_found",
        }
    }

    fn message(&self) -> String {
        match self {
            LikeError::VideoNotFound => "Video not found".to_owned(),
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            LikeError::VideoNotFound => rocket::http::Status::NotFound,
        }
    }
}

#[post("/<video>/like")]
pub(crate) async fn add(video: &str, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<()> {
    let user = user?.user;
    match db.get_video(video).await? {
        Some(video) => {
            if video.user_authorized(Some(&user), &db).await? {
                db.add_like(&user, &video).await?;
                ApiResponder::Ok(())
            } else {
                ApiResponder::Err(LikeError::VideoNotFound.into())
            }
        }
        None => ApiResponder::Err(LikeError::VideoNotFound.into()),
    }
}

#[delete("/<video>/like")]
pub(crate) async fn delete(video: &str, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<()> {
    let user = user?.user;
    match db.get_video(video).await? {
        Some(video) => {
            if video.user_authorized(Some(&user), &db).await? {
                db.remove_like(&user, &video).await?;
                ApiResponder::Ok(())
            } else {
                ApiResponder::Err(LikeError::VideoNotFound.into())
            }
        }
        None => ApiResponder::Err(LikeError::VideoNotFound.into()),
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct VideoList(Vec<String>);

impl ApiResponse for VideoList {}

// get likes that the user left
#[get("/")]
pub(crate) async fn user(user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<VideoList> {
    let user = user?.user;
    let likes = db.get_user_likes(&user).await?;
    VideoList(likes).into()
}

impl ApiResponse for bool {}

// check if user left like on a video
#[get("/<video>")]
pub(crate) async fn user_single(video: &str, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<bool> {
    let user = user?.user;
    match db.get_video(video).await? {
        Some(video) => db.get_user_likes(&user)
            .await?
            .into_iter()
            .any(|v| v == video.id)
            .into(),
        None => ApiResponder::Err(LikeError::VideoNotFound.into()),
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct VideoLikes(HashMap<String, u16>);

impl ApiResponse for VideoLikes {}

// get number of likes for each video that the user has access to, under /video/likes
#[get("/like")]
pub(crate) async fn video(user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<VideoLikes> {
    let user = user?.user;
    let videos = db.get_videos_likes(&user).await?;
    VideoLikes(videos).into()
}

impl ApiResponse for u16 {}

// get number of likes for a single video, under /video/<video>/likes
#[get("/<video>/likes")]
pub(crate) async fn video_single(video: &str, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<u16> {
    let user = user?.user;
    match db.get_video(video).await? {
        Some(video) => {
            if video.user_authorized(Some(&user), &db).await? {
                let likes = db
                    .collection::<()>(DBWrapper::LIKES)
                    .count_documents(doc! {"video": video.id, "user": user.username}, None)
                    .await?;
                ApiResponder::Ok(likes as u16)
            } else {
                ApiResponder::Err(LikeError::VideoNotFound.into())
            }
        }
        None => ApiResponder::Err(LikeError::VideoNotFound.into()),
    }
}
