use std::{convert::Infallible, ops::FromResidual};

use rocket::{http::Status, response::Responder, serde::json::Json, Response};
use serde::Serialize;

pub(crate) trait ApiResponse: Serialize {}

pub(crate) trait ApiErrorType {
    fn ty(&self) -> &'static str;
    fn message(&self) -> String;
    fn status(&self) -> Status;
}

pub(crate) enum ApiResponder<T> where T: ApiResponse {
    Ok(T),
    Err(ApiError),
}

#[derive(Serialize, Debug)]
pub(crate) struct ApiError {
    error: &'static str,
    message: String,
    #[serde(skip)]
    status: Status,
}

impl<T> From<T> for ApiError where T: ApiErrorType {
    fn from(inner: T) -> Self {
        Self {
            error: inner.ty(),
            message: inner.message(),
            status: inner.status(),
        }
    }
}


impl<T> From<T> for ApiResponder<T> where T: ApiResponse {
    fn from(inner: T) -> Self {
        Self::Ok(inner)
    }
}

impl<'r, 'o: 'r, T> Responder<'r, 'o> for ApiResponder<T> where T: ApiResponse {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let mut build = Response::build();
        match self {
            Self::Ok(inner) => {
                build.status(rocket::http::Status::Ok);
                build.merge(Json(inner).respond_to(request)?);
            }
            Self::Err(inner) => {
                log::warn!("API Error: {:?}", inner);
                build.status(inner.status);
                build.merge(Json(inner).respond_to(request)?);
            }
        }
        build.ok()
    }
}

impl<E, T> FromResidual<Result<Infallible, E>> for ApiResponder<T> where E: ApiErrorType, T: ApiResponse {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        Self::Err(residual.map_err(Into::into).unwrap_err())
    }
}

