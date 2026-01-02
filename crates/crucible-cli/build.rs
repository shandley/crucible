//! Build script for crucible-cli.
//!
//! This script builds the React frontend when compiling in release mode.

use std::process::Command;

fn main() {
    // Only build frontend in release mode or if CRUCIBLE_BUILD_FRONTEND is set
    let is_release = std::env::var("PROFILE").map(|p| p == "release").unwrap_or(false);
    let force_build = std::env::var("CRUCIBLE_BUILD_FRONTEND").is_ok();

    if is_release || force_build {
        println!("cargo:warning=Building frontend...");

        let frontend_dir = std::path::Path::new("frontend");

        // Check if node_modules exists
        if !frontend_dir.join("node_modules").exists() {
            let status = Command::new("npm")
                .args(["install"])
                .current_dir(frontend_dir)
                .status()
                .expect("Failed to run npm install");

            if !status.success() {
                panic!("npm install failed");
            }
        }

        // Build the frontend
        let status = Command::new("npm")
            .args(["run", "build"])
            .current_dir(frontend_dir)
            .status()
            .expect("Failed to run npm build");

        if !status.success() {
            panic!("Frontend build failed");
        }
    } else {
        // In development, create an empty dist folder if it doesn't exist
        // so rust-embed doesn't fail
        let dist_dir = std::path::Path::new("dist");
        if !dist_dir.exists() {
            std::fs::create_dir_all(dist_dir).ok();
            // Create a placeholder index.html for development
            std::fs::write(
                dist_dir.join("index.html"),
                r#"<!DOCTYPE html>
<html>
<head><title>Crucible - Development</title></head>
<body>
<h1>Crucible Development Mode</h1>
<p>Run the frontend dev server: <code>cd frontend && npm run dev</code></p>
<p>Then visit <a href="http://localhost:5173">http://localhost:5173</a></p>
</body>
</html>"#,
            )
            .ok();
        }
    }

    // Re-run if frontend source changes
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/package.json");
}
