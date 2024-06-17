use anyhow::Result;
use std::{env::var, fs::copy, path::PathBuf};

fn main() -> Result<()> {
    // On macOS, we need to copy the Vulkan binary
    // to the output directory or we'll have issues.
    if cfg!(target_os = "macos") {
        let src = get_lib_dir()?.join("libvulkan.1.dylib");
        let dst = get_out_dir()?.join("libvulkan.1.dylib");

        copy(&src, &dst)?;
    }

    Ok(())
}

/// Get the workspace directory.
fn get_workspace_dir() -> Result<PathBuf> {
    let workspace_dir = var("WORKSPACE_DIR")?;
    let workspace_dir = PathBuf::from(workspace_dir);

    Ok(workspace_dir)
}

/// Get the libs directory for the specific os / arch.
fn get_lib_dir() -> Result<PathBuf> {
    let workspace_dir = get_workspace_dir()?;
    let arch = var("CARGO_CFG_TARGET_ARCH")?;
    let os = var("CARGO_CFG_TARGET_OS")?;

    Ok(PathBuf::from(workspace_dir.clone())
        .join("libs")
        .join(format!("{}-{}", os, arch)))
}

/// Get the output directory.
fn get_out_dir() -> Result<PathBuf> {
    let workspace_dir = get_workspace_dir()?;
    let profile = var("PROFILE")?;

    Ok(PathBuf::from(workspace_dir)
        .join("target")
        .join(profile)
        .join("examples"))
}
