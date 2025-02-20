use rocket::request::FromRequest;
use rocket::Build;
use rocket_db_pools::mongodb::bson::doc;
use rocket_db_pools::{Connection, Database};
use rocket_db_pools::mongodb::{Client, IndexModel};
use rocket_db_pools::mongodb;

use crate::authentication::AuthenticationError;
use crate::response::ApiErrorType;

#[derive(Database)]
#[database("metube")]
pub struct Db(Client);

pub(crate) struct DBWrapper(Client);

impl DBWrapper {
    const DATABASE: &'static str = "metube_testing";

    fn new(db: Client) -> Self {
        Self(db)
    }

    async fn _enforce_constraints(&self) {
        self.database()
            .collection::<()>("videos")
            .create_index(IndexModel::builder().keys(doc! {"id": 1}).build(), None)
            .await.unwrap();
        self.database()
            .collection::<()>("video_files")
            .create_index(IndexModel::builder().keys(doc! {"id": 1}).build(), None)
            .await.unwrap();
        self.database()
            .collection::<()>("users")
            .create_index(IndexModel::builder().keys(doc! {"username": 1}).build(), None)
            .await.unwrap();
        self.database()
            .collection::<()>("games")
            .create_index(IndexModel::builder().keys(doc! {"id": 1}).build(), None)
            .await.unwrap();
        self.database()
            .collection::<()>("game_users")
            .create_index(IndexModel::builder().keys(doc! {"game": 1, "user": 1}).build(), None)
            .await.unwrap();
    }

    pub(crate) fn database(&self) -> mongodb::Database {
        self.0.database(Self::DATABASE)
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

