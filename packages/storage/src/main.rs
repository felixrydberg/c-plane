use c_plane_storage::{StorageService, config::Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    StorageService::from_config(Config::from_env()?)
        .await?
        .serve()
        .await?;
    Ok(())
}
