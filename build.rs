// pub const PKG_VERSION: &str = "0.1.0";
// pub const PKG_NAME: &str = "solgraph_retriever";
// pub const PKG_AUTHORS: &str = "Zenito Pires <zenitopires@gmail.com>";
//
// fn main() {
//     built::write_built_file().expect("Failed to acquire build-time information");
// }
fn main() {
    let mut opts = built::Options::default();
    opts.set_dependencies(true);

    let src = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dst = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built.rs");
    built::write_built_file_with_opts(&opts, src.as_ref(), &dst)
        .expect("Failed to acquire build-time information");
}