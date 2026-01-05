//! Build script for crucible-cli.
//!
//! This script builds the React frontend for embedding in the binary.

use std::process::Command;

fn main() {
    let frontend_dir = std::path::Path::new("frontend");
    let dist_dir = std::path::Path::new("dist");

    // Skip frontend build if CRUCIBLE_SKIP_FRONTEND is set (for faster iteration on Rust code)
    if std::env::var("CRUCIBLE_SKIP_FRONTEND").is_ok() {
        // Ensure dist exists with placeholder
        if !dist_dir.exists() {
            create_placeholder(dist_dir);
        }
        return;
    }

    // Check if frontend source exists
    if !frontend_dir.exists() {
        println!("cargo:warning=Frontend directory not found, skipping build");
        create_placeholder(dist_dir);
        return;
    }

    // Check if node_modules exists, install if not
    if !frontend_dir.join("node_modules").exists() {
        println!("cargo:warning=Installing frontend dependencies...");
        let status = Command::new("npm")
            .args(["install"])
            .current_dir(frontend_dir)
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(_) => {
                println!("cargo:warning=npm install failed, using placeholder");
                create_placeholder(dist_dir);
                return;
            }
            Err(e) => {
                println!("cargo:warning=npm not found ({e}), using placeholder");
                create_placeholder(dist_dir);
                return;
            }
        }
    }

    // Build the frontend
    println!("cargo:warning=Building frontend...");
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(frontend_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=Frontend built successfully");
        }
        Ok(_) => {
            println!("cargo:warning=Frontend build failed, using placeholder");
            create_placeholder(dist_dir);
        }
        Err(e) => {
            println!("cargo:warning=Could not run npm ({e}), using placeholder");
            create_placeholder(dist_dir);
        }
    }

    // Re-run if frontend source changes
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/package.json");
}

fn create_placeholder(dist_dir: &std::path::Path) {
    std::fs::create_dir_all(dist_dir).ok();
    std::fs::write(
        dist_dir.join("index.html"),
        r#"<!DOCTYPE html>
<html>
<head><title>Crucible - Build Error</title></head>
<body>
<h1>Frontend Not Built</h1>
<p>The frontend could not be built. Please ensure Node.js and npm are installed.</p>
<p>Then rebuild with: <code>cargo build --bin crucible</code></p>
</body>
</html>"#,
    )
    .ok();
}
