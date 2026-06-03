//! Supply kernel's core compile-generated constants with the crate.

use buildConfig::SysConfig;

const CONFIG_NAME: &'static str = "core.yaml";

fn main() {
    SysConfig::new(CONFIG_NAME)
        .include_default(CONFIG_NAME)
        .include_yaml(CONFIG_NAME)
        .generate();
}
