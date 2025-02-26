use std::collections::HashMap;
use lazy_static::lazy_static;
use chrono::{DateTime, TimeDelta, Utc};
use rocket::{futures::TryStreamExt, serde::json::Json};
use rocket_db_pools::mongodb::{self, bson::doc};
use serde::{Deserialize, Serialize};
use rand::{rngs::OsRng, RngCore};
use base64::{Engine, prelude::BASE64_URL_SAFE};
use argon2::{Argon2, PasswordHasher, PasswordVerifier, PasswordHash, password_hash::SaltString};
use crate::{authentication::{AuthenticationError, IsAdmin, OkExpired, UserGuard}, config::CONFIG, db::DBWrapper, response::{ApiErrorType, ApiResponder, ApiResponse}};

fn secure_rnd_string() -> String {
    let mut rng = OsRng;
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    BASE64_URL_SAFE.encode(bytes)
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ExpiringToken {
    pub(crate) token: String,
    expires: DateTime<Utc>,
}

impl ExpiringToken {
    pub(crate) fn new(duration: TimeDelta) -> Self {
        Self {
            token: secure_rnd_string(),
            expires: Utc::now() + duration,
        }
    }

    pub(crate) fn valid(&self, token: &str) -> bool {
        self.expires > Utc::now() && token == self.token
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    password_hash: String,
    access_token: Option<ExpiringToken>,
    refresh_token: Option<ExpiringToken>,
    permissions: Permissions,
    pub(crate) password_reset: bool,
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidUsername,
    InvalidPassword,
    UsernameTaken,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidUsername => write!(f, "Invalid username"),
            ValidationError::InvalidPassword => write!(f, "Invalid password"),
            ValidationError::UsernameTaken => write!(f, "Username taken"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct Permissions {
    inner: u32,
}

#[allow(dead_code)]
impl Permissions {
    pub const ADMIN: u32 = 1 << 0;
    pub const ADD_GAME: u32 = 1 << 1;
    pub const ADD_VIDEOS: u32 = 1 << 2;
    pub const VIEW_VIDEOS: u32 = 1 << 3;
    pub const VIEW_GAMES: u32 = 1 << 4;
    pub const READ_MEDIA: u32 = 1 << 5;
    pub const WATCH_VIDEO: u32 = 1 << 6;
    // Modify videos that are not owned by the user.
    // The modifying user must be part of the game.
    pub const MODIFY_VIDEO_OTHERS: u32 = 1 << 7;

    pub(crate) fn new() -> Self {
        Self { inner: 0 }
    }

    pub(crate) fn label(p: u32) -> &'static str {
        match p {
            Self::ADMIN => "forbidden",
            Self::ADD_GAME => "add games",
            Self::ADD_VIDEOS => "add videos",
            Self::VIEW_VIDEOS => "view videos",
            Self::VIEW_GAMES => "view games",
            Self::READ_MEDIA => "view media",
            Self::WATCH_VIDEO => "watch videos",
            Self::MODIFY_VIDEO_OTHERS => "modify videos of others",
            _ => "unknown",
        }
    }

    pub fn push(&mut self, permission: u32) {
        self.inner |= permission;
    }
}

impl User {
    fn password_hash(password: String) -> String {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string()
    }


    pub fn validate(username: Option<&str>, password: Option<&str>) -> Result<(), ValidationError> {
        if let Some(username) = username {
            if username.len() < 4 {
                return Err(ValidationError::InvalidUsername);
            }
        } 
        if let Some(password) = password {
            if password.len() < 8 ||
                // !password.chars().any(|c| c.is_uppercase()) ||
                // !password.chars().any(|c| c.is_lowercase()) ||
                !password.chars().any(|c| c.is_alphanumeric()) ||
                !password.chars().any(|c| c.is_numeric()) {
                Err(ValidationError::InvalidPassword)
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn create(username: String, password: String) -> Self {
        Self {
            username,
            password_hash: Self::password_hash(password),
            access_token: None,
            refresh_token: None,
            permissions: Permissions::new(),
            password_reset: false,
        }
    }

    pub fn push_permissions(&mut self, permission: u32) {
        self.permissions.push(permission);
    }

    pub(crate) fn verify_password(&self, password: String) -> bool {
        let argon2 = Argon2::default();
        let hash = PasswordHash::new(&self.password_hash).unwrap();
        argon2.verify_password(password.as_bytes(), &hash).is_ok()
    }

    pub(crate) fn check_access(&self, access_token: &str) -> bool {
        self.access_token
            .as_ref()
            .is_some_and(|t| t.valid(access_token))
    }

    pub(crate) fn check_refresh(&self, refresh_token: &str) -> bool {
        self.refresh_token
            .as_ref()
            .is_some_and(|t| t.valid(refresh_token))
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
    pub async fn get_user(&self, username: &str) -> Result<Option<User>, mongodb::error::Error> {
        self
            .collection(Self::USERS)
            .find_one(doc! {"username": username}, None)
            .await
    }

    pub async fn add_user(&self, user: User) -> Result<(), mongodb::error::Error> {
        self
            .collection(Self::USERS)
            .insert_one(user, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn update_user(&self, user: &User) -> Result<(), mongodb::error::Error> {
        self
            .collection::<User>(Self::USERS)
            .replace_one(doc! {"username": &user.username}, user, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_user(&self, user: User) -> Result<(), mongodb::error::Error> {
        self
            .collection::<User>(Self::USERS)
            .delete_one(doc! {"username": user.username}, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_user_by_access(&self, access: &str) -> Result<Option<User>, mongodb::error::Error> {
        self
            .collection(Self::USERS)
            .find_one(doc! {"access_token.token": access}, None)
            .await
    }

    pub(crate) async fn get_user_by_refresh(&self, access: &str) -> Result<Option<User>, mongodb::error::Error> {
        self
            .collection(Self::USERS)
            .find_one(doc! {"refresh_token.token": access}, None)
            .await
    }

    pub(crate) async fn get_users(&self) -> Result<Vec<User>, mongodb::error::Error>  {
        self
            .collection::<User>(Self::USERS)
            .find(None, None)
            .await?
            .try_collect()
            .await
    }
}

#[derive(Serialize)]
#[serde(remote = "User")]
struct StrippedUser {
    username: String,
    permissions: Permissions,
}

#[derive(Serialize)]
struct StrippedUserWrapper(#[serde(with = "StrippedUser")] User);

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct ListUsersResponse {
    users: Vec<StrippedUserWrapper>,
}

impl From<Vec<User>> for ListUsersResponse {
    fn from(users: Vec<User>) -> Self {
        ListUsersResponse {
            users: users.into_iter().map(StrippedUserWrapper).collect(),
        }
    }
}

impl ApiResponse for ListUsersResponse {}

#[get("/")]
pub(crate) async fn list(user: Result<UserGuard<IsAdmin>, AuthenticationError>, db: DBWrapper) -> ApiResponder<ListUsersResponse> {
    let _ = user?;
    match db.get_users().await {
        Ok(users) => ListUsersResponse::from(users).into(),
        Err(e) => ApiResponder::Err(e.into())
    }
}

#[derive(Serialize)]
pub(crate) struct MeResponse {
    username: String,
    is_admin: bool,
    password_reset: bool,
}

impl ApiResponse for MeResponse {}

#[get("/me")]
pub(crate) async fn me(user: Result<UserGuard<OkExpired>, AuthenticationError>) -> ApiResponder<MeResponse> {
    let user = user?.user;

    let is_admin = user.allowed(Permissions::ADMIN);
    let username = user.username;
    let password_reset = user.password_reset;
    MeResponse { username, is_admin, password_reset }.into()
}


#[derive(Deserialize)]
pub(crate) struct PatchForm {
    password: Option<String>,
    permissions: Option<Permissions>,
}

#[derive(Deserialize)]
pub(crate) struct PostForm {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub(crate) struct PostResponse;

impl ApiResponse for PostResponse {}

#[derive(Serialize)]
pub(crate) enum PostError {
    UsernameTaken,
    InvalidPassword,
    InvalidUsername,
    UserNotFound,
}

impl From<ValidationError> for PostError {
    fn from(e: ValidationError) -> Self {
        match e {
            ValidationError::InvalidUsername => PostError::InvalidUsername,
            ValidationError::InvalidPassword => PostError::InvalidPassword,
            ValidationError::UsernameTaken => PostError::UsernameTaken,
        }
    }
}

impl ApiErrorType for PostError {
    fn ty(&self) -> &'static str {
        match self {
            PostError::UsernameTaken => "username_taken",
            PostError::InvalidPassword => "invalid_password",
            PostError::InvalidUsername => "invalid_username",
            PostError::UserNotFound => "user_not_found",
        }
    }

    fn message(&self) -> String {
        match self {
            PostError::UsernameTaken => "Username taken".to_string(),
            PostError::InvalidPassword => "Invalid password".to_string(),
            PostError::InvalidUsername => "Invalid username".to_string(),
            PostError::UserNotFound => "User not found".to_string(),
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            PostError::UsernameTaken => rocket::http::Status::Conflict,
            PostError::InvalidPassword => rocket::http::Status::BadRequest,
            PostError::InvalidUsername => rocket::http::Status::BadRequest,
            PostError::UserNotFound => rocket::http::Status::NotFound,
        }
    }
}

#[patch("/<username>", data = "<form>", format = "json")]
pub(crate) async fn patch(form: Json<PatchForm>, username: &str, user: Result<UserGuard<OkExpired>, AuthenticationError>, db: DBWrapper) -> ApiResponder<PostResponse> {
    let PatchForm { password, permissions } = form.into_inner();
    // is user authenticated?
    let user = user?.user;
    // does target user exist?
    let target_user = db.get_user(username).await?;
    if target_user.is_none() {
        ApiResponder::Err(PostError::UserNotFound.into())
    } else {
        let mut target_user = target_user.unwrap();
        // modify permissions only if logged user is admin
        if !user.allowed(Permissions::ADMIN) && permissions.is_some() {
            return ApiResponder::Err(AuthenticationError::InsufficientPermissions(Permissions::ADMIN).into());
        } else {
            target_user.permissions = permissions.unwrap_or(target_user.permissions);
        }
        // modify other fields only if logged user is admin or target_user is self
        if user.allowed(Permissions::ADMIN) || user.username == target_user.username {
            User::validate(None, password.as_deref()).map_err(PostError::from)?;
            if let Some(password) = password {
                target_user.password_hash = User::password_hash(password);
                target_user.password_reset = false;
            }
            db.update_user(&target_user).await?;
            PostResponse.into()
        } else {
            ApiResponder::Err(AuthenticationError::InsufficientPermissions(Permissions::ADMIN).into())
        }
    }
}

#[post("/", data = "<form>", format = "json")]
pub(crate) async fn post(form: Json<PostForm>, user: Result<UserGuard<IsAdmin>, AuthenticationError>,  db: DBWrapper) -> ApiResponder<PostResponse> {
    // check for admin in user guard
    let _ = user?;
    let PostForm { username, password } = form.into_inner();
    // check if username is taken
    if db.get_user(&username).await?.is_some() {
        return ApiResponder::Err(PostError::UsernameTaken.into());
    }
    // check if username and password are valid
    User::validate(Some(&username), Some(&password)).map_err(PostError::from)?;
    let user = User::create(username.to_string(), password);
    db.add_user(user).await?;

    PostResponse.into()
}

#[delete("/<username>")]
pub(crate) async fn delete(username: &str, user: Result<UserGuard<IsAdmin>, AuthenticationError>, db: DBWrapper) -> ApiResponder<PostResponse> {
    let user = user?.user;
    match db.get_user(username).await? {
        Some(_) => {
            db.delete_user(user).await?;
            PostResponse.into()
        }
        None => ApiResponder::Err(PostError::UserNotFound.into()),
    }
}


lazy_static!(
    static ref TABLE: HashMap<&'static str, u32> = HashMap::from([
        ("admin", Permissions::ADMIN),
        ("add_game", Permissions::ADD_GAME),
        ("add_video", Permissions::ADD_VIDEOS),
        ("view_video", Permissions::VIEW_VIDEOS),
        ("view_game", Permissions::VIEW_GAMES),
        ("read_media", Permissions::READ_MEDIA),
        ("watch_video", Permissions::WATCH_VIDEO),
    ]);
);

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct PermissionsResponse(&'static HashMap<&'static str, u32>);

impl ApiResponse for PermissionsResponse {}

#[get("/permissions")]
pub(crate) async fn permissions(user: Result<UserGuard<IsAdmin>, AuthenticationError>) -> ApiResponder<PermissionsResponse> {
    let _ = user?.user;
    PermissionsResponse(&TABLE).into()
}
