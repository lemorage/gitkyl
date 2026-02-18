//! Output directory and asset setup.
//!
//! Creates required directory structure and writes CSS assets
//! before page generation begins.

use anyhow::{Context, Result};
use gitkyl::UiMode;
use std::fs;
use std::path::Path;

/// Creates output directory structure and writes CSS assets.
///
/// Sets up required directories (output root, assets) and writes all CSS
/// bundles to the assets directory.
///
/// # Arguments
///
/// * `output_dir`: Base output directory path
/// * `ui_mode`: UI color mode (light or dark)
///
/// # Errors
///
/// Returns error if directory creation fails or CSS writing fails
pub fn setup_output_directories(output_dir: &Path, ui_mode: UiMode) -> Result<()> {
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let assets_dir = output_dir.join("assets");
    fs::create_dir_all(&assets_dir).context("Failed to create assets directory")?;

    gitkyl::write_css_assets(&assets_dir, ui_mode).context("Failed to write CSS assets")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_creates_directories() {
        use tempfile::TempDir;

        // Arrange: create temporary output directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_path = temp_dir.path().join("output");

        // Act: call setup function
        let result = setup_output_directories(&output_path, UiMode::Light);

        // Assert: directories should be created
        assert!(
            result.is_ok(),
            "setup_output_directories failed: {:?}",
            result.err()
        );
        assert!(output_path.exists(), "Output directory not created");
        assert!(
            output_path.join("assets").exists(),
            "Assets directory not created"
        );
    }

    #[test]
    fn test_setup_writes_css_assets() {
        use tempfile::TempDir;

        // Arrange: create temporary output directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_path = temp_dir.path().join("output");

        // Act: call setup function
        let result = setup_output_directories(&output_path, UiMode::Light);

        // Assert: CSS files should be written
        assert!(
            result.is_ok(),
            "setup_output_directories failed: {:?}",
            result.err()
        );

        let assets_dir = output_path.join("assets");
        assert!(assets_dir.exists(), "Assets directory not created");

        // Check that CSS assets were written by verifying files exist
        let css_files = std::fs::read_dir(&assets_dir)
            .expect("Failed to read assets dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "css"))
            .count();

        assert!(css_files > 0, "No CSS files written to assets directory");
    }
}
