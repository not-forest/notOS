fn main() {
    println!("cargo:rustc-link-search=native=/home/notforest/Documents/notOS/build");
    println!("cargo:rustc-link-arg=-T/home/notforest/Documents/notOS/src/arch/x86_64/linker.ld");
}