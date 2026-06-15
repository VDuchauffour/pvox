use std::fs;
use std::path::{Path, PathBuf};

use clap::builder::{
    Styles,
    styling::{AnsiColor, Effects},
};
use clap::{Parser, ValueEnum};
use serde::Deserialize;

pub fn cargo_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .placeholder(AnsiColor::Cyan.on_default())
        .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
        .valid(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
}

/// UI color theme. Replaces the old `--no-color` boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeKind {
    /// Full color palette.
    #[default]
    Default,
    /// Disable colors (monochrome output).
    NoColor,
}

/// Command-line arguments. Flat by design; values provided here take
/// precedence over the config file.
#[derive(Debug, Parser)]
#[command(name = "p9s", about = "A k9s-like terminal UI for Proxmox VE", styles = cargo_styles())]
pub struct Cli {
    #[arg(long, help = "Proxmox host URL")]
    pub host: Option<String>,

    #[arg(long, help = "API token ID (e.g. root@pam!p9s)")]
    pub token_id: Option<String>,

    #[arg(long, help = "API token secret")]
    pub secret: Option<String>,

    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        help = "Allow insecure HTTPS (self-signed certs)"
    )]
    pub insecure: Option<bool>,

    #[arg(long, help = "Path to config file")]
    pub config: Option<PathBuf>,
}

/// On-disk config file. Mirrors the published JSON schema
/// (`schema/config.schema.json`).
#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    connection: ConnectionSection,
    #[serde(default)]
    ui: UiSection,
    #[serde(default)]
    refresh_interval: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct ConnectionSection {
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    token_id: Option<String>,
    #[serde(default)]
    secret: Option<String>,
    #[serde(default)]
    insecure: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct UiSection {
    #[serde(default)]
    theme: Option<ThemeKind>,
}

/// Fully resolved runtime configuration consumed by the app.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: Option<String>,
    pub token_id: Option<String>,
    pub secret: Option<String>,
    pub insecure: bool,
    pub refresh_interval: u64,
    pub theme: ThemeKind,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: None,
            token_id: None,
            secret: None,
            insecure: false,
            refresh_interval: default_refresh_interval(),
            theme: ThemeKind::Default,
        }
    }
}

impl Config {
    /// Whether colors are disabled.
    pub fn no_color(&self) -> bool {
        self.theme == ThemeKind::NoColor
    }
}

fn default_refresh_interval() -> u64 {
    5
}

fn default_config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/p9s/config.yaml"))
}

const CONFIG_SCHEMA: &str = include_str!("../schema/config.schema.json");

fn validate_against_schema(value: &serde_json::Value) -> anyhow::Result<()> {
    let schema: serde_json::Value =
        serde_json::from_str(CONFIG_SCHEMA).expect("embedded config schema is valid JSON");
    let validator = jsonschema::draft7::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("embedded config schema is a valid JSON Schema");

    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| format!("  - {} (at `{}`)", error, error.instance_path()))
        .collect();

    if !errors.is_empty() {
        anyhow::bail!("config file does not match schema:\n{}", errors.join("\n"));
    }

    Ok(())
}

fn read_file_config(path: Option<&Path>) -> anyhow::Result<FileConfig> {
    let resolved = match path {
        Some(p) => Some(p.to_path_buf()),
        None => default_config_path(),
    };

    if let Some(p) = resolved
        && p.exists()
    {
        let contents = fs::read_to_string(&p)?;
        let value: serde_json::Value = serde_yaml::from_str(&contents)?;
        if value.is_null() {
            return Ok(FileConfig::default());
        }
        validate_against_schema(&value)?;
        return Ok(serde_json::from_value(value)?);
    }

    Ok(FileConfig::default())
}

impl Cli {
    /// Resolve final configuration: load the file (if present), then apply
    /// CLI overrides on top. CLI values take precedence over file values.
    pub fn load(self) -> anyhow::Result<Config> {
        let FileConfig {
            connection,
            ui,
            refresh_interval,
        } = read_file_config(self.config.as_deref())?;

        let mut cfg = Config {
            host: connection.host,
            token_id: connection.token_id,
            secret: connection.secret,
            insecure: connection.insecure.unwrap_or(false),
            refresh_interval: refresh_interval.unwrap_or_else(default_refresh_interval),
            theme: ui.theme.unwrap_or_default(),
        };

        // CLI overrides — only when a value was actually provided.
        if self.host.is_some() {
            cfg.host = self.host;
        }
        if self.token_id.is_some() {
            cfg.token_id = self.token_id;
        }
        if self.secret.is_some() {
            cfg.secret = self.secret;
        }
        if let Some(insecure) = self.insecure {
            cfg.insecure = insecure;
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn cli_with_config(config: PathBuf) -> Cli {
        Cli {
            host: None,
            token_id: None,
            secret: None,
            insecure: None,
            config: Some(config),
        }
    }

    #[test]
    fn test_yaml_config_parsing() {
        let yaml = r#"
connection:
  host: https://pve.example.com
  token_id: root@pam!p9s
  secret: secret123
  insecure: true
ui:
  theme: no-color
refresh_interval: 10
"#;
        let file: FileConfig = serde_yaml::from_str(yaml).unwrap();
        let conn = file.connection;
        assert_eq!(conn.host, Some("https://pve.example.com".to_string()));
        assert_eq!(conn.token_id, Some("root@pam!p9s".to_string()));
        assert_eq!(conn.secret, Some("secret123".to_string()));
        assert_eq!(conn.insecure, Some(true));
        assert_eq!(file.ui.theme, Some(ThemeKind::NoColor));
        assert_eq!(file.refresh_interval, Some(10));
    }

    #[test]
    fn test_schema_rejects_unknown_connection_field() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "connection:\n  hostname: https://pve.example.com").unwrap();

        let args = cli_with_config(tmp.path().to_path_buf());
        assert!(args.load().is_err());
    }

    #[test]
    fn test_cli_overrides_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "connection:\n  host: https://file.example.com").unwrap();

        let args = Cli {
            host: Some("https://cli.example.com".to_string()),
            ..cli_with_config(tmp.path().to_path_buf())
        };

        let cfg = args.load().unwrap();
        assert_eq!(cfg.host, Some("https://cli.example.com".to_string()));
    }

    #[test]
    fn test_file_values_used_when_no_cli() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            "connection:\n  host: https://file.example.com\n  insecure: true\nui:\n  theme: no-color\nrefresh_interval: 20"
        )
        .unwrap();

        let args = cli_with_config(tmp.path().to_path_buf());

        let cfg = args.load().unwrap();
        assert_eq!(cfg.host, Some("https://file.example.com".to_string()));
        assert!(cfg.insecure);
        assert_eq!(cfg.refresh_interval, 20);
        assert_eq!(cfg.theme, ThemeKind::NoColor);
        assert!(cfg.no_color());
    }

    #[test]
    fn test_cli_insecure_false_overrides_file_true() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "connection:\n  insecure: true").unwrap();

        let args = Cli {
            insecure: Some(false),
            ..cli_with_config(tmp.path().to_path_buf())
        };

        let cfg = args.load().unwrap();
        assert!(!cfg.insecure);
    }

    #[test]
    fn test_cli_insecure_true_overrides_file_false() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "connection:\n  insecure: false").unwrap();

        let args = Cli {
            insecure: Some(true),
            ..cli_with_config(tmp.path().to_path_buf())
        };

        let cfg = args.load().unwrap();
        assert!(cfg.insecure);
    }

    #[test]
    fn test_missing_file_uses_defaults() {
        let args = cli_with_config(PathBuf::from("/nonexistent/p9s/config.yaml"));

        let cfg = args.load().unwrap();
        assert_eq!(cfg.host, None);
        assert_eq!(cfg.refresh_interval, 5);
        assert_eq!(cfg.theme, ThemeKind::Default);
        assert!(!cfg.no_color());
    }

    #[test]
    fn test_embedded_schema_is_valid() {
        let schema: serde_json::Value = serde_json::from_str(CONFIG_SCHEMA).unwrap();
        assert!(jsonschema::meta::is_valid(&schema));
    }

    #[test]
    fn test_schema_rejects_refresh_interval_below_minimum() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "refresh_interval: 0").unwrap();

        let args = cli_with_config(tmp.path().to_path_buf());
        let err = args.load().unwrap_err().to_string();
        assert!(err.contains("schema"), "unexpected error: {err}");
        assert!(err.contains("refresh_interval"), "unexpected error: {err}");
    }

    #[test]
    fn test_schema_rejects_unknown_theme() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "ui:\n  theme: rainbow").unwrap();

        let args = cli_with_config(tmp.path().to_path_buf());
        assert!(args.load().is_err());
    }

    #[test]
    fn test_schema_rejects_unknown_top_level_key() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "connexion:\n  host: https://pve.example.com").unwrap();

        let args = cli_with_config(tmp.path().to_path_buf());
        assert!(args.load().is_err());
    }

    #[test]
    fn test_empty_file_uses_defaults() {
        let tmp = tempfile::NamedTempFile::new().unwrap();

        let args = cli_with_config(tmp.path().to_path_buf());
        let cfg = args.load().unwrap();
        assert_eq!(cfg.refresh_interval, 5);
        assert_eq!(cfg.theme, ThemeKind::Default);
    }

    #[test]
    fn test_default_refresh_interval() {
        let cfg = Config::default();
        assert_eq!(cfg.refresh_interval, 5);
    }

    #[test]
    fn test_default_insecure() {
        let cfg = Config::default();
        assert!(!cfg.insecure);
    }
}
