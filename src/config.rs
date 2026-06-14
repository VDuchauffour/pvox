use std::fs;
use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Deserialize, Parser)]
#[command(name = "metron", about = "A k9s-like terminal UI for Proxmox VE")]
pub struct Config {
    #[arg(long, help = "Proxmox host URL")]
    #[serde(default)]
    pub host: Option<String>,

    #[arg(long, help = "API token ID (e.g. root@pam!metron)")]
    #[serde(default)]
    pub token_id: Option<String>,

    #[arg(long, help = "API token secret")]
    #[serde(default)]
    pub token: Option<String>,

    #[arg(long, help = "Allow insecure HTTPS (self-signed certs)")]
    #[serde(default)]
    pub insecure: bool,

    #[arg(long, help = "Data refresh interval in seconds", default_value = "5")]
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,

    #[arg(long, help = "Initial resource filter")]
    #[serde(default)]
    pub filter: Option<String>,

    #[arg(long, help = "Disable colors")]
    #[serde(default)]
    pub no_color: bool,

    #[arg(long, help = "Path to config file")]
    #[serde(skip)]
    pub config: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: None,
            token_id: None,
            token: None,
            insecure: false,
            refresh_interval: default_refresh_interval(),
            filter: None,
            no_color: false,
            config: None,
        }
    }
}

fn default_refresh_interval() -> u64 {
    5
}

impl Config {
    pub fn load(self) -> anyhow::Result<Config> {
        let mut cfg = if let Some(path) = self.config.as_ref() {
            let contents = fs::read_to_string(path)?;
            serde_yaml::from_str(&contents)?
        } else if let Ok(home) = std::env::var("HOME") {
            let default_path = PathBuf::from(home).join(".metron/config.yaml");
            if default_path.exists() {
                let contents = fs::read_to_string(&default_path)?;
                serde_yaml::from_str(&contents)?
            } else {
                Config::default()
            }
        } else {
            Config::default()
        };

        // CLI overrides file (CLI wins)
        if self.host.is_some() {
            cfg.host = self.host;
        }
        if self.token_id.is_some() {
            cfg.token_id = self.token_id;
        }
        if self.token.is_some() {
            cfg.token = self.token;
        }
        cfg.insecure = self.insecure;
        cfg.refresh_interval = self.refresh_interval;
        cfg.no_color = self.no_color;

        // METRON_TOKEN env var fallback
        if cfg.token.is_none() {
            if let Ok(token) = std::env::var("METRON_TOKEN") {
                cfg.token = Some(token);
            }
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_yaml_config_parsing() {
        let yaml = r#"
host: https://pve.example.com
token_id: root@pam!metron
token: secret123
insecure: true
refresh_interval: 10
no_color: true
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.host, Some("https://pve.example.com".to_string()));
        assert_eq!(config.token_id, Some("root@pam!metron".to_string()));
        assert_eq!(config.token, Some("secret123".to_string()));
        assert!(config.insecure);
        assert_eq!(config.refresh_interval, 10);
        assert!(config.no_color);
    }

    #[test]
    fn test_cli_overrides_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "host: https://file.example.com").unwrap();

        let args = Config {
            config: Some(tmp.path().to_path_buf()),
            host: Some("https://cli.example.com".to_string()),
            ..Default::default()
        };

        let cfg = args.load().unwrap();
        assert_eq!(cfg.host, Some("https://cli.example.com".to_string()));
    }

    #[test]
    fn test_metron_token_env_fallback() {
        unsafe {
            std::env::set_var("METRON_TOKEN", "env-token-123");
        }

        let args = Config {
            token: None,
            ..Default::default()
        };

        let cfg = args.load().unwrap();
        assert_eq!(cfg.token, Some("env-token-123".to_string()));

        unsafe {
            std::env::remove_var("METRON_TOKEN");
        }
    }

    #[test]
    fn test_default_refresh_interval() {
        let args = Config::default();
        assert_eq!(args.refresh_interval, 5);
    }

    #[test]
    fn test_default_insecure() {
        let args = Config::default();
        assert!(!args.insecure);
    }
}
