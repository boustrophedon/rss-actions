use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Error, Result};

use serde::{Deserialize, Serialize};

use directories::ProjectDirs;

/// Configuration file for RSS Actions
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// The path to the rss-actions database file. 
    pub db_path: PathBuf,
}

impl Config {
    /// Creates a new config in the default directory, possibly creating the directory as well if
    /// it does not exist. Additionally creates the data directory in which the database is stored
    /// in by default.
    ///
    /// The default directory `$XDG_CONFIG_DIR/rss-actions/config.toml` or equivalent on other
    /// platforms.
    pub fn make_new() -> Result<Config> {
        let project_dirs = ProjectDirs::from("", "", "rss-actions")
            .ok_or_else(|| Error::msg("No home directory exists. Could not find config directory."))?;

        let mut db_path: PathBuf = project_dirs.data_dir().into();
        std::fs::create_dir_all(&db_path)
            .with_context(|| format!("Unable to create database dir: {:?}", &db_path))?;
        db_path.push("rss-actions.db");

        let cfg = Config {
            db_path,
        };

        let mut config_file: PathBuf = project_dirs.config_dir().into();
        config_file.push("config.toml");

        cfg.write_out(&config_file)
            .with_context(|| format!("Failed to write config file to {:?}", config_file))?;

        Ok(cfg)
    }

    /// Writes out the config file to the specified path. Will create intemediate directories if
    /// necessary.
    pub fn write_out(&self, config_file: &Path) -> Result<()> {
        if config_file.is_dir() {
            return Err(anyhow!("File path {:?} is existing directory, not file.", config_file));
        }
        let config_dir = config_file.parent()
            .ok_or_else(|| anyhow!("We should not hit this, but somehow couldn't get parent directory of \
                            non-directory {:?}", config_file))?;

        std::fs::create_dir_all(config_dir)
            .with_context(|| format!("Unable to create config dir: {:?}", config_dir))?;

        let cfg_file_data = toml::to_string(&self)
            .context("Could not serialize config to toml.")?;

        std::fs::write(config_file, cfg_file_data)
            .with_context(|| format!("Could not write config data to file: {:?}", config_file))?;

        Ok(())
    }

    /// Opens existing configuration file if there is one, or creates a new one with default
    /// values.
    ///
    /// If None is passed, uses the default location `$XDG_CONFIG_DIR/rss-actions/config.toml` or
    /// equivalent on other platforms.
    pub fn open(config_file: Option<&Path>) -> Result<Config> {
        if config_file.is_none() {
            return Config::make_new();
        }
        let config_file = config_file.unwrap();

        // Otherwise, parse existing file
        let config_data = std::fs::read_to_string(config_file)
            .with_context(|| format!("Failed to open config file: {:?}", config_file))?;
       
        toml::from_str(&config_data)
            .with_context(|| format!("Failed to read config file: {:?}", config_file))
    }
}
