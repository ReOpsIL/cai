use std::fs;
use serde::{Serialize, Deserialize};
use directories::UserDirs;
use std::io::Read;
use toml_edit;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub model: String,
}

pub fn load_configuration() -> Result<Config, Box<dyn std::error::Error>> {
    let user_dirs = UserDirs::new().expect("Could not find user directories");
    let config_path = user_dirs.home_dir().join("cai.conf");

    let config: Config = if config_path.exists() {
        let mut config_file = fs::File::open(&config_path)?;
        let mut config_string = String::new();
        config_file.read_to_string(&mut config_string)?;
        toml_edit::de::from_str(&config_string)?
    } else {
        let default_config = Config {
            model: "google/gemini-2.0-flash-exp:free".to_string(),
        };
        let toml = toml::to_string(&default_config)?;
        fs::write(&config_path, toml)?;
        default_config
    };

    Ok(config)
}

pub fn save_configuration(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let user_dirs = UserDirs::new().expect("Could not find user directories");
    let config_path = user_dirs.home_dir().join("cai.conf");
    let toml = toml_edit::ser::to_string(config)?;
    fs::write(&config_path, toml)?;
    Ok(())
}