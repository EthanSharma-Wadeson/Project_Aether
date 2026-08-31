use aether_control_plane::config::Config;
use aether_control_plane::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    aether_control_plane::run(config).await
}
