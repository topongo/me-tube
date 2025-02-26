
#![feature(try_trait_v2)]

#[macro_use]
extern crate rocket;

pub mod db;
mod authentication;
mod response;
mod user;
mod config;
mod video;
mod game;
mod cors;
mod media;
mod like;

pub use config::CONFIG;
pub use user::{User, Permissions};

use rocket::{fairing::AdHoc, fs::FileServer};
use rocket_db_pools::Database;


#[get("/")]
fn index() -> &'static str {
    "MeTube"
}

#[options("/<_..>")]
pub fn options() {
    // CORS support
    // intentionally empty
}

pub fn rocket() -> rocket::Rocket<rocket::Build> {
    // check if config are initialized
    config::CONFIG.check();
    let build = rocket::build()
        .mount("/", routes![index])
        .mount("/api", routes![options])
        .mount("/api/auth", routes![
            authentication::login,
            authentication::refresh,
        ])
        .mount("/api/user", routes![
            user::me,
            user::post,
            user::patch,
            user::delete,
            user::list,
            user::permissions,
        ])
        .mount("/api/video", routes![
            video::upload,
            video::list,
            video::list_file,
            video::thumb,
            video::get_token,
            video::delete,
            video::update,
            like::add,
            like::delete,
            like::video,
        ])
        .mount("/api/game", routes![
            game::add,
            game::list,
            game::add_user,
            game::remove_user,
            game::list_user_games,
        ])
        .mount("/api/media", routes![
            media::serve_file,
        ])
        .mount("/api/like", routes![
            like::user,
        ])
        .mount("/share", routes![
            video::share::get,
        ])
        .attach(db::Db::init())
        .attach(AdHoc::try_on_ignite("MeTube db init", |rocket| async { db::DBWrapper::constraints_fairing(rocket).await }))
        .attach(cors::Cors);

    #[cfg(debug_assertions)]
    let build = build
        .mount("/static", FileServer::from("static"));

    build
}
