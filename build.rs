//! Supply kernel's core compile-generated constants with the crate.
//!
//! Generates constant definitions, compile-time flags, defines linker
//! script memory layout and appends external source files, like C code
//! or assembly.

use buildConfig::SysConfig;
use std::env;

/* Found in "arch/<target_arch>/cfg" */
const CONFIG_NAME: &'static str = "core.yaml";

fn main() {
    let mut cfg = SysConfig::new(CONFIG_NAME)
        .include_default(CONFIG_NAME);

    /* x86_64 architecture copies most of values from x86 */
    // TODO: Make this work automatically in buildConfig.
    if env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "x86_64" {
        use std::path::PathBuf;
        let x86_yaml = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("arch/x86/cfg/core.yaml");
        
        cfg = cfg.include_yaml_path(x86_yaml);
    }

    cfg
        .include_yaml(CONFIG_NAME)
        .generate();
}
