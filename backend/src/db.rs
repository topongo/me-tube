use rocket_db_pools::mongodb::bson::doc;
use rocket_db_pools::Database;
use rocket_db_pools::mongodb::Client;
use rocket_db_pools::mongodb;

use crate::response::ApiErrorType;

#[derive(Database)]
#[database("metube")]
pub struct Db(Client);

pub(crate) struct DBWrapper(Client);

impl DBWrapper {
    const DATABASE: &'static str = "metube";

    pub(crate) fn new(db: Client) -> Self {
        Self(db)
    }

    pub(crate) fn database(&self) -> mongodb::Database {
        self.0.database(Self::DATABASE)
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
