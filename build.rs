use std::env;

fn main() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    println!("cargo:rustc-link-search=native={}/build", current_dir.as_path().to_string_lossy());
    println!("cargo:rustc-link-arg=-T{}/src/arch/x86_64/linker.ld", current_dir.as_path().to_string_lossy());
}