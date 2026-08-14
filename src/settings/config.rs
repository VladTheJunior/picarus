use anyhow::Result;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::error;

pub trait Config: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + Default + Sized {
    const FILE: &'static str;

    fn save(&self) -> Result<()> {
        let app = std::env::current_exe().expect("Could not determine application path");
        let mut config_dir = app
            .parent()
            .ok_or(anyhow!("Application path has no parent directory: {:?}", app))?
            .join("configs");
        fs::create_dir_all(&config_dir)?;
        config_dir.push(Self::FILE);
        fs::write(config_dir, toml::to_string(self)?)?;

        Ok(())
    }
    fn try_save(&self) -> bool {
        return match self.save() {
            Ok(_) => true,
            Err(e) => {
                error!(?self,  file = Self::FILE, error = %e, "failed to save");
                false
            }
        };
    }

    fn try_load() -> (Self, bool) {
        return match Self::load() {
            Ok(result) => (result, true),
            Err(e) => {
                error!(file = Self::FILE, error = %e, "failed to load");
                (Self::default(), false)
            }
        };
    }

    fn load() -> Result<Self> {
        let app = std::env::current_exe().expect("Could not determine application path");
        let config = app
            .parent()
            .ok_or(anyhow!("Application path has no parent directory: {:?}", app))?
            .join("configs")
            .join(Self::FILE);
        let f = fs::read_to_string(config)?;
        Ok(toml::from_str(&f)?)
    }
}
