use chrono::{DateTime, TimeDelta, Utc};
use rocket_db_pools::mongodb::{self, bson::doc};
use serde::{Deserialize, Serialize};
use rand::{rngs::OsRng, RngCore};
use base64::{Engine, prelude::BASE64_STANDARD};
use argon2::{Argon2, PasswordHasher, PasswordVerifier, PasswordHash, password_hash::SaltString};

use crate::{config::CONFIG, db::DBWrapper};

fn secure_rnd_string() -> String {
    let mut rng = OsRng;
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    BASE64_STANDARD.encode(bytes)
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ExpiringToken {
    token: String,
    expires: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct User {
    pub username: String,
    password_hash: String,
    access_token: Option<ExpiringToken>,
    refresh_token: Option<ExpiringToken>,
    permissions: Permissions,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct Permissions {
    inner: u32,
}

impl Permissions {
    pub(crate) const ADMIN: u32 = 1 << 0;
    pub(crate) const ADD_GAME: u32 = 1 << 1;
    pub(crate) const ADD_VIDEO: u32 = 1 << 2;
    pub(crate) const VIEW_VIDEO: u32 = 1 << 3;
    pub(crate) const VIEW_GAME: u32 = 1 << 4;
    pub(crate) const READ_MEDIA: u32 = 1 << 5;

    pub(crate) fn new() -> Self {
        Self { inner: 0 }
    }
}

impl User {
    fn password_hash(password: String) -> String {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string()
    }

    pub(crate) fn new(username: String, password: String) -> Self {
        Self {
            username,
            password_hash: Self::password_hash(password),
            access_token: None,
            refresh_token: None,
            permissions: Permissions::new(),
        }
    }

    pub(crate) fn verify_password(&self, password: String) -> bool {
        let argon2 = Argon2::default();
        let hash = PasswordHash::new(&self.password_hash).unwrap();
        argon2.verify_password(password.as_bytes(), &hash).is_ok()
    }

    pub(crate) fn check_access(&self, access_token: &str) -> bool {
        println!("access token will expire in {:?}", self.access_token.as_ref().map(|t| t.expires - Utc::now()));
        self.access_token.as_ref().map_or(false, |t| t.expires > Utc::now() && t.token == access_token)
    }

    pub(crate) fn check_refresh(&self, refresh_token: &str) -> bool {
        println!("refresh token will expire in {:?}", self.refresh_token.as_ref().map(|t| t.expires - Utc::now()));
        self.refresh_token.as_ref().map_or(false, |t| t.expires > Utc::now() && t.token == refresh_token)
    }

    pub(crate) fn generate_expiring_token(duration: TimeDelta) -> ExpiringToken {
        ExpiringToken {
            token: secure_rnd_string(),
            expires: Utc::now() + duration,
        }
    }

    pub(crate) fn generate_access(&mut self) -> String {
        let access = Self::generate_expiring_token(CONFIG.access_token_duration);
        let r = access.token.clone();
        self.access_token = Some(access);
        r
    }

    pub(crate) fn generate_refresh(&mut self) -> String {
        let refresh = Self::generate_expiring_token(CONFIG.refresh_token_duration);
        let r = refresh.token.clone();
        self.refresh_token = Some(refresh);
        r
    }

    pub(crate) fn generate_access_and_refresh(&mut self) -> (String, String) {
        (self.generate_access(), self.generate_refresh())
    }

    pub(crate) fn allowed(&self, permission: u32) -> bool {
        self.permissions.inner & Permissions::ADMIN != 0 ||
        self.permissions.inner & permission != 0
    }
}

impl DBWrapper {
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

