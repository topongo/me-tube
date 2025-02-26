use std::marker::PhantomData;

use rocket::http::CookieJar;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use rocket_db_pools::mongodb;
use serde::{Deserialize, Serialize};

use crate::db::DBWrapper;
use crate::response::{ApiErrorType, ApiResponder, ApiResponse};
use crate::user::{Permissions, User};

struct Authorization(String);

impl Authorization {
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorization {
    type Error = AuthenticationError;

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let auth = request.headers().get_one("Authorization");
        match auth {
            Some(auth) => {
                let auth = auth.split_whitespace().collect::<Vec<&str>>();
                if auth.len() != 2 || auth[0] != "Bearer" {
                    return Self::Error::MalformedAccessToken.outcome()
                }
                rocket::request::Outcome::Success(Authorization(auth[1].to_string()))
            },
            None => Self::Error::MissingAccessToken.outcome()
        }
    }
}


pub(crate) trait UserGuardType {}

impl UserGuardType for () {}

pub(crate) struct OkExpired;
impl UserGuardType for OkExpired {}

pub(crate) struct IsAdmin;
impl UserGuardType for IsAdmin {}

#[derive(Debug)]
pub(crate) struct UserGuard<T> where T: UserGuardType {
    pub(crate) user: User,
    _phantom: PhantomData<T>,
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for UserGuard<OkExpired> {
    type Error = AuthenticationError;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let db = match request.guard::<DBWrapper>().await {
            Outcome::Success(db) => db,
            Outcome::Error(_) => return Self::Error::InternalServerError.outcome(),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };
        let auth = match request.guard::<Authorization>().await {
            Outcome::Success(auth) => auth,
            Outcome::Error(_) => return Self::Error::InvalidAccessToken.outcome(),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };
        let auth = auth.into_inner();
        let user = match db.get_user_by_access(&auth).await {
            Ok(u) => match u {
                Some(u) => {
                    if u.check_access(&auth) {
                        u
                    } else {
                        return Self::Error::ExpiredAccessToken.outcome()
                    }
                },
                None => return Self::Error::InvalidAccessToken.outcome(),
            }
            Err(e) => return Self::Error::DatabaseError(e).outcome(),
        };
        rocket::request::Outcome::Success(Self { user, _phantom: PhantomData })
    }
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for UserGuard<()> {
    type Error = AuthenticationError;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let user = match request.guard::<UserGuard<OkExpired>>().await {
            Outcome::Success(user) => user.user,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };
        if user.password_reset {
            return Self::Error::ExpiredPassword.outcome()
        }
        rocket::request::Outcome::Success(Self { user, _phantom: PhantomData })
    }
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for UserGuard<IsAdmin> {
    type Error = AuthenticationError;

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let user = match request.guard::<UserGuard<()>>().await {
            Outcome::Success(user) => user.user,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };
        if user.allowed(Permissions::ADMIN) {
            rocket::request::Outcome::Success(Self { user, _phantom: PhantomData })
        } else {
            Self::Error::InsufficientPermissions(Permissions::ADMIN).outcome()
        }
    }
}

#[derive(Serialize, Debug)]
pub enum AuthenticationError {
    InvalidAccessToken,
    ExpiredAccessToken,
    MissingAccessToken,
    #[serde(skip)]
    DatabaseError(mongodb::error::Error),
    InsufficientPermissions(u32),
    GameNotAllowed,
    MalformedAccessToken,
    InternalServerError,
    ExpiredPassword,
}

impl ApiErrorType for AuthenticationError {
    fn ty(&self) -> &'static str {
        match self {
            Self::InvalidAccessToken => "invalid_access_token",
            Self::ExpiredAccessToken => "expired_access_token",
            Self::MissingAccessToken => "missing_access_token",
            Self::DatabaseError(..) => "database_error",
            Self::InsufficientPermissions(..) => "insufficient_permissions",
            Self::MalformedAccessToken => "malformed_access_token",
            Self::InternalServerError => "internal_server_error",
            Self::GameNotAllowed => "game_not_allowed",
            Self::ExpiredPassword => "expired_password",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            Self::InvalidAccessToken => rocket::http::Status::Unauthorized,
            Self::ExpiredAccessToken => rocket::http::Status::Unauthorized,
            Self::MissingAccessToken => rocket::http::Status::Forbidden,
            Self::DatabaseError(..) => rocket::http::Status::InternalServerError,
            Self::InsufficientPermissions(..) => rocket::http::Status::Forbidden,
            Self::MalformedAccessToken => rocket::http::Status::Unauthorized,
            Self::InternalServerError => rocket::http::Status::InternalServerError,
            Self::ExpiredPassword => rocket::http::Status::Unauthorized,
            Self::GameNotAllowed => rocket::http::Status::Forbidden,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::InvalidAccessToken => "Invalid access token".to_string(),
            Self::ExpiredAccessToken => "Expired access token".to_string(),
            Self::MissingAccessToken => "Missing access token".to_string(),
            Self::DatabaseError(e) => {
                #[cfg(debug_assertions)]
                return format!("Debug error: {:?}", e);
                #[cfg(not(debug_assertions))]
                {
                    log::error!("{:?}", e);
                    "Database error".to_string()
                }
            },
            Self::InsufficientPermissions(p) => format!("Insufficient permissions: {}", Permissions::label(*p)),
            Self::MalformedAccessToken => "Malformed access token".to_string(),
            Self::InternalServerError => "Internal server error".to_string(),
            Self::ExpiredPassword => "Password expired".to_string(),
            Self::GameNotAllowed => "You are not part of this game".to_string(),
        }
    }
}

impl<T> From<AuthenticationError> for ApiResponder<T> where T: ApiResponse {
    fn from(e: AuthenticationError) -> Self {
        ApiResponder::Err(e.into())
    }
}

#[derive(Deserialize)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub(crate) struct LoginResponse {
    access_token: String,
}

impl ApiResponse for LoginResponse {}

#[derive(Serialize)]
pub(crate) enum LoginError {
    UserNotFound,
    InvalidCredentials,
}

impl ApiErrorType for LoginError {
    fn ty(&self) -> &'static str {
        match self {
            LoginError::UserNotFound => "user_not_found",
            LoginError::InvalidCredentials => "invalid_credentials",
        }
    }

    fn message(&self) -> String {
        match self {
            LoginError::UserNotFound => "User not found".to_string(),
            LoginError::InvalidCredentials => "Invalid credentials".to_string(),
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            LoginError::UserNotFound => rocket::http::Status::NotFound,
            LoginError::InvalidCredentials => rocket::http::Status::Unauthorized,
        }
    }
}

#[post("/login", data = "<form>", format = "json")]
pub(crate) async fn login(form: Json<LoginForm>, cookies: &CookieJar<'_>, db: DBWrapper) -> ApiResponder<LoginResponse> {
    let form = form.into_inner();

    let mut user = match db.get_user(&form.username).await? {
        Some(u) => u,
        None => return ApiResponder::Err(LoginError::UserNotFound.into()),
    };

    if user.verify_password(form.password) {
        let (access, refresh) = user.generate_access_and_refresh();
        db.update_user(&user).await?;
        #[cfg(debug_assertions)]
        {
            let user  = db.get_user(&form.username).await.unwrap().unwrap();
            assert!(user.check_access(&access));
            assert!(user.check_refresh(&refresh));
        }
        cookies.add_private(("refresh", refresh));
        LoginResponse { access_token: access }.into()
    } else {
        ApiResponder::Err(LoginError::InvalidCredentials.into())
    }
}


#[derive(Serialize)]
pub(crate) struct RefreshResponse {
    access_token: String,
}

impl ApiResponse for RefreshResponse {}

#[derive(Serialize)]
pub(crate) enum RefreshError {
    InvalidRefreshToken,
    MissingRefreshToken,
}

impl ApiErrorType for RefreshError {
    fn ty(&self) -> &'static str {
        match self {
            RefreshError::InvalidRefreshToken => "invalid_refresh_token",
            RefreshError::MissingRefreshToken => "missing_refresh_token",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            RefreshError::InvalidRefreshToken => rocket::http::Status::Unauthorized,
            RefreshError::MissingRefreshToken => rocket::http::Status::Forbidden,
        }
    }

    fn message(&self) -> String {
        match self {
            RefreshError::InvalidRefreshToken => "Invalid refresh token".to_string(),
            RefreshError::MissingRefreshToken => "Missing refresh token".to_string(),
        }
    }
}

#[post("/refresh")]
pub(crate) async fn refresh(cookies: &CookieJar<'_>, db: DBWrapper) -> ApiResponder<RefreshResponse> {
    match cookies.get_private("refresh") {
        Some(r) => {
            match db.get_user_by_refresh(r.value()).await? {
                Some(mut u) => {
                    if u.check_refresh(r.value()) {
                        let access = u.generate_access();
                        db.update_user(&u).await?;
                        cookies.add_private(("refresh", r.value().to_string()));
                        RefreshResponse { access_token: access }.into()
                    } else {
                        ApiResponder::Err(RefreshError::InvalidRefreshToken.into())
                    }
                },
                None => ApiResponder::Err(RefreshError::InvalidRefreshToken.into())
            }
        }
        None => ApiResponder::Err(RefreshError::MissingRefreshToken.into())
    }
}

