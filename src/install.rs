use anyhow::{Context, Result};
use ripunzip::{UnzipEngine, UnzipOptions};
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::base::NeptuneInstall;
use crate::progress::ProgressDisplayer;

fn report_on_insufficient_readahead_size() {
    warn!("Warning: this operation required several HTTP(S) streams.\nThis can slow down decompression.");
}

fn download_and_extract(output_directory: &PathBuf) -> Result<()> {
    let engine = UnzipEngine::for_uri(
        "https://github.com/uwu/neptune/archive/refs/heads/master.zip",
        None,
        report_on_insufficient_readahead_size,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create UnzipEngine: {e}"))?;

    let opts: UnzipOptions = UnzipOptions {
        output_directory: Some(output_directory.clone()),
        password: None,
        single_threaded: false,
        filename_filter: None,
        progress_reporter: Box::new(ProgressDisplayer::new()),
    };

    engine
        .unzip(opts)
        .map_err(|e| anyhow::anyhow!("Failed to unzip: {e}"))?;

    Ok(())
}

pub fn install(install: &NeptuneInstall, force: bool) -> Result<()> {
    debug!("Using install path: {}", install.install_path.display());

    // Check if Neptune is already installed
    if install.app_path.exists() {
        if force {
            info!(
                "Removing old Neptune app folder {}",
                install.app_path.display()
            );
            std::fs::remove_dir_all(&install.app_path)?;
        } else {
            anyhow::bail!("Neptune app folder already exists. Use --force to override.");
        }
    }

    // check if original app.asar moved
    if !install.orig_asar_path.exists() {
        info!(
            "Backing up {} to {}",
            install.app_asar_path.display(), 
												  install.orig_asar_path.display()
        );
        std::fs::rename(&install.app_asar_path, &install.orig_asar_path)?;
    }

    info!(
        "Downloading & extracting Neptune to {}",
        install.temp_path.display()
    );
    download_and_extract(&install.temp_path)?;

    let injector_path = install.temp_path.join("neptune-master/injector");
    if !injector_path.exists() {
        anyhow::bail!(
            "Neptune injector failed to extract to {}",
            injector_path.display()
        );
    }

    info!("Installing neptune to {}", install.app_path.display());
    std::fs::rename(&injector_path, &install.app_path).with_context(|| {
        format!(
            "Failed to move injector from {} to {}",
            injector_path.display(),
            install.app_path.display()
        )
    })?;

    info!("Neptune has been installed successfully.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    fn mock_install_fs(install: &NeptuneInstall) -> Result<()> {
        // Create a mock Neptune directory structure
        let neptune_download_dir = install.temp_path.join("neptune-master");
        fs::create_dir(&neptune_download_dir)?;
        fs::create_dir(neptune_download_dir.join("injector"))?;

        // Create a mock app.asar file
        File::create(install.install_path.join("app.asar"))?;

        Ok(())
    }

    fn assert_install_success(neptune: &NeptuneInstall) -> Result<()> {
        // Check if the injector was moved correctly
        assert!(neptune.install_path.join("app").exists());

        // Check if app.asar was renamed to original.asar
        assert!(neptune.install_path.join("original.asar").exists());
        assert!(!neptune.install_path.join("app.asar").exists());

        Ok(())
    }

    #[test]
    fn test_install() -> Result<()> {
        let neptune = NeptuneInstall::mock()?;

        mock_install_fs(&neptune)?;

        assert!(install(&neptune, false).is_ok());

        assert_install_success(&neptune)?;

        Ok(())
    }

    #[test]
    fn test_install_with_force() -> Result<()> {
        let neptune = NeptuneInstall::mock()?;

        mock_install_fs(&neptune)?;

        // Create a mock existing app directory
        fs::create_dir(&neptune.app_path)?;

        assert!(install(&neptune, true).is_ok());

        assert_install_success(&neptune)?;

        Ok(())
    }
}
