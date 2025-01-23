use rocket::http::CookieJar;
use rocket::request::FromRequest;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;
use serde::{Deserialize, Serialize};

use crate::db::{DBWrapper, Db};
use crate::response::{ApiError, ApiErrorType, ApiResponder, ApiResponse};
use crate::user::User;

pub(crate) struct Authorization(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorization {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let auth = request.headers().get_one("Authorization");
        match auth {
            Some(auth) => {
                let auth = auth.split_whitespace().collect::<Vec<&str>>();
                if auth.len() != 2 || auth[0] != "Bearer" {
                    return rocket::request::Outcome::Error((rocket::http::Status::Unauthorized, ()));
                }
                rocket::request::Outcome::Success(Authorization(auth[1].to_string()))
            },
            None => rocket::request::Outcome::Error((rocket::http::Status::Unauthorized, ()))
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum LoginResponse {
    Ok {
        access_token: String,
    },
    Error(LoginError),
}

#[derive(Serialize)]
pub(crate) enum LoginError {
    UserNotFound,
    InvalidCredentials,
    DatabaseError,
}

impl ApiErrorType for LoginError {
    fn ty(&self) -> &'static str {
        match self {
            LoginError::UserNotFound => "user_not_found",
            LoginError::InvalidCredentials => "invalid_credentials",
            LoginError::DatabaseError => "database_error",
        }
    }

    fn message(&self) -> String {
        match self {
            LoginError::UserNotFound => "User not found".to_string(),
            LoginError::InvalidCredentials => "Invalid credentials".to_string(),
            LoginError::DatabaseError => "Database error".to_string(),
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            LoginError::UserNotFound => rocket::http::Status::NotFound,
            LoginError::InvalidCredentials => rocket::http::Status::Unauthorized,
            LoginError::DatabaseError => rocket::http::Status::InternalServerError,
        }
    }
}

impl ApiResponse for LoginResponse {
    fn status(&self) -> rocket::http::Status {
        match self {
            LoginResponse::Ok { .. } => rocket::http::Status::Ok,
            LoginResponse::Error(e) => e.status(),
        }
    }

    fn respond(self) -> Result<Self, ApiError> {
        if let LoginResponse::Error(e) = self {
            Err(e.into())
        } else {
            Ok(self)
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct RegisterForm {
    username: String,
    password: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum RegisterResponse {
    Ok { },
    Error(RegisterError),
}

#[derive(Serialize)]
pub(crate) enum RegisterError {
    UsernameTaken,
    InvalidPassword,
    InvalidUsername,
    DatabaseError,
}

impl ApiResponse for RegisterResponse {
    fn status(&self) -> rocket::http::Status {
        match self {
            RegisterResponse::Ok { .. } => rocket::http::Status::Ok,
            RegisterResponse::Error(e) => e.status(),
        }
    }

    fn respond(self) -> Result<Self, ApiError> {
        if let RegisterResponse::Error(e) = self {
            Err(e.into())
        } else {
            Ok(self)
        }
    }
}

impl ApiErrorType for RegisterError {
    fn ty(&self) -> &'static str {
        match self {
            RegisterError::UsernameTaken => "username_taken",
            RegisterError::InvalidPassword => "invalid_password",
            RegisterError::InvalidUsername => "invalid_username",
            RegisterError::DatabaseError => "database_error",
        }
    }

    fn message(&self) -> String {
        match self {
            RegisterError::UsernameTaken => "Username taken".to_string(),
            RegisterError::InvalidPassword => "Invalid password".to_string(),
            RegisterError::InvalidUsername => "Invalid username".to_string(),
            RegisterError::DatabaseError => "Database error".to_string(),
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            RegisterError::UsernameTaken => rocket::http::Status::Conflict,
            RegisterError::InvalidPassword => rocket::http::Status::BadRequest,
            RegisterError::InvalidUsername => rocket::http::Status::BadRequest,
            RegisterError::DatabaseError => rocket::http::Status::InternalServerError,
        }
    }
}

impl RegisterForm {
    fn validate(&self) -> Option<RegisterError> {
        if self.username.len() < 4 {
            Some(RegisterError::InvalidUsername)
        } else if self.password.len() < 8 ||
            !self.password.chars().any(|c| c.is_uppercase()) ||
            !self.password.chars().any(|c| c.is_lowercase()) ||
            !self.password.chars().any(|c| c.is_numeric()) {
            Some(RegisterError::InvalidPassword)
        } else {
            None
        }
    }
}

impl From<RegisterForm> for User {
    fn from(form: RegisterForm) -> Self {
        User::new(form.username, form.password)
    }
}

#[post("/register", data = "<form>", format = "json")]
pub(crate) async fn register(form: Json<RegisterForm>, db: Connection<Db>) -> ApiResponder<RegisterResponse> {
    let form = form.into_inner();
    let db = DBWrapper::new(db.into_inner());
    match db.get_user(&form.username).await {
        Ok(u) => match u {
            //  username is taken
            Some(_) => return RegisterResponse::Error(RegisterError::UsernameTaken).into(),
            None => { /* ok, proceed */ },
        }
        // db error
        Err(_) => return RegisterResponse::Error(RegisterError::DatabaseError).into()
    }
    if let Some(e) = form.validate() {
        return RegisterResponse::Error(e).into();
    }
    let user: User = form.into();
    if let Err(e) = db.add_user(user).await {
        panic!("{:?}", e)
    }

    RegisterResponse::Ok { }.into()
}

#[post("/login", data = "<form>", format = "json")]
pub(crate) async fn login(form: Json<LoginForm>, cookies: &CookieJar<'_>, db: Connection<Db>) -> ApiResponder<LoginResponse> {
    let db = DBWrapper::new(db.into_inner());
    let form = form.into_inner();

    let mut user = match db.get_user(&form.username).await {
        Ok(u) => match u {
            Some(u) => u,
            None => return LoginResponse::Error(LoginError::UserNotFound).into(),
        }
        Err(_) => return LoginResponse::Error(LoginError::DatabaseError).into()
    };

    if user.verify_password(form.password) {
        let (access, refresh) = user.generate_access_and_refresh();
        if let Err(e) = db.update_user(&user).await {
            LoginResponse::Error(LoginError::DatabaseError).into()
        } else {
            #[cfg(debug_assertions)]
            {
                let user  = db.get_user(&form.username).await.unwrap().unwrap();
                assert!(user.check_access(&access));
                assert!(user.check_refresh(&refresh));
            }
            cookies.add_private(("refresh", refresh));
            LoginResponse::Ok { access_token: access }.into()
        }
    } else {
        LoginResponse::Error(LoginError::InvalidCredentials).into()
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum MeResponse {
    Ok { username: String },
    Error(MeError),
}

#[derive(Serialize)]
pub(crate) enum MeError {
    InvalidAccessToken,
    MissingAccessToken,
    DatabaseError,
}

impl ApiErrorType for MeError {
    fn ty(&self) -> &'static str {
        match self {
            MeError::InvalidAccessToken => "invalid_access_token",
            MeError::DatabaseError => "database_error",
            MeError::MissingAccessToken => "missing_access_token",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            MeError::InvalidAccessToken => rocket::http::Status::Unauthorized,
            MeError::DatabaseError => rocket::http::Status::InternalServerError,
            MeError::MissingAccessToken => rocket::http::Status::Forbidden,
        }
    }

    fn message(&self) -> String {
        match self {
            MeError::InvalidAccessToken => "Invalid access token".to_string(),
            MeError::DatabaseError => "Database error".to_string(),
            MeError::MissingAccessToken => "Missing access token".to_string(),
        }
    }
}

impl ApiResponse for MeResponse {
    fn status(&self) -> rocket::http::Status {
        match self {
            MeResponse::Ok { .. } => rocket::http::Status::Ok,
            MeResponse::Error(e) => e.status(),
        }
    }

    fn respond(self) -> Result<Self, ApiError> {
        match self {
            MeResponse::Ok { .. } => Ok(self),
            MeResponse::Error(e) => Err(e.into())
        }
    }
}

#[get("/me")]
pub(crate) async fn me(auth: Option<Authorization>, db: Connection<Db>) -> ApiResponder<MeResponse> {
    let auth = match auth {
        Some(a) => a,
        None => return MeResponse::Error(MeError::MissingAccessToken).into(),
    };
    let db = DBWrapper::new(db.into_inner());
    match db.get_user_by_access(&auth.0).await {
        Ok(u) => match u {
            // user exists and have the right access token
            Some(u) => {
                if u.check_access(&auth.0) {
                    // access token is valid
                    log::info!("access token is valid");
                    MeResponse::Ok { username: u.username }.into()
                } else {
                    // access token is expired
                    log::info!("access token is expired");
                    MeResponse::Error(MeError::InvalidAccessToken).into()
                }
            },
            // access token is invalid
            None => {
                log::info!("access token not in db");
                MeResponse::Error(MeError::InvalidAccessToken).into()
            }
        }
        Err(_) => MeResponse::Error(MeError::DatabaseError).into()
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum RefreshResponse {
    Ok { access_token: String },
    Error(RefreshError),
}

#[derive(Serialize)]
pub(crate) enum RefreshError {
    InvalidRefreshToken,
    MissingRefreshToken,
    DatabaseError,
}

impl ApiErrorType for RefreshError {
    fn ty(&self) -> &'static str {
        match self {
            RefreshError::InvalidRefreshToken => "invalid_refresh_token",
            RefreshError::DatabaseError => "database_error",
            RefreshError::MissingRefreshToken => "missing_refresh_token",
        }
    }

    fn status(&self) -> rocket::http::Status {
        match self {
            RefreshError::InvalidRefreshToken => rocket::http::Status::Unauthorized,
            RefreshError::DatabaseError => rocket::http::Status::InternalServerError,
            RefreshError::MissingRefreshToken => rocket::http::Status::Forbidden,
        }
    }

    fn message(&self) -> String {
        match self {
            RefreshError::InvalidRefreshToken => "Invalid refresh token".to_string(),
            RefreshError::DatabaseError => "Database error".to_string(),
            RefreshError::MissingRefreshToken => "Missing refresh token".to_string(),
        }
    }
}

impl ApiResponse for RefreshResponse {
    fn status(&self) -> rocket::http::Status {
        match self {
            RefreshResponse::Ok { .. } => rocket::http::Status::Ok,
            RefreshResponse::Error(e) => e.status(),
        }
    }

    fn respond(self) -> Result<Self, ApiError> {
        match self {
            RefreshResponse::Ok { .. } => Ok(self),
            RefreshResponse::Error(e) => Err(e.into())
        }
    }
}

#[post("/refresh")]
pub(crate) async fn refresh(cookies: &CookieJar<'_>, db: Connection<Db>) -> ApiResponder<RefreshResponse> {
    let refresh = cookies.get_private("refresh");
    let db = DBWrapper::new(db.into_inner());
    match refresh {
        Some(r) => {
            match db.get_user_by_refresh(r.value()).await {
                Ok(u) => match u {
                    Some(mut u) => {
                        if u.check_refresh(r.value()) {
                            let access = u.generate_access();
                            if let Err(e) = db.update_user(&u).await {
                                log::error!("{:?}", e);
                                RefreshResponse::Error(RefreshError::DatabaseError).into()
                            } else {
                                RefreshResponse::Ok { access_token: access }.into()
                            }
                        } else {
                            RefreshResponse::Error(RefreshError::InvalidRefreshToken).into()
                        }
                    },
                    None => RefreshResponse::Error(RefreshError::InvalidRefreshToken).into()
                }
                Err(e) => {
                    log::error!("{:?}", e);
                    RefreshResponse::Error(RefreshError::DatabaseError).into()
                }
            }
        }
        None => {
            RefreshResponse::Error(RefreshError::MissingRefreshToken).into()
        }
    }
}


