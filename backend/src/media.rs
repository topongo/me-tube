use std::{io::SeekFrom, path::Path, pin::Pin, task::{Context, Poll}};

use rocket::{request::{FromRequest, Outcome}, response::{stream::ByteStream, Responder}, serde::json::Json, tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, ReadBuf}};
use rocket_db_pools::Connection;
use rocket::tokio::fs::File;
use serde::Serialize;

use crate::{authentication::{AuthenticationError, UserGuard}, config::CONFIG, db::Db, response::{ApiError, ApiResponder, ApiResponse}, user::Permissions};

// struct RangedResponder {
//     file: Path,
//     start: Option<u64>,
//     end: Option<u64>,
// }
//
// impl Responder for RangedResponder {
//     fn respond_to(self, _: &rocket::Request) -> rocket::response::Result<'static> {
//
//     }
// }

struct Range {
    start: Option<u64>,
    end: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[derive(Debug)]
enum RangeError {
    InvalidRange,
    InvalidStart,
    InvalidEnd,
}

#[async_trait]
impl<'r> FromRequest<'r> for Range {
    type Error = RangeError;

    async fn from_request(request: &'r rocket::request::Request<'_>) -> Outcome<Self, Self::Error> {
        let head = request.headers().get_one("range").unwrap_or_default();

        fn r(r: RangeError) -> Outcome<Range, RangeError> {
            Outcome::Error((rocket::http::Status::BadRequest, r))
        }

        let head = head.split('=').collect::<Vec<&str>>();
        if head.len() != 2 || head[0] != "bytes" {
            return r(RangeError::InvalidRange);
        }
        let ends = head[1].split('-').collect::<Vec<&str>>();
        if ends.len() != 2 {
            return r(RangeError::InvalidRange);
        }
        let start = if ends[0].is_empty() {
            None
        } else {
            match ends[0].parse::<u64>() {
                Ok(s) => Some(s),
                Err(_) => return r(RangeError::InvalidStart),
            }
        };
        let end = if ends[1].is_empty() {
            None
        } else {
            match ends[1].parse::<u64>() {
                Ok(e) => Some(e),
                Err(_) => return r(RangeError::InvalidEnd),
            }
        };
        Outcome::Success(Range { start, end })
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum StreamError {
    ApiError(ApiError),
    RangeError(RangeError),
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::internal("io_error", e.to_string())
    }
}

const CHUNK_SIZE: u64 = 1 << 20;


#[get("/media/<id>")]
pub async fn stream(id: String, range: Range, user: Result<UserGuard, AuthenticationError>, db: Connection<Db>) -> Result<ByteStream![Vec<u8>], Json<StreamError>> {
    let user = user.map_err(|e| Json(StreamError::ApiError(e.into())))?.user;
    if user.allowed(Permissions::READ_MEDIA) {
        return Err(Json(StreamError::ApiError(ApiError::internal("permission_error", "You don't have permission to read media".to_string()))));
    }
    let mut file = File::open(Path::new(&CONFIG.video_storage).join(id)).await.map_err(|e| Json(StreamError::ApiError(e.into())))?;
    let flen = file.metadata().await.map(|m| m.len()).map_err(|e| Json(StreamError::ApiError(e.into())))?;
    let start = if let Some(ref start) = range.start {
        if *start >= flen {
            return Err(Json(StreamError::RangeError(RangeError::InvalidStart)));
        } else {
            *start
        }
    } else {
        0
    };
    if let Some(ref end) = range.end {
        if *end >= flen {
            return Err(Json(StreamError::RangeError(RangeError::InvalidEnd)));
        } else {
            *end
        }
    } else {
        flen
    };

    let mut pos = start;
    let mut buf = Vec::with_capacity(CHUNK_SIZE as usize);
    Ok(ByteStream!{
        // use default chunk size:
        loop {
            if pos + CHUNK_SIZE > flen {
                let mut last_buf = vec![0; (flen - pos) as usize];
                file.read_exact(&mut last_buf).await.unwrap();
                yield last_buf;
            } else {
                pos += CHUNK_SIZE;
                file.read_exact(&mut buf).await.unwrap();
                yield buf.clone();
            }
        }
    })
}
