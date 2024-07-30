/// Defines an ACPI namespace and it's components.

use alloc::{collections::BTreeMap, vec::Vec};
use super::{
    aml::{NameSeg, DUAL_NAME_PREFIX, PARENT_PREFIX_CHAR, ROOT_CHAR}, objects::ACPIObject, parser::PkgLength, AMLParserError, AMLResult
};

/// ACPI Namespace
///
/// It is a tree-like data structure that maps variable names to internal objects. When OS queries
/// the AML interpreter, the interpreter searches the namespace for the requested variable,
/// evaluates the object associated with the variable and returns the result of the computation.
///
/// ACPI namespace is owned by AML interpreter and must stay in kernel space. AML interpreter must
/// build the namespace first, by parsing the DSDT and SSDT tables.
pub struct ACPINamespace {
    /// This field is required only when parsing new data to the namespace.
    current_seg: NameSeg,
    /// Root layer of the namespace. Each next layer can be obtained recursively. Basically it is
    /// just a regular scope layer, however it contains all sub-layers.
    root: NamespaceLayer,
}

impl ACPINamespace {
    /// Creates a new clear namespace instance for further building. This is creates by parser
    /// after it's initialization.
    pub fn blank() -> Self {
        Self {
            current_seg: NameSeg::new(0, 0, 0, 0),
            root: NamespaceLayer::new(LayerType::Scope),
        }
    }

    pub fn append_layers(&mut self, pkg_len: PkgLength, name_string: NameString, ltype: LayerType) -> AMLResult<AMLParserError> {
        let mut iter = name_string.0.into_iter();

        while let Some(ns) = iter.next() {
            match ns {
                NamespacePath::RootChar => {
                    if let Some(NamespacePath::NamePath(nameseg)) = iter.next() {
                        crate::println!("test: {:#?}", nameseg);
                        self.root().populate_layer(nameseg, ltype);
                        self.current_seg = nameseg;
                    } else {
                        return Err(AMLParserError::UnexpectedToken) 
                    }
                },
                NamespacePath::DualPrefixChar => {
                    if let Some(NamespacePath::NamePath(nameseg)) = iter.next() {
                        if let Some(layer) = self.root().children.get_mut(&nameseg) {
                            layer.populate_layer(nameseg, ltype);
                            self.current_seg = nameseg;
                        }
                    } else { 
                        return Err(AMLParserError::UnexpectedToken) 
                    }
                }
                NamespacePath::ParentPrefixChar => {
                    unimplemented!()
                },
                NamespacePath::NamePath(nameseg) => {
                    if let Some(layer) = self.root().children.get_mut(&nameseg) {
                        layer.populate_layer(nameseg, ltype);
                        self.current_seg = nameseg;
                    } else {
                        return Err(AMLParserError::NotInNamespace) 
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns a mutable reference to a current namespace layer.
    ///
    /// # Warn
    ///
    /// 'current_seg' field must be initialized with proper data.
    pub fn current_layer(&mut self) -> Option<&mut NamespaceLayer> {
        self.root.children.get_mut(&self.current_seg)
    }

    /// Returns a root namespace layer.
    pub fn root(&mut self) -> &mut NamespaceLayer {
        &mut self.root
    }
}

/// Defines a namespace layer.
///
/// This structure is nothing but a container for ACPI objects and sublayers. Each layer may define a another 
/// layer (scope), or define ACPI objects.
#[derive(Debug)]
pub struct NamespaceLayer {
    /// Layer type define different behavior of interpreter, when manupulating with underlying data
    pub r#type: LayerType, 
    /// Each layer can hold unlimited amount of sub-layers.
    children: BTreeMap<NameSeg, NamespaceLayer>,
    /// Each layer can hold unlimited amount of objects.
    objs: BTreeMap<NameSeg, ACPIObject>,
}

impl NamespaceLayer {
    /// Creates a new empty namespace layer with provided layer type. This is basically done each
    /// time a new scope/layer of some type is defined with AML code.
    pub(crate) fn new(ltype: LayerType) -> Self {
        Self {
            r#type: ltype,
            children: BTreeMap::new(),
            objs: BTreeMap::new(),
        }
    }

    /// Adds one new layer to a binary tree, if it's NameSeg is not used.
    ///
    /// This function cannot fail, because multiple definition of one layer is allowed and would be
    /// just simply ignored.
    pub(crate) fn populate_layer(&mut self, nameseg: NameSeg, ltype: LayerType) {
        if !self.children.contains_key(&nameseg) {
            self.children.insert(nameseg, NamespaceLayer::new(ltype));
        }
    }

    /// Adds one new object to this particular layer.
    ///
    /// # Error
    ///
    /// This function would return an error if the same name segment would be used twice.
    pub(crate) fn populate_object(&mut self, nameseg: NameSeg, obj: ACPIObject) -> AMLResult<()> { 
        if self.objs.contains_key(&nameseg) {
            Err(()) 
        } else {
            self.objs.insert(nameseg, obj);
            Ok(())
        }
    }
}

/// Defines different AML layer's type in namespace hierarchy. This includes regular scopes, devices, thermal zones etc.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    /// Defines some arbitrary AML scope.
    Scope,
    /// Represents a scope of a device.
    Device,
    /// Legacy scope for processors. New AML code will define processors as 'Device'
    Processor,
    /// Scope of power resources.
    PowerResource,
    /// Scope of thermal zone resources.
    ThermalZone,
}

/// A special order of character that defines either an absolute path (from root layer) or relative
/// path to some scope, device or another layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameString(Vec<NamespacePath>);

impl NameString {
    /// Creates a new empty namestring path.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Builds a path from the obtained bytes.
    pub fn build(&mut self, bytes: &'static [u8]) {
        let mut iter = bytes.iter();
        let mut separator = false;

        while let Some(byte) = iter.next() {
            let path = match *byte {
                ROOT_CHAR           => {separator = true; NamespacePath::RootChar},
                PARENT_PREFIX_CHAR  => {separator = true; NamespacePath::ParentPrefixChar},
                DUAL_NAME_PREFIX    => {separator = true; NamespacePath::DualPrefixChar},
                b1 @ _              => {
                    if separator {
                        if let Some(b2 @ _) = iter.next() {
                            if let Some(b3 @ _) = iter.next() {
                                if let Some(b4 @ _) = iter.next() {
                                    separator = false;
                                    NamespacePath::NamePath(NameSeg::new(b1, *b2, *b3, *b4)) 
                                } else { return }
                            } else { return }
                        } else { return }
                    } else { return }
                },
            };

            self.0.push(path);
        }
    }

    /// Calculates the about of bytes in the name string path.
    pub fn len(&self) -> usize {
        let mut counter = 0;
        for ns in self.0.iter() {
            match ns {
                NamespacePath::RootChar | NamespacePath::ParentPrefixChar | NamespacePath::DualPrefixChar => counter += 1,
                NamespacePath::NamePath(_) => counter += 4,
            }
        }
        counter
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespacePath { 
    /// Absolute path from the root layer.
    RootChar,
    /// Going further to child
    DualPrefixChar,
    /// Going back to parent.
    ParentPrefixChar,
    /// Name of layers, which are part of the path.
    NamePath(NameSeg),
}
