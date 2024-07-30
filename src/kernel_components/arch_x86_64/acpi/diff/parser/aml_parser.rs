/// Custom module for parsing AML code within the DSDT and SSDT ACPI structures.
///
/// It defines a set of helper structures to decode an encoded AML stream and return
/// an output structure that is delivered in an ASL-like readable code format.

use core::sync::atomic::AtomicBool;
use crate::kernel_components::arch_x86_64::acpi::diff::namespace::ACPINamespace;
use crate::kernel_components::arch_x86_64::acpi::diff::aml::*;
use crate::kernel_components::arch_x86_64::acpi::diff::parser::definitions::Scope;
use crate::kernel_components::sync::Mutex;

/// This trait must be implemented by structures, which are a result of partial or full bytecode parsing.
///
/// A full stream of new bytecode is provided as an input data, with a mutable reference to a
/// pointer. Once this structure was properly parsed, it must increase the pointer to a proper
/// amount, so that next bytestream won't cover old data. If something went wrong, then an aml
/// error must be thrown to the very first call.
pub trait Parsed: Sized {
    /// This function must be implemented by a structure.
    fn parse(ptr: &mut usize, bytes: &'static [u8], ns: &mut ACPINamespace) -> AMLParserResult<Self>;

    #[no_mangle]
    fn _parse(ptr: &mut usize, bytes: &'static [u8], ns: &mut ACPINamespace) -> AMLParserResult<Self> {
        if bytes.is_empty() {
            return Err(AMLParserError::UnexpectedEndOfStream)
        }
        Self::parse(ptr, bytes, ns)
    }
}

/// Result value obtained from the parser, that would either invoke some function, which mutates
/// the namespace or throw an error.
pub type AMLParserResult<T> = Result<T, AMLParserError>;

/// Special parser structures that is created during the first interpreter's invokation. It parses
/// the upcoming AML bytecode, and based on it, builds the Namespace tree.
pub struct AMLParser {
    /// Holds a namespace that is required for AML interpreter to perform power-related operations.
    /// Only when parser is not parsing new upcoming byte stream, the interpreter can obtain the
    /// namespace.
    namespace: Mutex<ACPINamespace>,
}

impl AMLParser {
    /// Returns a new instance of AML Parser structure.
    pub fn new() -> Self {
        Self {
            namespace: Mutex::new(ACPINamespace::blank())
        }
    }

    /// Parses a given AML stream.
    ///
    /// Mutates self and builds the namespace based on the AML stream provided. Each new AML stream
    /// may provide some modifications to the namespace.
    pub fn parse(&mut self, aml_stream: &AMLStream) -> AMLParserResult<()> {
        // The Namespace is locked here.
        let nspace = &mut self.namespace.lock();
        // Raw bytestream slice.
        let mut bytes = aml_stream.0;
        // Flags for extended bytes.
        let mut extended_byte = false;
        // Pointer to move through the slice.
        let mut ptr = 0;

        // Different byte chunks are getting feed to the function.
        while ptr < bytes.len() {
            let current_ptr = ptr;
            self.parse_bytes(&mut extended_byte, &mut ptr, &bytes[current_ptr..], nspace)?;
        }

        Ok(()) // From here, interpreter can use the namespace freely
    }

    /// Parsing chunks of opcode bytes from the AML stream. 
    ///
    /// The namespace must be obtained from the mutex outside this function, for faster parsing.
    fn parse_bytes(&self, is_extended: &mut bool, ptr: &mut usize, aml_bytes: &'static [u8], namespace: &mut ACPINamespace) -> AMLParserResult<()> {
        let aml_byte = aml_bytes[0];

        // Extended byte 0x5B
        if *is_extended {
            match aml_byte {
                MUTEX_OP                => crate::print!("MUTEX_OP "),
                EVENT_OP                => crate::print!("EVENT_OP "),
                COND_REFOF_OP           => crate::print!("COND_REFOF_OP "),
                CREATE_FIELD_OP         => crate::print!("CREATE_FIELD_OP "),
                LOAD_TABLE_OP           => crate::print!("LOAD_TABLE_OP "),
                LOAD_OP                 => crate::print!("LOAD_OP "),
                STALL_OP                => crate::print!("STALL_OP "),
                SLEEP_OP                => crate::print!("SLEEP_OP "),
                ACQUIRE_OP              => crate::print!("ACQUIRE_OP "),
                SIGNAL_OP               => crate::print!("SIGNAL_OP "),
                WAIT_OP                 => crate::print!("WAIT_OP "),
                RESET_OP                => crate::print!("RESET_OP "),
                RELEASE_OP              => crate::print!("RELEASE_OP "),
                FROM_BCD_OP             => crate::print!("FROM_BCD_OP "),
                TO_BCD                  => crate::print!("TO_BCD "),
                REVISION_OP             => crate::print!("REVISION_OP "),
                DEBUG_OP                => crate::print!("DEBUG_OP "),
                FATAL_OP                => crate::print!("FATAL_OP "),
                TIMER_OP                => crate::print!("TIMER_OP "),
                OP_REGION_OP            => crate::print!("OP_REGION_OP "),
                FIELD_OP                => crate::print!("FIELD_OP "),
                DEVICE_OP               => crate::print!("DEVICE_OP "),
                POWER_RES_OP            => crate::print!("POWER_RES_OP "),
                THERMAL_ZONE_OP         => crate::print!("THERMAL_ZONE_OP "),
                INDEX_FIELD_OP          => crate::print!("INDEX_FIELD_OP "),
                BANK_FIELD_OP           => crate::print!("BANK_FIELD_OP "),
                DATA_REGION_OP          => crate::print!("DATA_REGION_OP "),
                _                       => unreachable!(),
            }
            *is_extended = false;
            return Ok(())
        }

        match aml_byte {
            ZERO_OP                 => crate::print!("ZERO_OP "),
            ONE_OP                  => crate::print!("ONE_OP "),
            ALIAS_OP                => crate::print!("ALIAS_OP "),
            NAME_OP                 => crate::print!("NAME_OP "),
            BYTE_PREFIX             => crate::print!("BYTE_PREFIX "),
            WORD_PREFIX             => crate::print!("WORD_PREFIX "),
            DWORD_PREFIX            => crate::print!("DWORD_PREFIX "),
            STRING_PREFIX           => crate::print!("STRING_PREFIX "),
            QWORD_PREFIX            => crate::print!("QWORD_PREFIX "),
            SCOPE_OP                => { crate::print!("SCOPE_OP "); Scope::_parse(ptr, aml_bytes, namespace); },
            BUFFER_OP               => crate::print!("BUFFER_OP "),
            PACKAGE_OP              => crate::print!("PACKAGE_OP "),
            VAR_PACKAGE_OP          => crate::print!("VAR_PACKAGE_OP "),
            METHOD_OP               => crate::print!("METHOD_OP "),
            EXTERNAL_OP             => crate::print!("EXTERNAL_OP "),
            DUAL_NAME_PREFIX        => crate::print!("DUAL_NAME_PREFIX "),
            MULTI_NAME_PREFIX       => crate::print!("MULTI_NAME_PREFIX "),
            // Marking next byte as extended.
            EXT_OP_PREFIX           => { crate::print!("EXT_OP_PREFIX "); *is_extended = true },
            ROOT_CHAR               => crate::print!("ROOT_CHAR "),
            PARENT_PREFIX_CHAR      => crate::print!("PARENT_PREFIX_CHAR "),
            NAME_CHAR               => crate::print!("NAME_CHAR "),
            LOCAL0_OP               => crate::print!("LOCAL0_OP "),
            LOCAL1_OP               => crate::print!("LOCAL1_OP "),
            LOCAL2_OP               => crate::print!("LOCAL2_OP "),
            LOCAL3_OP               => crate::print!("LOCAL3_OP "),
            LOCAL4_OP               => crate::print!("LOCAL4_OP "),
            LOCAL5_OP               => crate::print!("LOCAL5_OP "),
            LOCAL6_OP               => crate::print!("LOCAL6_OP "),
            LOCAL7_OP               => crate::print!("LOCAL7_OP "),
            ARG0_OP                 => crate::print!("ARG0_OP "),
            ARG1_OP                 => crate::print!("ARG1_OP "),
            ARG2_OP                 => crate::print!("ARG2_OP "),
            ARG3_OP                 => crate::print!("ARG3_OP "),
            ARG4_OP                 => crate::print!("ARG4_OP "),
            ARG5_OP                 => crate::print!("ARG5_OP "),
            ARG6_OP                 => crate::print!("ARG6_OP "),
            STORE_OP                => crate::print!("STORE_OP "),
            REFOF_OP                => crate::print!("REFOF_OP "),
            ADD_OP                  => crate::print!("ADD_OP "),
            CONCAT_OP               => crate::print!("CONCAT_OP "),
            SUBTRACT_OP             => crate::print!("SUBTRACT_OP "),
            INCREMENT_OP            => crate::print!("INCREMENT_OP "),
            DECREMENT_OP            => crate::print!("DECREMENT_OP "),
            MULTIPLY_OP             => crate::print!("MULTIPLY_OP "),
            DIVIDE_OP               => crate::print!("DIVIDE_OP "),
            SHIFT_LEFT_OP           => crate::print!("SHIFT_LEFT_OP "),
            SHIFT_RIGHT_OP          => crate::print!("SHIFT_RIGHT_OP "),
            AND_OP                  => crate::print!("AND_OP "),
            NAND_OP                 => crate::print!("NAND_OP "),
            OR_OP                   => crate::print!("OR_OP "),
            NOR_OP                  => crate::print!("NOR_OP "),
            XOR_OP                  => crate::print!("XOR_OP "),
            NOT_OP                  => crate::print!("NOT_OP "),
            FIND_SET_LEFT_BIT_OP    => crate::print!("FIND_SET_LEFT_BIT_OP "),
            FIND_SET_RIGHT_BIT_OP   => crate::print!("FIND_SET_RIGHT_BIT_OP "),
            DEREFOF_OP              => crate::print!("DEREFOF_OP "),
            CONCAT_RES_OP           => crate::print!("CONCAT_RES_OP "),
            MOD_OP                  => crate::print!("MOD_OP "),
            NOTIFY_OP               => crate::print!("NOTIFY_OP "),
            SIZEOF_OP               => crate::print!("SIZEOF_OP "),
            INDEX_OP                => crate::print!("INDEX_OP "),
            MATCH_OP                => crate::print!("MATCH_OP "),
            CREATE_DWORD_FIELD_OP   => crate::print!("CREATE_DWORD_FIELD_OP "),
            CREATE_WORD_FIELD_OP    => crate::print!("CREATE_WORD_FIELD_OP "),
            CREATE_BYTE_FIELD_OP    => crate::print!("CREATE_BYTE_FIELD_OP "),
            CREATE_BIT_FIELD_OP     => crate::print!("CREATE_BIT_FIELD_OP "),
            OBJECT_TYPE_OP          => crate::print!("OBJECT_TYPE_OP "),
            CREATE_QWORD_FIELD_OP   => crate::print!("CREATE_QWORD_FIELD_OP "),
            LAND_OP                 => crate::print!("LAND_OP "),
            LOR_OP                  => crate::print!("LOR_OP "),
            LNOT_OP                 => crate::print!("LNOT_OP "),
            LEQUAL_OP               => crate::print!("LEQUAL_OP "),
            LGREATER_OP             => crate::print!("LGREATER_OP "),
            LLESS_OP                => crate::print!("LLESS_OP "),
            TO_BUFFER_OP            => crate::print!("TO_BUFFER_OP "),
            TO_DECIMAL_STRING_OP    => crate::print!("TO_DECIMAL_STRING_OP "),
            TO_HEX_STRING_OP        => crate::print!("TO_HEX_STRING_OP "),
            TO_INTEGER_OP           => crate::print!("TO_INTEGER_OP "),
            TO_STRING_OP            => crate::print!("TO_STRING_OP "),
            COPY_OBJECT_OP          => crate::print!("COPY_OBJECT_OP "),
            MID_OP                  => crate::print!("MID_OP "),
            CONTINUE_OP             => crate::print!("CONTINUE_OP "),
            IF_OP                   => crate::print!("IF_OP "),
            ELSE_OP                 => crate::print!("ELSE_OP "),
            WHILE_OP                => crate::print!("WHILE_OP "),
            NOOP_OP                 => crate::print!("NOOP_OP "),
            RETURN_OP               => crate::print!("RETURN_OP "),
            BREAK_OP                => crate::print!("BREAK_OP "),
            BREAKPOINT_OP           => crate::print!("BREAKPOINT_OP "),
            ONES_OP                 => crate::print!("ONES_OP "),
            b @ _                   => crate::print!("{:x} ", b),
        };
        *ptr += 1;
        if *ptr == 30 {
            unimplemented!()
        }
        Ok(())
    }
}

/// A custom error type related to AML parsing. If some token cannot be parsed into a logically
/// correct AML Namespace Object, then the error is throwed to the parser.
pub enum AMLParserError {
    /// The token obtained was not expected when parsing some specific structure.
    UnexpectedToken,
    /// The stream ended too early.
    UnexpectedEndOfStream,
    /// There was not enough bytes expected to read something.
    NotEnoughBytes,
    /// Reserved bits was expected to have different value, than the one found.
    InvalidResevedBits,
    /// Some layer or data was expected to be already within a namespace.
    NotInNamespace,
}
