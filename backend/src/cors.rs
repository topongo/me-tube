use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
// use bruss_config::CONFIGS;

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
        // if let Some(ref origin) = CONFIGS.api.cors_allowed_origin {
        //     response.set_header(Header::new("Access-Control-Allow-Origin", origin));
        // }
        // if let Some(ref methods) = CONFIGS.api.cors_allowed_methods {
        //     response.set_header(Header::new("Access-Control-Allow-Methods", methods.join(", ")));
        // }
        // if let Some(ref headers) = CONFIGS.api.cors_allowed_headers {
        //     response.set_header(Header::new("Access-Control-Allow-Headers", headers.join(", ")));
        // }
        // if let Some(ref credentials) = CONFIGS.api.cors_allow_credentials {
        //     response.set_header(Header::new("Access-Control-Allow-Credentials", credentials.to_string()));
        // }
        response.set_header(Header::new("Access-Control-Allow-Origin", "http://127.0.0.1:8001"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "GET, POST"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "content-type, authorization, set-cookie"));
        // response.set_header(Header::new("Access-Control-Allow-Credentials", credentials.to_string()));
    }
}
