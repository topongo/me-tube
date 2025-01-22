use rocket_db_pools::Database;
use rocket_db_pools::mongodb::Client;

#[derive(Database)]
#[database("mongodb")]
pub struct Db(Client);
