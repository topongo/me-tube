use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use crate::config::{CorsConfig, CONFIG};

pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        // make code crash if request is made in production
        let CorsConfig { ref allowed_origins, ref allowed_methods, ref allowed_headers } = CONFIG.cors;
        if allowed_origins.is_empty() {
            response.set_header(Header::new("Access-Control-Allow-Origin", allowed_origins.join(", ")));
        }
        if !allowed_methods.is_empty() {
            response.set_header(Header::new("Access-Control-Allow-Methods", allowed_methods.join(", ")));
        }
        if !allowed_headers.is_empty() {
            response.set_header(Header::new("Access-Control-Allow-Headers", allowed_headers.join(", ")));
        }
        // if !allow_credentials.is_empty() {
        //     response.set_header(Header::new("Access-Control-Allow-Credentials", credentials.to_string()));
        // }
        // response.set_header(Header::new("Access-Control-Allow-Origin", "http://127.0.0.1:8001"));
        // response.set_header(Header::new("Access-Control-Allow-Methods", "GET, POST"));
        // response.set_header(Header::new("Access-Control-Allow-Headers", "content-type, authorization, set-cookie"));
        // response.set_header(Header::new("Access-Control-Allow-Credentials", credentials.to_string()));
    }
}
