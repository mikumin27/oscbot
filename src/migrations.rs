use std::{fs::File, io::Write, path::Path};

use sqlx::SqlitePool;

use crate::Error;

pub async fn update_migrations() -> Result<(), Error> {
    if !Path::new("app.db").exists() {
        File::create("app.db").unwrap().flush()?;
    }
    
    let pool = SqlitePool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must exist")).await?;

    // runs pending migrations from ./migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(())
}