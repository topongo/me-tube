#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    me_tube::rocket()
}
