use rocket::request::FromRequest;
use rocket::form::{Form, FromForm};

pub(crate) struct Authorization(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorization {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let auth = request.headers().get_one("Authorization");
        match auth {
            Some(auth) => {
                let auth = auth.split_whitespace().collect::<Vec<&str>>();
                if auth.len() != 2 || auth[0] != "Bearer" {
                    return rocket::request::Outcome::Error((rocket::http::Status::Unauthorized, ()));
                }
                rocket::request::Outcome::Success(Authorization(auth[1].to_string()))
            },
            None => rocket::request::Outcome::Error((rocket::http::Status::Unauthorized, ()))
        }
    }
}

#[derive(FromForm)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

#[post("/login", data = "<form>", format = "json")]
pub(crate) async fn login(form: Form<LoginForm>) -> String {
    format!("{}:{}", form.username, form.password)
}
