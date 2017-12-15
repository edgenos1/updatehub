/*
 * UpdateHub
 * Copyright (C) 2017
 * O.S. Systems Sofware LTDA: contact@ossystems.com.br
 *
 * SPDX-License-Identifier:     GPL-2.0
 */

use serde_ini;

extern crate chrono;
use self::chrono::{DateTime, Utc};

use std::io;
use std::path::Path;
use std::path::PathBuf;

use de_helpers::bool_from_str;
use se_helpers::bool_to_string;

#[derive(Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RuntimeSettings {
    pub polling: RuntimePolling,
    pub update: RuntimeUpdate,
    #[serde(skip)]
    path: PathBuf,
}

impl RuntimeSettings {
    pub fn new() -> Self {
        RuntimeSettings::default()
    }

    pub fn load(mut self, path: &str) -> Result<Self, RuntimeSettingsError> {
        use std::fs::File;
        use std::io::Read;

        let path = Path::new(path);

        if path.exists() {
            info!(
                "Loading runtime settings from '{}'...",
                path.to_string_lossy()
            );

            let mut content = String::new();
            File::open(path)?.read_to_string(&mut content)?;
            self = self.parse(&content)?;
        } else {
            debug!(
                "Runtime settings file {} does not exists.",
                path.to_string_lossy()
            );
            info!("Using default runtime settings...");
        }

        self.path = path.to_path_buf();
        Ok(self)
    }

    fn parse(self, content: &str) -> Result<Self, RuntimeSettingsError> {
        Ok(serde_ini::from_str::<RuntimeSettings>(content)?)
    }

    pub fn save(&self) -> Result<usize, RuntimeSettingsError> {
        use std::fs::File;
        use std::io::Write;

        debug!(
            "Saving runtime settings from '{}'...",
            &self.path.to_string_lossy()
        );

        Ok(File::create(&self.path)?.write(
            &self.serialize()?.as_bytes(),
        )?)
    }

    fn serialize(&self) -> Result<String, RuntimeSettingsError> {
        Ok(serde_ini::to_string(&self)?)
    }
}

#[derive(Debug)]
pub enum RuntimeSettingsError {
    Io(io::Error),
    IniDeserialize(serde_ini::de::Error),
    IniSerialize(serde_ini::ser::Error),
}

impl From<io::Error> for RuntimeSettingsError {
    fn from(err: io::Error) -> RuntimeSettingsError {
        RuntimeSettingsError::Io(err)
    }
}

impl From<serde_ini::de::Error> for RuntimeSettingsError {
    fn from(err: serde_ini::de::Error) -> RuntimeSettingsError {
        RuntimeSettingsError::IniDeserialize(err)
    }
}

impl From<serde_ini::ser::Error> for RuntimeSettingsError {
    fn from(err: serde_ini::ser::Error) -> RuntimeSettingsError {
        RuntimeSettingsError::IniSerialize(err)
    }
}

#[derive(Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RuntimePolling {
    #[serde(rename = "LastPoll")]
    pub last: DateTime<Utc>,
    #[serde(rename = "FirstPoll")]
    pub first: DateTime<Utc>,
    pub extra_interval: usize,
    pub retries: usize,
    #[serde(rename = "ProbeASAP")]
    #[serde(deserialize_with = "bool_from_str")]
    #[serde(serialize_with = "bool_to_string")]
    pub now: bool,
}

impl Default for RuntimePolling {
    fn default() -> Self {
        RuntimePolling {
            last: Utc::now(),
            first: Utc::now(),
            extra_interval: 0,
            retries: 0,
            now: false,
        }
    }
}

#[derive(Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RuntimeUpdate {
    #[serde(rename = "UpgradeToInstallation")]
    pub upgrading_to: i8,
}

impl Default for RuntimeUpdate {
    fn default() -> Self {
        RuntimeUpdate { upgrading_to: -1 }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn de() {
        let ini = r"
[Polling]
LastPoll=2017-01-01T00:00:00Z
FirstPoll=2017-02-02T00:00:00Z
ExtraInterval=4
Retries=5
ProbeASAP=false

[Update]
UpgradeToInstallation=1
";
        let expected = RuntimeSettings {
            polling: RuntimePolling {
                last: "2017-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
                first: "2017-02-02T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
                extra_interval: 4,
                retries: 5,
                now: false,
            },
            update: RuntimeUpdate { upgrading_to: 1 },
            ..Default::default()
        };

        assert!(
            serde_ini::from_str::<RuntimeSettings>(&ini)
                .map_err(|e| println!("{}", e))
                .as_ref()
                .ok() == Some(&expected)
        );
        assert!(RuntimeSettings::new().parse(&ini).as_ref().ok() == Some(&expected));
    }

    #[test]
    fn default() {
        let settings = RuntimeSettings::new();
        let expected = RuntimeSettings {
            polling: RuntimePolling {
                last: settings.polling.last,
                first: settings.polling.first,
                extra_interval: 0,
                retries: 0,
                now: false,
            },
            update: RuntimeUpdate { upgrading_to: -1 },
            path: PathBuf::new(),
        };

        assert!(Some(settings) == Some(expected));
    }

    #[test]
    fn ser() {
        let settings = RuntimeSettings {
            polling: RuntimePolling {
                last: "2017-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
                first: "2017-02-02T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
                extra_interval: 4,
                retries: 5,
                now: false,
            },
            update: RuntimeUpdate { upgrading_to: 1 },
            ..Default::default()
        };


        assert!(
            serde_ini::from_str(&settings.serialize().ok().unwrap())
                .map_err(|e| println!("{}", e))
                .ok() == Some(settings)
        );
    }

    #[test]
    fn load_and_save() {
        use mktemp::Temp;

        let settings_file = Temp::new_file().unwrap().to_path_buf();

        let mut settings = RuntimeSettings::new()
            .load(&settings_file.to_str().unwrap())
            .unwrap();

        assert!(settings.polling.now == false);
        settings.polling.now = true;

        assert!(settings.polling.now == true);
        settings.save().expect(
            "Failed to save the runtime settings",
        );

        let new_settings = RuntimeSettings::new()
            .load(&settings_file.to_str().unwrap())
            .unwrap();

        assert!(&settings.update == &new_settings.update);
    }
}
