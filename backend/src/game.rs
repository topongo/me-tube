use std::collections::HashSet;

use rocket::{futures::{StreamExt, TryStreamExt}, serde::json::Json};
use serde::{Deserialize, Serialize};
use rocket_db_pools::mongodb::{self, bson::{doc, oid::ObjectId, Document}, options::ReplaceOptions};

use crate::{authentication::{AuthenticationError, IsAdmin, UserGuard}, db::DBWrapper, response::{ApiErrorType, ApiResponder, ApiResponse}, user::{Permissions, User}};

#[derive(Serialize, Deserialize)]
pub(crate) struct Game {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    name: String,
}

impl DBWrapper {
    pub(crate) async fn add_game(&self, mut game: Game) -> Result<String, mongodb::error::Error> {
        if game.id.is_none() {
            game.id = Some(ObjectId::new().to_hex());
        }
        ObjectId::new();
        self
            .collection::<Game>(Self::GAMES)
            .insert_one(&game, None)
            .await
            .map(|_| game.id.unwrap())
    }

    pub(crate) async fn add_user_to_game(&self, game: &Game, user: &User) -> Result<(), mongodb::error::Error> {
        self
            .collection(Self::GAME_USERS)
            .replace_one(
                doc! {"game": game.id.as_ref().unwrap(), "user": user.username.clone()},
                doc! {"game": game.id.as_ref().unwrap(), "user": user.username.clone()},
                Some(ReplaceOptions::builder().upsert(true).build()),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn remove_user_from_game(&self, game: &Game, user: &User) -> Result<(), mongodb::error::Error> {
        self
            .collection::<()>(Self::GAME_USERS)
            .delete_one(doc! {"game": game.id.as_ref().unwrap(), "user": user.username.clone()}, None)
            .await
            .map(|_| ())
    }

    pub(crate) async fn is_user_in_game(&self, game: &str, user: &str) -> Result<bool, mongodb::error::Error> {
        self
            .collection::<()>(Self::GAME_USERS)
            .count_documents(
                doc! {"game": game, "user": user},
                None,
            )
            .await
            .map(|x| x > 0)
    }

    pub(crate) async fn get_games(&self) -> Result<Vec<Game>, mongodb::error::Error> {
        self
            .collection::<Game>(Self::GAMES)
            .find(doc!{}, None)
            .await?
            .try_collect()
            .await
    }

    pub(crate) async fn get_user_games_list(&self, user: &User) -> Result<HashSet<String>, mongodb::error::Error> {
        self
            .collection::<Document>(Self::GAME_USERS)
            .find(doc!{"user": &user.username}, None)
            .await?
            .map(|d| d.map(|d| d.get_str("game").unwrap().to_string()))
            .try_collect()
            .await
    }

    pub(crate) async fn get_user_games(&self, user: &User) -> Result<Vec<Game>, mongodb::error::Error> {
        self
            .collection::<Document>(Self::GAME_USERS)
            .aggregate(vec![
                doc!{"$match": {"user": &user.username}},
                doc!{"$lookup": {
                    "from": Self::GAMES,
                    "localField": "game",
                    "foreignField": "_id",
                    "as": "game",
                }},
                doc!{"$unwind": "$game"},
                doc!{"$replaceRoot": {"newRoot": "$game"}},
            ], None)
            .await?
            .map(|d| d.map(|d| mongodb::bson::from_document(d).unwrap()))
            .try_collect()
            .await
    }

    pub(crate) async fn get_game(&self, id: &str) -> Result<Option<Game>, mongodb::error::Error> {
        self
            .collection::<Game>(Self::GAMES)
            .find_one(doc!{"_id": id}, None)
            .await
    }
}

#[derive(Deserialize)]
pub(crate) struct AddForm {
    name: String,
}

impl From<AddForm> for Game {
    fn from(form: AddForm) -> Self {
        Game {
            id: None,
            name: form.name,
        }
    }
}

impl ApiResponse for AddResponse {}

#[derive(Serialize)]
pub(crate) struct AddResponse {
    id: String,
}

#[post("/", data = "<form>")]
pub(crate) async fn add(form: Json<AddForm>, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<AddResponse> {
    let user = user?.user;
    if !user.allowed(Permissions::ADD_GAME) {
        return AuthenticationError::InsufficientPermissions(Permissions::ADD_GAME).into();
    }
    let form = form.into_inner();
    let game: Game = form.into();
    let id = db.add_game(game).await?;
    AddResponse { id }.into()
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct GetResponse {
    games: Vec<Game>,
}

impl ApiResponse for GetResponse {}

// this gets all games
#[get("/")]
pub(crate) async fn list(user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<GetResponse> {
    let user = user?.user;
    if !user.allowed(Permissions::VIEW_GAMES) {
        return AuthenticationError::InsufficientPermissions(Permissions::VIEW_GAMES).into();
    }
    let games = db.get_games().await?;
    GetResponse { games }.into()
}

// list games for a user
#[get("/user/<username>")]
pub(crate) async fn list_user_games(username: Option<&str>, user: Result<UserGuard<()>, AuthenticationError>, db: DBWrapper) -> ApiResponder<GetResponse> {
    let user = user?.user;
    let target = username.unwrap_or(&user.username);
    if target != user.username && !user.allowed(Permissions::ADMIN) {
        return AuthenticationError::InsufficientPermissions(Permissions::ADMIN).into();
    }
    let games = db.get_user_games(&user).await?;
    GetResponse { games }.into()
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum GameUserError {
    UserNotFound,
    GameNotFound,
}

impl ApiErrorType for GameUserError {
    fn ty(&self) -> &'static str {
        match self {
            GameUserError::UserNotFound => "user_not_found",
            GameUserError::GameNotFound => "game_not_found",
        }
    }

    fn status(&self) -> rocket::http::Status {
        rocket::http::Status::NotFound
    }

    fn message(&self) -> String {
        match self {
            GameUserError::UserNotFound => "User not found".to_string(),
            GameUserError::GameNotFound => "Game not found".to_string(),
        }
    }
}

#[post("/<game>/<new_user>", format = "json")]
pub(crate) async fn add_user(game: String, new_user: &str, user: Result<UserGuard<IsAdmin>, AuthenticationError>, db: DBWrapper) -> ApiResponder<()> {
    let _ = user?;
    match db.get_game(&game).await? {
        Some(game) => {
            match db.get_user(new_user).await? {
                Some(u) => {
                    db.add_user_to_game(&game, &u).await?;
                    ApiResponder::Ok(())
                }
                None => ApiResponder::Err(GameUserError::UserNotFound.into()),
            }
        }
        None => ApiResponder::Err(GameUserError::GameNotFound.into()),
    }
}

#[delete("/<game>/<new_user>", format = "json")]
pub(crate) async fn remove_user(game: String, new_user: &str, user: Result<UserGuard<IsAdmin>, AuthenticationError>, db: DBWrapper) -> ApiResponder<()> {
    let _ = user?;
    match db.get_game(&game).await? {
        Some(game) => {
            match db.get_user(new_user).await? {
                Some(u) => {
                    db.remove_user_from_game(&game, &u).await?;
                    ApiResponder::Ok(())
                }
                None => ApiResponder::Err(GameUserError::UserNotFound.into()),
            }
        }
        None => ApiResponder::Err(GameUserError::GameNotFound.into()),
    }
}
