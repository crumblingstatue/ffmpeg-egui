use {
    recently_used_list::RecentlyUsedList,
    serde::{Deserialize, Serialize},
    std::process::{Command, ExitStatus},
};

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub recently_used_list: RecentlyUsedList<String>,
}

impl Config {
    pub fn load_or_default() -> Self {
        let path = toml_path();
        match std::fs::read_to_string(&path) {
            Ok(string) => toml::from_str(&string).unwrap(),
            Err(e) => {
                eprintln!("Failed to load config: {e}. Creating default.");
                Self::default()
            }
        }
    }

    pub(crate) fn save(&self) -> anyhow::Result<()> {
        let string = toml::to_string_pretty(self)?;
        std::fs::create_dir_all(conf_dir_path())?;
        std::fs::write(toml_path(), string.as_bytes())?;
        Ok(())
    }
}

fn conf_dir_path() -> std::path::PathBuf {
    dirs::config_dir().unwrap().join("ffmfrog")
}

fn toml_path() -> std::path::PathBuf {
    conf_dir_path().join("config.toml")
}

pub(crate) fn shell_open() -> anyhow::Result<ExitStatus> {
    let path = toml_path();
    Ok(Command::new("xdg-open").arg(path).status()?)
}
