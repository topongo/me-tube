use rocket::{futures::TryStreamExt, serde::json::Json};
use serde::{Deserialize, Serialize};
use rocket_db_pools::{mongodb::{self, bson::{doc, oid::ObjectId}}, Connection};

use crate::{authentication::{AuthenticationError, UserGuard}, db::{DBWrapper, Db}, response::{ApiResponder, ApiResponse}, user::Permissions};

#[derive(Serialize, Deserialize)]
pub(crate) struct Game {
    #[serde(rename = "_id")]
    id: Option<String>,
    name: String,
}

impl DBWrapper {
    pub(crate) async fn add_game(&self, mut game: Game) -> Result<String, mongodb::error::Error> {
        if game.id.is_none() {
            game.id = Some(ObjectId::new().to_hex());
        }
        ObjectId::new();
        self.database()
            .collection::<Game>("games")
            .insert_one(&game, None)
            .await
            .map(|_| game.id.unwrap())
    }

    pub(crate) async fn get_games(&self, games: Vec<String>) -> Result<Vec<Game>, mongodb::error::Error> {
        let d = if games.is_empty() {
            doc!{}
        } else {
            doc!{"_id": {"$in": games}}
        };
        self.database()
            .collection::<Game>("games")
            .find(d, None)
            .await?
            .try_collect()
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

#[post("/add", data = "<form>")]
pub(crate) async fn add(form: Json<AddForm>, user: Result<UserGuard, AuthenticationError>, db: Connection<Db>) -> ApiResponder<AddResponse> {
    let user = user?.user;
    if !user.allowed(Permissions::ADD_GAME) {
        return AuthenticationError::InsufficientPermissions.into();
    }
    let form = form.into_inner();
    let game: Game = form.into();
    let db = DBWrapper::new(db.into_inner());
    let id = db.add_game(game).await?;
    AddResponse { id }.into()
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct GameResponse {
    games: Vec<Game>,
}

impl ApiResponse for GameResponse {}

#[get("/")]
pub(crate) async fn list(user: Result<UserGuard, AuthenticationError>, db: Connection<Db>) -> ApiResponder<GameResponse> {
    let user = user?.user;
    if !user.allowed(Permissions::VIEW_GAME) {
        return AuthenticationError::InsufficientPermissions.into();
    }
    let db = DBWrapper::new(db.into_inner());
    let games = db.get_games(vec![]).await?;
    GameResponse { games }.into()
}

