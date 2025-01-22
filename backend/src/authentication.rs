use rocket::request::FromRequest;
use rocket::form::{Form, FromForm};
use rocket::serde::json::Json;
use rocket_db_pools::{Connection, Database};
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
enum LoginResponse {
    Ok {
        access_token: String,
        refresh_token: String,
    },
    Error(LoginError),
}

#[derive(Serialize)]
enum LoginError {
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
enum RegisterResponse {
    Ok,
    Error(RegisterError),
}

#[derive(Serialize)]
enum RegisterError {
    UsernameTaken,
    InvalidPassword,
    InvalidUsername,
    DatabaseError,
}

impl ApiResponse for RegisterResponse {
    fn status(&self) -> rocket::http::Status {
        match self {
            RegisterResponse::Ok => rocket::http::Status::Ok,
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

impl Into<User> for RegisterForm {
    fn into(self) -> User {
        User::new(self.username, self.password)
    }
}

#[post("/register", data = "<form>", format = "json")]
pub(crate) async fn register(form: Json<RegisterForm>, db: Connection<Db>) -> ApiResponder<RegisterResponse> {
    let form = form.into_inner();
    let db = DBWrapper::new(db.into_inner());
    let user = match db.get_user(&form.username).await {
        Ok(u) => match u {
            Some(_) => return RegisterResponse::Error(RegisterError::UsernameTaken).into(),
            None => u,
        }
        Err(_) => return RegisterResponse::Error(RegisterError::DatabaseError).into()
    };
    if let Some(e) = form.validate() {
        return RegisterResponse::Error(e).into();
    }
    let user: User = form.into();
    if let Err(e) = db.add_user(user).await {
        panic!("{:?}", e)
    }

    RegisterResponse::Ok.into()
}

#[post("/login", data = "<form>", format = "json")]
pub(crate) async fn login(form: Json<RegisterForm>, db: Connection<Db>) -> ApiResponder<LoginResponse> {
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
            LoginResponse::Ok { access_token: access, refresh_token: refresh }.into()
        }
    } else {
        LoginResponse::Error(LoginError::InvalidCredentials).into()
    }
}

#[get("/me")]
pub(crate) async fn me(auth: Authorization, db: Connection<Db>) -> ! {
    let db = DBWrapper::new(db.into_inner());
    let user = match db.get_user_by_access(&auth.0).await {
        Ok(u) => match u {
            // user exists and have the right access token
            Some(u) => u,
            // access token is invalid or expired
            None => {},
        }
    };
    Json(user)
}
