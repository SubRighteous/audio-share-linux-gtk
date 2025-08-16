use std::{fs,io};
use directories::ProjectDirs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize };

use crate::audioshare;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub audio_endpoint: String,
    pub audio_encoding: String,
    pub server_ip: String,
    pub server_port: u16,
    pub minimize_on_exit: bool,
    pub auto_start_server: bool,
    pub keep_last_state: bool,
    pub last_server_state: bool,
    pub notification_error: bool,
    pub notification_device_connect: bool,
    pub notification_device_disconnect: bool,
}

pub fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "subrighteous", "AudioShareGTK")
        .map(|dirs| dirs.config_dir().join("config.json"))
}

pub fn load_or_create_config() -> io::Result<AppConfig> {
    let path = get_config_path().expect("No valid config path available");

    if path.exists() {
        let data = fs::read_to_string(&path)?;
        let config: AppConfig = serde_json::from_str(&data)?;
        Ok(config)
    } else {

        let audio_endpoint_name: String;
        let audio_encoding_name: String;

        let server_ip: String = audioshare::get_local_ipv4();

        if let Some((_, _id, name)) = audioshare::get_default_endpoint() {
            audio_endpoint_name = name;
            //println!("Default endpoint -> id: {}, name: {}", id, name);
        }else{
            audio_endpoint_name = String::new();
        }

        if let Some((_,desc)) = audioshare::get_default_encoding(){
            audio_encoding_name = desc;
        }else{
            audio_encoding_name = String::new();
        }

        let config = AppConfig {
            audio_endpoint: audio_endpoint_name.to_string(),
            audio_encoding: audio_encoding_name.to_string(),
            server_ip: server_ip.to_string(),
            server_port: 65530,
            minimize_on_exit: false,
            auto_start_server: false,
            keep_last_state: false,
            last_server_state: false,
            notification_error: true,
            notification_device_connect: true,
            notification_device_disconnect: true,
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = serde_json::to_string_pretty(&config)?;
        fs::write(&path, data)?;
        Ok(config)
    }
}

pub fn save_config(config: &AppConfig) -> io::Result<()> {
    let path = get_config_path().expect("No valid config path available");
    let data = serde_json::to_string_pretty(config)?;
    fs::write(path, data)?;
    Ok(())
}
