//! notOS Configuration Manager.
//!
//! Helper crate that parses `.yaml` files and appends required constants
//! and data types to kernel crates, allowing to compile `notOS` for different
//! targets.
#![allow(non_snake_case)]

use std::{
    any::type_name, 
    env,
};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use build_const::{ConstWriter, ConstValueWriter};
use rust_yaml::{Yaml, Value};

/// System Configuration Builder Structure.
///
/// Parses configuration `.yaml`s and generates compile-time constants,
/// flags, and type binding.
///
/// ## Default
///
/// Default configuration path is expected in `<workspace_root>/arch/<target_arch>/cfg`.
/// By default all `.yaml` configuration files must include `mandatory` nested mapping,
/// which defines mandatory key-value pairs for kernel files.
pub struct SysConfig {
    /// Path to `.yaml` configuration.
    pub path: PathBuf,
    consts: ConstValueWriter,
}

impl SysConfig {
    const KB: usize = 1024;

    /// Creates a new instance of [`SysConfig`].
    ///
    /// **Parameters**
    ///
    /// * `cfg_name`: Name of configuration `.yaml` file to be included during the build.
    ///
    /// ## Note
    ///
    /// Default configuration path: `<workspace_root>/arch/<target_arch>/cfg`
    pub fn new(cfg_name: &str) -> Self {
        Self {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent().expect("Failed to locate workspace root.")
                .join("arch")
                .join(env::var("CARGO_CFG_TARGET_ARCH")
                    .expect("Failed to read target architecture"))
                .join("cfg")
                .join(cfg_name),
            consts: ConstWriter::for_build(cfg_name)
                .expect("Failed to initialize constant code generator")
                .finish_dependencies(),
        }
    }

    /// Stops the configuration phase and generates constants.
    ///
    /// Apply this function in `build.rs` within the module to generate
    /// constants derived from hardware-specific configuration files.
    pub fn generate(mut self) {
        self.parse_yaml();
        self.consts.finish();
    }

    fn parse_yaml(&mut self) {
        println!("cargo:rerun-if-changed={}", self.path.display());

        let yaml_engine = Yaml::new();
        let mut yaml_content = String::new();

        File::open(&self.path).unwrap_or_else(|err| {
            panic!("Failed to open config file at {}: {}", self.path.display(), err);
        })
        .read_to_string(&mut yaml_content)
        .expect("Failed to write contents into string");

        let root_doc = yaml_engine.load_str(&yaml_content).unwrap_or_else(|err| {
            panic!("Invalid YAML syntax: {}", err);
        });

        println!("cargo:warning=[notOS-config] Processing hardware target spec: {}", self.path.display());

        self.parse_key("", &root_doc);
    }

    fn parse_key(&mut self, key_str: &str, value: &rust_yaml::Value) {
        // Enforce the requested "CONFIG_" prefix combined with Screaming Snake Case
        let rust_const_name = format!("CONFIG_{}", key_str.to_uppercase());

        match value {
            Value::Int(i) => self.gen_key(&rust_const_name, *i),
            Value::Bool(b) => self.gen_key(&rust_const_name, *b),
            Value::Float(f) => self.gen_key(&rust_const_name, *f),
            Value::String(s) => { 
                // Special cases.
                if s.ends_with("KB") {
                    let i = Self::as_bytes(Self::KB, s);
                    self.gen_key(&rust_const_name, i);
                } else {
                    self.gen_key(&rust_const_name, s.as_str())
                }
            },
            Value::Mapping(nested) => { 
                for (key, value) in nested.iter() {
                    let key_str = key.as_str()
                        .expect("YAML configuration key found as invalid string literal.");

                    self.parse_key(key_str, value);
                }
            },
            _ => panic!(
                    "Unsupported YAML value structure found for parameter '{}'. \
                    Only raw primitives (ints, bools, strings, floats) are supported.", 
                    key_str
                ),
        }
    }

    // Static key-value generator helper.
    fn gen_key<T>(&mut self, name: &String, val: T) where 
        T: std::fmt::Debug + std::fmt::Display
    {
        let ty = type_name::<T>();
        println!("cargo:warning=[notOS-config] Generated const {name} ({ty}): {}", &val);
        self.consts.add_value(&name, ty, val);
    }

    // KB, MB, GB parser helper.
    fn as_bytes(mul: usize, str: &String) -> i64 {
        let idx = str.chars().count() - 2;
        let mut str = str.clone();

        if let Some((byte_index, _)) = str.char_indices().nth(idx) {
            str.truncate(byte_index);
        }

        str.parse().expect("Unable to parse compound string: {str}")
    }
}
