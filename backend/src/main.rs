#[macro_use]
extern crate rocket;

mod db;
mod authentication;
mod response;
mod user;

use rocket::fs::FileServer;
use rocket_db_pools::Database;

#[get("/")]
fn index() -> &'static str {
    "MeTube"
}

#[launch]
fn rocker() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/api/auth", routes![
            authentication::login,
            authentication::register,
        ])
        .mount("/static", FileServer::from("static"))
        .attach(db::Db::init())
        // .attach(Template::fairing())
}
