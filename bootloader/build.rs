//! Supply bootloader with specific configuration and linkage.

use buildConfig::SysConfig;

const CONFIG_NAME: &'static str = "bootloader.yaml";

fn main() {
    SysConfig::new(CONFIG_NAME)
        .include_yaml(CONFIG_NAME)
        .generate();
}
