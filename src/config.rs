use std::{env, net::SocketAddr};

use anyhow::{anyhow, Context, Result};

#[derive(Clone, Debug)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub listen_addr: SocketAddr,
    pub database_url: String,
    pub max_db_connections: u32,
    pub basic_auth: Option<BasicAuth>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let listen_addr = env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse()
            .context("LISTEN_ADDR must be a valid socket address, e.g. 0.0.0.0:8080")?;

        let database_url =
            env::var("DATABASE_URL").map_err(|_| anyhow!("DATABASE_URL is required"))?;

        let max_db_connections = env::var("DB_MAX_CONNECTIONS")
            .ok()
            .map(|v| {
                v.parse::<u32>()
                    .context("DB_MAX_CONNECTIONS must be a positive integer")
            })
            .transpose()?
            .unwrap_or(20);

        let username = env::var("HTTP_BASIC_USERNAME").ok();
        let password = env::var("HTTP_BASIC_PASSWORD").ok();

        let basic_auth = match (username, password) {
            (Some(username), Some(password)) => Some(BasicAuth { username, password }),
            (None, None) => None,
            _ => {
                return Err(anyhow!(
                    "Set both HTTP_BASIC_USERNAME and HTTP_BASIC_PASSWORD, or neither"
                ))
            }
        };

        Ok(Self {
            listen_addr,
            database_url,
            max_db_connections,
            basic_auth,
        })
    }
}
