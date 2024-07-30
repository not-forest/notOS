/// This module handles AML definitions from parsed AML stream. 

use crate::kernel_components::arch_x86_64::acpi::diff::{self, namespace::{ACPINamespace, LayerType, NameString}};

use super::{aml_parser::Parsed, pkg::PkgLength};

/// A ZST type that implements Parsed trait to parse the scope.
pub struct Scope;

impl Parsed for Scope {
    fn parse(ptr: &mut usize, bytes: &'static [u8], ns: &mut ACPINamespace) -> super::aml_parser::AMLParserResult<Self> {
        // Obtaining the length of the scope.
        let old_ptr = *ptr;
        let pkg_length = PkgLength::parse(ptr, bytes, ns)?;
        
        // Obtaining scope's name string (path)
        let mut name_string = NameString::new();
        name_string.build(&bytes[*ptr - old_ptr..]);

        *ptr += name_string.len();
        // Appending the layer with a new scope.
        ns.append_layers(pkg_length, name_string, LayerType::Scope)?;
        Ok(Scope)
    }
}
