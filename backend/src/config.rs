use std::path::PathBuf;

use chrono::TimeDelta;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationSeconds;

#[serde_as]    
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub(crate) struct MeTube {
    #[serde_as(as = "DurationSeconds<f64>")]
    pub(crate) access_token_duration: TimeDelta,
    #[serde_as(as = "DurationSeconds<f64>")]
    pub(crate) refresh_token_duration: TimeDelta,
    pub(crate) video_storage: String,
}

impl MeTube {
    pub(crate) fn check(&self) {
        if !PathBuf::from(&self.video_storage).exists() {
            panic!("Video storage path does not exist");
        } else {
            // if upload/thumbs does not exist, create it
            let thumbs = PathBuf::from(&self.video_storage).join("thumbs");
            if !thumbs.exists() {
                std::fs::create_dir_all(thumbs).expect("Failed to create thumbs directory");
            }
        }
    }
}

lazy_static!{
    pub(crate) static ref CONFIG: MeTube = {
        let config = std::fs::read_to_string("MeTube.toml").expect("Failed to read config file");
        toml::from_str(&config).expect("Failed to parse config file")
    };
}
