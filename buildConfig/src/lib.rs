//! notOS Configuration Manager.
//!
//! Helper crate that parses configuration `.yaml` files, appending hardware-specific
//! constants, types, external dependencies and compilation flags.
#![allow(non_snake_case)]

use std::collections::HashMap;
use std::{
    any::type_name, 
    env,
};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use build_const::{ConstWriter, ConstValueWriter};
use rust_yaml::{Yaml, Value};
use cc::Build;

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
    /// Path to `.yaml` configuration files.
    pub paths: Vec<PathBuf>,
    consts: ConstValueWriter,
    includes: Build,

    root_path: PathBuf,
}

impl SysConfig {
    const KB: i64 = 1024;
    const MB: i64 = 1024 * Self::KB;
    const GB: i64 = 1024 * Self::MB;

    /// Creates a new instance of [`SysConfig`].
    ///
    /// **Parameters**
    ///
    /// * `gen_name`: Name of generated file that will include all necessary constants.
    pub fn new(gen_name: &str) -> Self {
        Self {
            consts: ConstWriter::for_build(gen_name)
                .expect("Failed to initialize constant code generator")
                .finish_dependencies(),
            includes: Build::new(),
            paths: Vec::new(),
            root_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent().expect("Failed to locate workspace root.")
                .to_path_buf()
        }
    }

    /// Includes a `.yaml` file to parse via provided path.
    ///
    /// **Parameters**
    ///
    /// * `path`: Custom path to `.yaml` configuration.
    #[inline]
    pub fn include_yaml_path(mut self, path: PathBuf) -> Self {
        self.paths.push(path);
        self
    }

    /// Includes a `.yaml` file to parse via provided name, assuming its 
    /// default location within the below path:
    /// - `<workspace_root>/arch/<target_arch>/cfg`
    ///
    /// **Parameters**
    ///
    /// * `name`: Configuration file name.
    pub fn include_yaml(self, name: &str) -> Self {
        let root = self.root_path.clone();
        self.include_yaml_path(
            root
                .join("arch")
                .join(env::var("CARGO_CFG_TARGET_ARCH")
                    .expect("Failed to read target architecture"))
                .join("cfg")
                .join(name),
        )
    }

    /// Includes a default `.yaml` file to parse via provided name, assuming its 
    /// default location within the below path:
    /// - `<workspace_root>/arch/default_cfg`
    ///
    /// **Parameters**
    ///
    /// * `name`: Configuration file name.
    pub fn include_default(self, name: &str) -> Self {
        let root = self.root_path.clone();
        self.include_yaml_path(
            root
                .join("arch/default_cfg")
                .join(name.replace(".yaml", ".default.yaml")),
        )
    }

    /// Stops the configuration phase and generates constants.
    ///
    /// Apply this function in `build.rs` within the module to generate
    /// constants derived from hardware-specific configuration files.
    ///
    /// ## Behavior
    ///
    /// - appends all constants defined within the file to the crate. For main
    /// hardware definition yaml (`core.yaml`) the `mandatory` and `compilation`
    /// nested mappings are expected (but naming can be different) to implement
    /// all main kernel constants;
    /// - arrays are defines as constant static arrays;
    /// - boolean constants are also passed as compile-time cargo flags (`#[cfg(CONFIG_<NAME>)]`);
    /// - special constant `_linker` defines linker script;
    /// - special constants starting with `_c`/`_assembly` compiles and includes files within the
    /// provided directory path.
    /// - special constants starting with `_include` will include `.h` headers within the provided
    /// directory path.
    pub fn generate(mut self) {
        self.parse_yaml();
        // Only compile when at least one source is added.
        if self.includes.get_files().next().is_some() {
            // Creating a set of object files to link against. 
            // Rust compiler takes care of the rest.
            let objects = self.includes.compile_intermediates();
            for obj_path in objects {
                println!("cargo:rustc-link-arg-bins={}", obj_path.display());
            }
        }
        self.consts.finish();
    }

    /// Allows to append custom constant value, which is not defined in `.yaml`. 
    ///
    /// ## Prototype
    ///
    /// It is recommended to use this for prototyping, so that `.yaml` files stay 
    /// clean. For product code it is not recommended to be used.
    ///
    /// **Parameters**
    ///
    /// * `key_str`: Name of constant variable. <name> is converted to CONFIG_<NAME>
    /// * `val`: Value of any type that can be creates as constant.
    ///
    pub fn include_non_yaml_constant<T>(mut self, key_str: &str, val: T) -> Self
        where T: std::fmt::Debug + std::fmt::Display
    {
        let rust_const_name = format!("CONFIG_{}", key_str.to_uppercase());
        self.gen_key(&rust_const_name, val);
        self
    }

    /// Include external source file.
    ///
    /// ## Supported formats:
    /// - `.c`: C Language source files.
    /// - `.S`: Assembly source files with C preprocessing.
    /// - `.s`: Assembly source files.
    pub fn include_source(mut self, path: PathBuf) -> Self {
        self.__include_source(path);
        self
    }

    fn __include_source(&mut self, path: PathBuf) {
        println!("cargo:rerun-if-changed={}", path.display());
        println!("cargo:warning=[notOS-config] Including source file: {}", 
            path.display());
        self.includes.file(path);
    }

    /// Include C header directory.
    ///
    /// Allows external C/C++ definitions and functions linkage. All `.h` files
    /// within provided directory will be included and linked against.
    pub fn include_directory(mut self, path: PathBuf) -> Self {
        self.__include_directory(path);
        self
    }

    fn __include_directory(&mut self, path: PathBuf) {
        println!("cargo:warning=[notOS-config] Including directory {}", path.display());
        self.includes.include(path);
    }

    /// Link build crate against linker script.
    ///
    /// **Parameters**
    ///
    /// * `path`: Path to linker script for build.
    pub fn include_linker_script(self, path: PathBuf) -> Self {
        Self::__include_linker_script(path);
        self
    }

    fn __include_linker_script(path: PathBuf) {
        println!("cargo:warning=[notOS-config] Linking against: {}", path.display()); 
        println!("cargo:rustc-link-arg-bins=-T{}",   // Main script to link against. 
            path.display());
        println!("cargo:rerun-if-changed={}", path.display());
    }

    /// Parses `yaml` configuration document with current path.
    fn parse_yaml(&mut self) {
        let mut yaml_content = String::new();

        /* Packing all YAMLs together. */
        self.paths.iter().for_each(|path| {
            println!("cargo:rerun-if-changed={}", path.display());

            File::open(path).unwrap_or_else(|err| {
                panic!("Failed to open config file at {}: {}", path.display(), err);
            })
            .read_to_string(&mut yaml_content)
            .expect("Failed to write contents into string");

            // Replacing paths on run.
            yaml_content = yaml_content
                .replace("./",
                    &(path.parent().unwrap().display().to_string() + "/"))
                .replace("//", &(self.root_path.to_string_lossy() + "/"));

            println!("cargo:warning=[notOS-config] Processing hardware target spec: {}", path.display());
        });

        let yaml_engine = Yaml::new();
        let root_doc = yaml_engine.load_str(&yaml_content).unwrap_or_else(|err| {
            panic!("Invalid YAML 1.2 syntax: {}", err);
        });

        self.parse_key("", &root_doc);
    }

    fn parse_key(&mut self, key_str: &str, value: &rust_yaml::Value) {
        // Enforce the requested "CONFIG_" prefix.
        let rust_const_name = format!("CONFIG{}{}", 
            if key_str.starts_with("_") { "" } else { "_" }, 
            key_str.to_uppercase());

        match value {
            Value::Int(i) => self.gen_key(&rust_const_name, *i),
            Value::Float(f) => self.gen_key(&rust_const_name, *f),
            Value::Bool(b) => { 
                self.gen_key(&rust_const_name, *b);

                if *b == true {
                    // Also append as a compile-time flag if set.
                    println!("cargo::rustc-check-cfg=cfg({})", rust_const_name);
                }
            },
            Value::String(s) => { 
                if let Some(i) = Self::as_xbytes(s) {
                    // KB, MB, GB special case.
                    self.gen_key(&rust_const_name, i);
                } else if rust_const_name.starts_with("CONFIG_LINKER") {
                    // Link against linker script.
                    Self::__include_linker_script(PathBuf::from(s));
                } else {
                    // Generate regular string literal. 
                    self.gen_key(&rust_const_name, s.as_str())
                }
            },
            Value::Mapping(nested) => { 
                for (key, value) in nested.iter() {
                    key.as_str().map(|k| 
                        self.parse_key(k, value)
                    );
                }
            },
            // Sequences are generated recursively.
            Value::Sequence(seq) => {
                // Special case for external source code.
                if rust_const_name.starts_with("CONFIG_C") ||
                   rust_const_name.starts_with("CONFIG_ASSEMBLY") {
                    self.gen_sources(&seq);
                } else if rust_const_name.starts_with("CONFIG_INCLUDE") {
                    self.gen_includes(&seq);
                } else if rust_const_name.starts_with("CONFIG_LINKER") {
                    self.gen_linkers(&seq);
                } else {
                    self.gen_sequence(&rust_const_name, seq);
                }
            },
            // FEAT: Create ZST struct instead?
            Value::Null => println!("cargo:warning=[notOS-config] Null value of name {} ignored.",
                rust_const_name),
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

    fn gen_array<T>(&mut self, name: &String, vals: &[T]) where 
        T: std::fmt::Debug + std::fmt::Display
    {
        let ty = type_name::<T>();
        println!("cargo:warning=[notOS-config] Generated const {name} ({ty}): {:?}", &vals);
        self.consts.add_array(&name, ty, vals);
    }

    // Generates sequences with recursive support.
    fn gen_sequence(&mut self, name: &String, seq: &Vec<Value>) {
        let flattener = FlattenedValues::new(seq);
        let first_primitive = FlattenedValues { stack: flattener.stack.clone() }.next();

        let Some(target_type) = first_primitive else { return; };

        match target_type {
            Value::Int(_) => {
                let data: Vec<i64> = flattener.filter_map(|v| match v {
                    Value::Int(i) => Some(*i),
                    _ => None,
                }).collect();
                self.gen_array(name, data.as_slice());
            },
            Value::Bool(_) => {
                let data: Vec<bool> = flattener.filter_map(|v| match v {
                    Value::Bool(b) => Some(*b),
                    _ => None,
                }).collect();
                self.gen_array(name, data.as_slice());
            },
            Value::Float(_) => {
                let data: Vec<f64> = flattener.filter_map(|v| match v {
                    Value::Float(f) => Some(*f),
                    _ => None,
                }).collect();
                self.gen_array(name, data.as_slice());
            },
            Value::String(_) => {
                let data: Vec<&str> = flattener.filter_map(|v| match v {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                }).collect();
                self.gen_array(name, data.as_slice());
            },
            _ => panic!("Unsupported type definition for constant sequence."),
        }
    }

    // Parses sequence of string paths to include during compilation stage.
    fn gen_sources(&mut self, seq: &Vec<Value>) {
        seq.iter().for_each(|v| match v {
            Value::String(s) => self.__include_source(
                PathBuf::from(s)
            ),
            _ => panic!("Data types other than strings are not allowed for external source files list."),
        });
    }

    // Parses sequence of string path to include as directories with headers.
    fn gen_includes(&mut self, seq: &Vec<Value>) {
        seq.iter().for_each(|v| match v {
            Value::String(s) => self.__include_directory(
                PathBuf::from(s)
            ),
            _ => panic!("Data types other than strings are not allowed for external source files list."),
        });
    }

    fn gen_linkers(&mut self, seq: &Vec<Value>) {
        seq.iter().for_each(|v| match v {
            Value::String(s) => Self::__include_linker_script(
                PathBuf::from(s)
            ),
            _ => panic!("Data types other than strings are not allowed for external source files list."),
        });
    }

    // KB, MB, GB parser helper.
    fn as_xbytes(str: &String) -> Option<i64> {
        let mut str = str.to_uppercase();
        let mut mul = None;
        let idx = str.chars().count() - 2;

        // Converting values to actual representation.
        let hash = HashMap::from([
            ("KB", Self::KB),
            ("MB", Self::MB),
            ("GB", Self::GB),
        ]);

        for (key, value) in hash.iter() {
            if str.ends_with(key) {
                mul.replace(value);
            }
        }
        let m = mul?;

        if let Some((byte_index, _)) = str.char_indices().nth(idx) {
            str.truncate(byte_index);
        } 
        let v = str.parse::<i64>().ok()?;

        Some(m * v)
    }
}

/// A zero-allocation structural flattening iterator for deep sequences
///
/// Allows for nested definitions of arrays with any dimension.
struct FlattenedValues<'a> {
    stack: Vec<(&'a [Value], usize)>,
}

impl<'a> FlattenedValues<'a> {
    fn new(root: &'a [Value]) -> Self {
        Self { stack: vec![(root, 0)] }
    }
}

impl<'a> Iterator for FlattenedValues<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (current_slice, index) = self.stack.last_mut()?;

            if *index >= current_slice.len() {
                self.stack.pop();
                continue;
            }

            let val = &current_slice[*index];
            *index += 1;

            if let Value::Sequence(sub_vec) = val {
                self.stack.push((sub_vec.as_slice(), 0));
                continue;
            }

            return Some(val);
        }
    }
}
