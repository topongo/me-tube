#[macro_use] extern crate rocket;

mod db;
mod authentication;

use rocket::fs::FileServer;
use rocket_dyn_templates::{Template, context};

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {
        title: "MeTube", 
    })
}

#[launch]
fn rocker() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/api/auth", routes![
            authentication::login,
        ])
        .mount("/static", FileServer::from("static"))
        .attach(Template::fairing())
}
