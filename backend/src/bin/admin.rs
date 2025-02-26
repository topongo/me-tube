use std::io::Write;

use me_tube::{self, db::DBWrapper, Permissions, User};
use clap::Parser;
use rocket::figment::providers::Serialized;

#[derive(Parser)]
enum Command {
    #[clap(name = "create-admin")]
    CreateAdmin {
        #[clap(short, long)]
        username: Option<String>,
        #[clap(short, long)]
        password: Option<String>,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::parse();
    let figment = rocket::Config::figment().select("default").focus("databases.metube")
        .join(Serialized::default("max_connections", 4))
        .join(Serialized::default("connect_timeout", 1));
    let config = figment.extract::<rocket_db_pools::Config>().unwrap();
    let db = rocket_db_pools::mongodb::Client::with_uri_str(&config.url).await?;
    let db = DBWrapper::new(db);
    db.database();
    match command {
        Command::CreateAdmin { username, password } => {
            let username = match username {
                Some(username) => username,
                None => {
                    print!("Username: ");
                    std::io::stdout().flush()?;
                    let mut username = String::new();
                    std::io::stdin().read_line(&mut username)?;
                    username.trim().to_string()
                }
            };
            // check if user exists
            match db.get_user(&username).await? {
               Some(_) => {
                   eprintln!("User already exists");
                   std::process::exit(1);
               } 
               None => { /* ok, username available */ }
            }
            let password = match password {
                Some(password) => password,
                None => {
                    let p = rpassword::prompt_password("Password: ")?;
                    let p2 = rpassword::prompt_password("Confirm password: ")?;
                    if p != p2 {
                        eprintln!("Passwords do not match");
                        std::process::exit(1);
                    }
                    p
                }
            };
            User::validate(Some(&username), Some(&password))?;
            let mut new_user = User::create(username, password);
            new_user.push_permissions(Permissions::ADMIN);
            db.add_user(new_user).await?
        }
    }
    Ok(())
}
