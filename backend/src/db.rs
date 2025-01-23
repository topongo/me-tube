use rocket_db_pools::mongodb::bson::doc;
use rocket_db_pools::Database;
use rocket_db_pools::mongodb::Client;
use rocket_db_pools::mongodb;

use crate::user::User;

#[derive(Database)]
#[database("metube")]
pub struct Db(Client);

pub(crate) struct DBWrapper(Client);

impl DBWrapper {
    const DATABASE: &'static str = "metube";

    pub(crate) fn new(db: Client) -> Self {
        Self(db)
    }

    fn database(&self) -> mongodb::Database {
        self.0.database(Self::DATABASE)
    }

    pub(crate) async fn get_user(&self, username: &str) -> Result<Option<User>, mongodb::error::Error> {
        self
            .database()
            .collection("users")
            .find_one(doc! {"username": username}, None)
            .await
    }

    pub(crate) async fn add_user(&self, user: User) -> Result<(), mongodb::error::Error> {
        self
            .database()
            .collection("users")
            .insert_one(user, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn update_user(&self, user: &User) -> Result<(), mongodb::error::Error> {
        self
            .database()
            .collection::<User>("users")
            .replace_one(doc! {"username": user.username.clone()}, user, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_user_by_access(&self, access: &str) -> Result<Option<User>, mongodb::error::Error> {
        self
            .database()
            .collection("users")
            .find_one(doc! {"access_token.token": access}, None)
            .await
    }

    pub(crate) async fn get_user_by_refresh(&self, access: &str) -> Result<Option<User>, mongodb::error::Error> {
        self
            .database()
            .collection("users")
            .find_one(doc! {"refresh_token.token": access}, None)
            .await
    }

    #[cfg(debug_assertions)]
    pub(crate) async fn dump_users(&self) -> Result<Vec<User>, mongodb::error::Error> {
        use rocket::futures::TryStreamExt;

        self
            .database()
            .collection("users")
            .find(None, None)
            .await?
            .try_collect()
            .await
    }
}
