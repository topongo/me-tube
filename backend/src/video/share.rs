use rocket::{http::Status, response::Responder};

use crate::{db::DBWrapper, media::{MediaStream, Range, StreamError}};

pub(crate) enum ShareResponder {
    InternalError,
    NotFound,
    Ok(Result<MediaStream, StreamError>),
}

impl<'r> Responder<'r, 'r> for ShareResponder {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'r> {
        match self {
            ShareResponder::InternalError => {
                rocket::response::Response::build()
                    .status(Status::InternalServerError)
                    .ok()
            }
            ShareResponder::NotFound => {
                rocket::response::Response::build()
                    .status(Status::NotFound)
                    .ok()
            }
            ShareResponder::Ok(stream) => {
                match stream {
                    Ok(s) => s.respond_to(request),
                    Err(e) => e.respond_to(request),
                }
            }
        }
    }
}

#[get("/<video>")]
pub(crate) async fn get(video: &str, db: DBWrapper, range: Option<Range>) -> ShareResponder {
    match db.get_video_resolved(video).await {
        Ok(Some(mut v)) => {
            if !v.public {
                return ShareResponder::NotFound;
            }
            // resolve conversion
            match v.resolve_converted(&db).await {
                Ok(()) => ShareResponder::Ok(MediaStream::from_video(range, v).await),
                Err(_) => ShareResponder::InternalError,
            }
        }
        Ok(None) => ShareResponder::NotFound,
        Err(_) => ShareResponder::InternalError,
    }
}
