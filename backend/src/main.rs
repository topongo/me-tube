
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
mod cors;
mod media;
mod like;

use rocket::fairing::AdHoc;
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

#[launch]
fn rocket() -> _ {
    // check if config are initialized
    config::CONFIG.check();
    rocket::build()
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
        .attach(cors::Cors)
    // TODO: add static FileServer only for debug_assertions
}
