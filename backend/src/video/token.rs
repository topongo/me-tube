use chrono::TimeDelta;
use rocket_db_pools::mongodb::{self, bson::doc};
use serde::{Deserialize, Serialize};

use crate::{db::DBWrapper, user::ExpiringToken};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct VideoToken {
    pub(crate) token: ExpiringToken,
    pub(crate) video: String,
}

impl VideoToken {
    pub(crate) fn new(video: &str) -> Self {
        Self {
            token: ExpiringToken::new(TimeDelta::minutes(5)),
            video: video.to_string(),
        }
    }
}

impl DBWrapper {
    pub(crate) async fn add_video_token(&self, token: &VideoToken) -> Result<(), mongodb::error::Error> {
        self.database()
            .collection::<VideoToken>("video_tokens")
            .insert_one(token, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_video_token(&self, token: &str) -> Result<Option<VideoToken>, mongodb::error::Error> {
        self.database()
            .collection("video_tokens")
            .find_one(doc! {"token.token": token}, None)
            .await
    }
}
