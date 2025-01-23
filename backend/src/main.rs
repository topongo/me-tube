#[macro_use]
extern crate rocket;

mod db;
mod authentication;
mod response;
mod user;
mod config;

use rocket::fs::FileServer;
use rocket_db_pools::Database;


#[get("/")]
fn index() -> &'static str {
    "MeTube"
}

#[launch]
fn rocket() -> _ {
    // check if config are initialized
    let _ = config::CONFIG.access_token_duration;

    rocket::build()
        .mount("/", routes![index])
        .mount("/api/auth", routes![
            authentication::login,
            authentication::register,
            authentication::me,
            authentication::refresh,
        ])
        .mount("/static", FileServer::from("static"))
        .attach(db::Db::init())
        // .attach(AdHoc::config::<config::MeTube>())
        // .attach(Template::fairing())
}
