use rocket::request::FromRequest;
use rocket::Build;
use rocket_db_pools::mongodb::bson::doc;
use rocket_db_pools::{Connection, Database};
use rocket_db_pools::mongodb::{Client, IndexModel};
use rocket_db_pools::mongodb;

use crate::authentication::AuthenticationError;
use crate::config::CONFIG;
use crate::response::ApiErrorType;

#[derive(Database)]
#[database("metube")]
pub struct Db(Client);

pub struct DBWrapper(Client);

impl DBWrapper {
    pub const USERS: &'static str = "users";
    pub const VIDEOS: &'static str = "videos";
    pub const VIDEO_FILES: &'static str = "video_files";
    pub const GAMES: &'static str = "games";
    pub const GAME_USERS: &'static str = "game_users";
    pub const LIKES: &'static str = "likes";
    pub const VIDEO_TOKENS: &'static str = "video_tokens";

    pub fn new(db: Client) -> Self {
        Self(db)
    }

    async fn _enforce_constraints(&self) {
        let unique_options = mongodb::options::IndexOptions::builder().unique(true).build();

        for (c, d) in vec![
            (Self::USERS, doc! {"username": 1}),
            (Self::GAME_USERS, doc! {"game": 1, "user": 1}),
            (Self::LIKES, doc! {"user": 1, "video": 1}),
        ] {
            self.database()
                .collection::<()>(c)
                .create_index(IndexModel::builder().keys(d).options(unique_options.clone()).build(), None)
                .await.unwrap();
        }
    }

    pub(crate) fn collection<T>(&self, name: &'static str) -> mongodb::Collection<T> {
        self.database().collection(name)
    }

    pub fn database(&self) -> mongodb::Database {
        self.0.database(CONFIG.database.as_str())
    }

    pub(crate) async fn constraints_fairing(rocket: rocket::Rocket<Build>) -> rocket::fairing::Result {
        match Db::fetch(&rocket) {
            Some(db) => {
                let db = DBWrapper::new(db.0.clone());
                db._enforce_constraints().await;
                Ok(rocket)
            }
            None => {
                eprintln!("Failed to fetch database connection for constraints fairing");
                Err(rocket)
            }
        }
    }
}

impl ApiErrorType for mongodb::error::Error {
    fn ty(&self) -> &'static str {
        "database_error"
    }

    fn message(&self) -> String {
        format!("{}", self)
    }

    fn status(&self) -> rocket::http::Status {
        rocket::http::Status::InternalServerError
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DBWrapper {
    type Error = AuthenticationError;

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let db = request.guard::<Connection<Db>>().await.unwrap();
        let db = DBWrapper::new(db.into_inner());
        db._enforce_constraints().await;
        rocket::request::Outcome::Success(db)
    }
}

