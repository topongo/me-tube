
#![feature(try_trait_v2)]

#[macro_use]
extern crate rocket;

mod db;
mod authentication;
mod response;
mod user;
mod config;
mod video;
mod game;

use rocket::fs::FileServer;
use rocket_db_pools::Database;


#[get("/")]
fn index() -> &'static str {
    "MeTube"
}

#[launch]
fn rocket() -> _ {
    // check if config are initialized
    config::CONFIG.check();

    rocket::build()
        .mount("/", routes![index])
        .mount("/api/auth", routes![
            authentication::login,
            authentication::register,
            authentication::me,
            authentication::refresh,
        ])
        .mount("/api/video", routes![
            video::upload,
            video::list,
            video::list_file,
            video::thumb,
        ])
        .mount("/api/game", routes![
            game::add,
            game::list,
        ])
        .mount("/static", FileServer::from("static"))
        .attach(db::Db::init())
        // .attach(AdHoc::config::<config::MeTube>())
        // .attach(Template::fairing())
}
