//! Command line configuration.

use anyhow::{Context, Result, bail};
use clap::Parser;
use std::path::PathBuf;

/// Command line configuration for Gitkyl.
#[derive(Debug, Clone, Parser)]
#[command(name = "gitkyl", version, about, long_about = None)]
pub struct Config {
    /// Repository path
    #[arg(default_value = ".")]
    pub repo: PathBuf,

    /// Output directory
    #[arg(short, long, default_value = "dist")]
    pub output: PathBuf,

    /// Project name
    #[arg(long)]
    pub name: Option<String>,

    /// Project owner
    #[arg(long)]
    pub owner: Option<String>,

    /// Syntax highlighting theme (Catppuccin-Latte, Catppuccin-Mocha, etc.)
    #[arg(long, default_value = "Catppuccin-Latte")]
    pub theme: String,
}

impl Config {
    /// Parses configuration from command line arguments.
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    /// Validates configuration.
    ///
    /// # Errors
    ///
    /// Returns error if repository path does not exist.
    pub fn validate(&self) -> Result<()> {
        if !self.repo.exists() {
            bail!("Repository path does not exist: {}", self.repo.display());
        }

        Ok(())
    }

    /// Returns project name from configuration or repository directory.
    ///
    /// # Errors
    ///
    /// Returns error if repository path has no name component or contains invalid UTF8.
    pub fn project_name(&self) -> Result<String> {
        if let Some(name) = &self.name {
            return Ok(name.clone());
        }

        let path = self
            .repo
            .canonicalize()
            .unwrap_or_else(|_| self.repo.clone());

        path.file_name()
            .and_then(|n| n.to_str())
            .with_context(|| format!("Cannot extract project name from path: {}", path.display()))
            .map(String::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_project_name_with_explicit_name() {
        // Arrange
        let config = Config {
            repo: PathBuf::from("."),
            output: PathBuf::from("dist"),
            name: Some("ExplicitName".to_string()),
            owner: None,
            theme: "Catppuccin-Latte".to_string(),
        };

        // Act
        let result = config.project_name();

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ExplicitName");
    }

    #[test]
    fn test_config_clone() {
        // Arrange
        let original = Config {
            repo: PathBuf::from("/test/path"),
            output: PathBuf::from("output"),
            name: Some("test".to_string()),
            owner: Some("owner".to_string()),
            theme: "Catppuccin-Mocha".to_string(),
        };

        // Act
        let cloned = original.clone();

        // Assert
        assert_eq!(cloned.repo, original.repo);
        assert_eq!(cloned.output, original.output);
        assert_eq!(cloned.name, original.name);
        assert_eq!(cloned.owner, original.owner);
        assert_eq!(cloned.theme, original.theme);
    }

    #[test]
    fn test_config_debug_format() {
        // Arrange
        let config = Config {
            repo: PathBuf::from("."),
            output: PathBuf::from("dist"),
            name: None,
            owner: None,
            theme: "base16-ocean.light".to_string(),
        };

        // Act
        let debug_str = format!("{:?}", config);

        // Assert
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("theme"));
    }

    #[test]
    fn test_validate_existing_path() {
        // Arrange
        let config = Config {
            repo: PathBuf::from("."),
            output: PathBuf::from("dist"),
            name: None,
            owner: None,
            theme: "Catppuccin-Latte".to_string(),
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(result.is_ok(), "Current directory should be valid");
    }

    #[test]
    fn test_config_default_theme() {
        // Arrange & Act
        let config = Config {
            repo: PathBuf::from("."),
            output: PathBuf::from("dist"),
            name: None,
            owner: None,
            theme: "Catppuccin-Latte".to_string(),
        };

        // Assert
        assert_eq!(
            config.theme, "Catppuccin-Latte",
            "Default theme should be Catppuccin-Latte"
        );
    }
}
