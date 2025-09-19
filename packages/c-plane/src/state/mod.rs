use crate::config::{Config, load_config};
use crate::errors::{AppError, DatabaseError};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::process;
use std::sync::OnceLock;

#[derive(Clone)]
pub struct State {
    pub db: DatabaseConnection,
    pub config: Config,
}

static STATE: OnceLock<State> = OnceLock::new();

pub async fn create_app_state() -> Result<State, AppError> {
    let config = load_config()?;
    let mut options = ConnectOptions::new(&config.database_url);
    options.sqlx_logging(true);

    let db = Database::connect(options)
        .await
        .map_err(|err| AppError::Database(DatabaseError::ConnectionFailed(err.to_string())))?;
    Ok(State { db, config })
}

pub fn get_app_state() -> State {
    match STATE.get() {
        Some(v) => v.clone(),
        None => {
            eprintln!("ERROR: get_app_state() called before initialization");
            let backtrace = std::backtrace::Backtrace::capture();
            eprintln!("{backtrace}");
            process::exit(0)
        }
    }
}
