use tracing::{debug, info, trace, warn, Level};

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

// Log package info
pub fn pkg_info() {
    info!(
        "Starting {}, version: {}",
        built_info::PKG_NAME,
        built_info::PKG_VERSION
    );
    info!("Host: {}", built_info::HOST);
    info!("Built for {}", built_info::TARGET);
    info!("Package authors: {}", built_info::PKG_AUTHORS);
    info!("Repository: {}", built_info::PKG_REPOSITORY);
    info!("Beginning collection retrieval...");
}