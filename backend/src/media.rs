use std::io::SeekFrom;

use rocket::{futures::Stream, http::ContentType, request::{FromRequest, Outcome}, response::{stream::{stream, ByteStream}, Responder}, serde::json::Json, tokio::io::{AsyncReadExt, AsyncSeekExt}, Response};
use rocket::tokio::fs::File;
use serde::Serialize;

use crate::{db::DBWrapper, response::{ApiError, ApiResponder}, video::Video};

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

#[derive(Debug)]
pub(crate) struct Range {
    start: Option<u64>,
    end: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[derive(Debug)]
pub(crate) enum RangeError {
    Format,
    Start,
    End,
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
            return r(RangeError::Format);
        }
        let ends = head[1].split('-').collect::<Vec<&str>>();
        if ends.len() != 2 {
            return r(RangeError::Format);
        }
        let start = if ends[0].is_empty() {
            None
        } else {
            match ends[0].parse::<u64>() {
                Ok(s) => Some(s),
                Err(_) => return r(RangeError::Start),
            }
        };
        let end = if ends[1].is_empty() {
            None
        } else {
            match ends[1].parse::<u64>() {
                Ok(e) => Some(e),
                Err(_) => return r(RangeError::End),
            }
        };
        Outcome::Success(Range { start, end })
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum StreamError {
    ApiError(ApiError),
    RangeError(RangeError),
    NotFound,
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::internal("io_error", e.to_string())
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for StreamError {
    fn respond_to(self, request: &'r rocket::Request) -> rocket::response::Result<'o> {
        let mut res = rocket::response::Response::build();
        match self {
            Self::ApiError(e) => {
                ApiResponder::Err::<()>(e).respond_to(request)
            }
            Self::RangeError(e) => {
                res
                    .status(rocket::http::Status::BadRequest)
                    .merge(Json(e).respond_to(request)?)
                    .ok()
            }
            Self::NotFound => {
                Self::ApiError(ApiError::not_found()).respond_to(request)
            }
        }
    }
}

const CHUNK_SIZE: u64 = 1 << 20;

pub(crate) struct MediaStream {
    range: Option<Range>,
    len: u64,
    file: rocket::tokio::fs::File,
    name: String,
}

// impl<S> MediaStream<S> {
//     fn stream_gen(self) -> ByteStream<impl Stream<Item = Vec<u8>>> {
//         ByteStream::from(self.stream)
//     }
// }

impl MediaStream {
    pub(crate) async fn from_video(range: Option<Range>, video: Video) -> Result<Self, StreamError> {
        let name = video.download_name();
        let file = File::open(video.file.unwrap_right().path()).await.map_err(|e| StreamError::ApiError(e.into()))?; 
        let flen = file.metadata().await.map(|m| m.len()).map_err(|e| StreamError::ApiError(e.into()))?;
        if let Some(ref range) = range {
            if let Some(ref start) = range.start {
                if *start >= flen {
                    return Err(StreamError::RangeError(RangeError::Start));
                }
            }
            if let Some(ref end) = range.end {
                if *end >= flen {
                    return Err(StreamError::RangeError(RangeError::End));
                }
            }
        }
        Ok(Self {
            range,
            len: flen,
            file,
            name,
        })
    }

    fn gen_stream(mut self) -> ByteStream<impl Stream<Item = Vec<u8>>> {
        let (mut pos, end) = match self.range {
            Some(r) => (r.start.unwrap_or(0u64), r.end.unwrap_or(self.len - 1) + 1),
            None => (0u64, self.len),
        };
        let mut buf = vec![0; CHUNK_SIZE as usize];
        ByteStream::from(stream! {
            // use default chunk size:
            if pos > 0 {
                self.file.seek(SeekFrom::Start(pos)).await.unwrap();
            }
            loop {
                if pos + CHUNK_SIZE > end {
                    let mut last_buf = vec![0; (end - pos) as usize];
                    self.file.read_exact(&mut last_buf).await.unwrap();
                    yield last_buf;
                    break;
                } else {
                    pos += CHUNK_SIZE;
                    self.file.read_exact(&mut buf).await.unwrap();
                    yield buf.clone();
                }
            }
        })
    }
}

impl<'r> Responder<'r, 'r> for MediaStream {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'r> {
        let mut res = Response::build();
        res
            .header(rocket::http::Header::new("Accept-Ranges", "bytes"));

        let ty = ContentType::from_extension(self.name.split('.').next_back().unwrap_or_default()).unwrap_or(ContentType::Binary);
        let length = match self.range {
            Some(ref r) => {
                res
                    .header(rocket::http::Header::new("Content-Range", format!("bytes {}-{}/{}", r.start.unwrap_or(0), r.end.unwrap_or(self.len - 1), self.len)));
                r.end.unwrap_or(self.len - 1) - r.start.unwrap_or(0) + 1
            }
            None => self.len,
        };
        let status = if self.range.is_some() {
            rocket::http::Status::PartialContent
        } else {
            rocket::http::Status::Ok
        };
        res
            .header(rocket::http::Header::new("Content-Disposition", format!("inline; filename=\"{}\"", self.name)))
            .merge(self.gen_stream().respond_to(request)?)
            .header(rocket::http::Header::new("Content-Length", length.to_string()))
            .status(status)
            .header(ty)
            .ok()
    }
}


#[get("/<token>")]
    pub async fn serve_file(
        token: &str,
        range: Option<Range>, 
        db: DBWrapper
    ) -> Result<MediaStream, StreamError> {
    let t = match db.get_video_token(token).await.map_err(|e| StreamError::ApiError(e.into()))? {
        // Some(t) => if !t.token.valid()&& false {
        Some(t) => if false {
            warn!("token is invalid: {:?}", t);
            return Err(StreamError::NotFound);
        } else { t },
        None => return Err(StreamError::NotFound),
    };
    let mut video = db.get_video_resolved(&t.video).await.map_err(|e| StreamError::ApiError(e.into()))?.ok_or(StreamError::NotFound)?;
    // resolve eventual converted video
    video.resolve_converted(&db).await.map_err(|e| StreamError::ApiError(e.into()))?;

    MediaStream::from_video(range, video).await
}

