//! Supply bootloader with specific configuration and linkage.

use buildConfig::SysConfig;

const CONFIG_NAME: &str = "bootloader.yaml";

fn main() {
    SysConfig::new(env!("CARGO_PKG_NAME"))
        .include_yaml(CONFIG_NAME)
        .generate();
}
