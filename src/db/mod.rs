pub mod entities;

use std::sync::OnceLock;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

use crate::{Error, db::entities::{score, skin, user}};

static DB: OnceLock<DatabaseConnection> = OnceLock::new();

pub async fn init_db() -> Result<(), Error> {
    let db_connection = Database::connect(std::env::var("DATABASE_URL").expect("missing DATABASE_URL")).await?;

    match DB.set(db_connection) {
        Ok(_) => return Ok(()),
        Err(_) => panic!("Database could not be initialized"),
    };
}

pub fn get_db() -> DatabaseConnection {
    DB.get().expect("Database is not initialized yet").clone()
}

pub async fn get_user_by_discord_id(discord_id: i64) -> Result<Option<user::Model>, Error> {
    Ok(user::Entity::find().filter(user::Column::DiscordId.eq(discord_id)).one(&get_db()).await?)
}

pub async fn get_user_by_discord_id_or_create(discord_id: i64, osu_id: i32) -> Result<user::Model, Error> {
    let user = match get_user_by_discord_id(discord_id).await? {
        Some(user) => user,
        None => {
            user::ActiveModel {
                discord_id: Set(discord_id),
                osu_id: Set(osu_id as i64),
                ..Default::default()
            }.insert(&get_db()).await?
        }
    };
    Ok(user)
}

pub async fn get_skin_by_identifier(user: user::Model, identifier: String) -> Result<Option<skin::Model>, Error> {
    Ok(skin::Entity::find()
        .filter(skin::Column::User.eq::<i64>(user.id as i64))
        .filter(skin::Column::Identifier.like(format!("%{}%", identifier)))
        .one(&get_db()).await?)
}

pub async fn has_score(score_reference: String) -> Result<bool, Error> {
    Ok(score::Entity::find_by_id(score_reference).count(&get_db()).await? > 0)
}

pub async fn insert_score(score_reference: String) -> Result<(), Error> {
    score::ActiveModel {
        identifier: Set(score_reference)
    }.insert(&get_db()).await?;
    Ok(())
}
