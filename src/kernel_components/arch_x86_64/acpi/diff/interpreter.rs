
/// AML interpreter implementation.

use crate::single;
use super::{AMLParser, AMLParserError, AMLResult, AMLStream, DSDT};

/// Global static AML intepreter variable.
single! {
    pub mut AML_INTERPRETER: AMLInterpreter = AMLInterpreter::new_empty();
}

/// AML Interpreter.
///
/// This structure is an interface between the OS and ACPI Namespace. It utilizes parser to build
/// the namespace on first initialization, therefore must be provided with DSDT table. After a
/// proper initialization it can evaluate objects within the namespace and perform actions on
/// mapped hardware registers based on methods and fields defined within the namespace.
///
/// OS would obtain requested data through the interpreter, which acts as a mediator for data
/// transfer. This data could then be used to initialize device drivers and perform specific tasks.
pub struct AMLInterpreter {
    /// Parser is required to read AML encoded data and append namespace accordingly. The namespace
    /// is only owned by parser when it is updating the namespace with new data.
    parser: AMLParser,
}

impl AMLInterpreter {
    /// Creates a new AML interpreter with proper namespace obtained from DSDT table.
    ///
    /// # Error
    ///
    /// Only parser error can be obtained from here, because building the namespace is completely
    /// parser's job.
    pub fn new(dsdt: DSDT) -> Result<Self, AMLParserError> {
        let mut int = AMLInterpreter::new_empty();
        int.parser.parse(&dsdt.aml())?;
        Ok(int)
    }

    /// A wrapper that calls inner parser to parse new AML stream. It allows to update the
    /// namespace with new content from SSDT tables.
    pub fn parse(&mut self, aml_stream: &AMLStream) -> AMLResult<AMLParserError> {
        self.parser.parse(aml_stream)
    }

    /// Creates a new AML interpreter with empty namespace.
    fn new_empty() -> Self {
        Self {
            parser: AMLParser::new()
        }
    }
}

/// Error codes obtained from interpreter.
///
/// Note that parser related errors are not wrapped to interpreter ones.
pub enum AMLInterpreterError {

}
