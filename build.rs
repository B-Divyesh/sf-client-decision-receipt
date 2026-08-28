fn main() {
    let sha = std::env::var("BUILD_SHA")
        .or_else(|_| std::env::var("GIT_SHA"))
        .or_else(|_| std::env::var("SOURCE_COMMIT"))
        .unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=BUILD_SHA={sha}");
}
