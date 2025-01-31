use std::path::Path;

use rocket::{outcome::Outcome, request::FromRequest, response::{stream::ReaderStream, Responder}};
use rocket_db_pools::Connection;
use rocket::tokio::fs::File;

use crate::{authentication::{AuthenticationError, UserGuard}, db::Db};

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
    start: Option<usize>,
    end: Option<usize>,
}

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
        let head = match request.headers().get("range").next() {
            Some(h) => h,
            None => return Outcome::Error(RangeError::InvalidRange),
        };

        let head = head.split('=').collect::<Vec<&str>>();
        if head.len() != 2 || head[0] != "bytes" {
            return Outcome::Error(RangeError::InvalidRange);
        }
        let ends = head[1].split('-').collect::<Vec<&str>>();
        if ends.len() != 2 {
            return Outcome::Error(RangeError::InvalidRange);
        }
        let s = if ends[0].is_empty() {
            None
        } else {
            match ends[0].parse::<usize>() {
                Ok(s) => Some(s),
                Err(_) => return Outcome::Error(RangeError::InvalidStart),
            }
        };
        let e = if ends[1].is_empty() {
            None
        } else {
            match ends[1].parse::<usize>() {
                Ok(e) => Some(e),
                Err(_) => return Outcome::Error(RangeError::InvalidEnd),
            }
        };
        Outcome::Success(Range { start: s, end: e })
    }
}

#[get("/media/<id>")]
pub async fn media(id: String, range: Range, user: Result<UserGuard, AuthenticationError>, db: Connection<Db>) -> std::io::Result<ReaderStream![File]> {
    let user = user?.user;
    todo!();
}
