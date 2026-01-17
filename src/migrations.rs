use std::{fs::File, io::Write, path::Path};

use sqlx::SqlitePool;

use crate::Error;

pub async fn update_migrations() -> Result<(), Error> {
    let database_path = std::env::var("DATABASE_URL").expect("DATABASE_URL must exist");
    let stripped_database_path = database_path.strip_prefix("sqlite://").unwrap();
    if !Path::new(stripped_database_path).exists() {
        File::create(stripped_database_path).unwrap().flush()?;
    }
    
    let pool = SqlitePool::connect(&database_path).await?;

    // runs pending migrations from ./migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(())
}