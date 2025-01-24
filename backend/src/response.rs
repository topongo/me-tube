use rocket::{http::Status, response::Responder, serde::json::Json, Response};
use serde::Serialize;

pub(crate) trait ApiResponse: Serialize {
    fn status(&self) -> Status;
    fn respond(self) -> Result<Self, ApiError> where Self: Sized;
}

#[derive(Serialize)]
pub(crate) struct ApiError {
    #[serde(rename = "type")]
    ty: &'static str,
    message: String,
}

pub(crate) trait ApiErrorType {
    fn ty(&self) -> &'static str;
    fn message(&self) -> String;
    fn status(&self) -> Status;
}

impl<T> From<T> for ApiError where T: ApiErrorType {
    fn from(e: T) -> Self {
        Self {
            ty: e.ty(),
            message: e.message(),
        }
    }
}

pub(crate) struct ApiResponder<T> where T: ApiResponse {
    inner: T
}

impl<T> From<T> for ApiResponder<T> where T: ApiResponse {
    fn from(inner: T) -> Self {
        Self { inner }
    }
}

#[derive(Serialize)]
struct ApiErrorResponder {
    error: &'static str,
    message: String,
    #[serde(skip)]
    status: Status,
}

impl ApiResponse for ApiErrorResponder {
    fn status(&self) -> Status {
        self.status
    }

    fn respond(self) -> Result<Self, ApiError> where Self: Sized {
        Ok(self)
    }
}

impl<T> From<T> for ApiResponder<ApiErrorResponder> where T: ApiErrorType {
    fn from(value: T) -> Self {
        Self {
            inner: ApiErrorResponder {
                error: value.ty(),
                message: value.message(),
                status: value.status(),
            }
        }
    }
}

impl<'r, 'o: 'r, T> Responder<'r, 'o> for ApiResponder<T> where T: ApiResponse {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let mut build = Response::build();
        build.status(self.inner.status());
        match self.inner.respond() {
            Ok(inner) => build.merge(Json(inner).respond_to(request)?),
            Err(e) => build.merge(Json(e).respond_to(request)?),
        };
        build.ok()
    }
}

