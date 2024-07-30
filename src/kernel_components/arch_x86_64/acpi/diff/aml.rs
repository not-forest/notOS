/// Custom module that defines AML (ACPI Machine Language) specific features for differentiated description tables.
///
/// AML code is a byte code which is parsed from the beginning of each table when that table is
/// read. This code is found in DSDT and SSDT differentiated tables. A parser is required to obtain
/// an AML Namespace from the AML code written in those tables, meanwhile an AML interpreter is
/// required for running specific methods defined in the Namespace by operating on memory mapped
/// registers pointed by the FADT table. 

use alloc::{collections::BTreeMap, string::String};
use core::{fmt::Debug, mem, sync::atomic::AtomicUsize};
use crate::kernel_components::os::UChar;

use super::objects::ACPIObject;

/// Special result type for AML related functions. AML result can be obtained from parser, interpreter
/// namespace and helper structures/functions.
pub type AMLResult<T> = Result<(), T>;

/// Custom structure that defines an AML bytecode stream.
///
/// This structure allows for parsing and decoding the AML bytecode without the need of AML
/// interpreter.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AMLStream(pub &'static [u8]);

// Four-letter variable name. All objects are defined by such name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NameSeg(pub [UChar; 4]);

impl NameSeg {
    /// Creates a NameSeg with filled bytes.
    pub fn new(b1: u8, b2: u8, b3: u8, b4: u8) -> Self {
        Self(
            unsafe{
                mem::transmute([b1, b2, b3, b4])
            }
        )
    }
}

/* All possible encoded byte tokens found it AML stream. */
pub(crate) const ZERO_OP: u8 = 0x00;
pub(crate) const ONE_OP: u8 = 0x01;
pub(crate) const ALIAS_OP: u8 = 0x06;
pub(crate) const NAME_OP: u8 = 0x08;
pub(crate) const BYTE_PREFIX: u8 = 0x0A;
pub(crate) const WORD_PREFIX: u8 = 0x0B;
pub(crate) const DWORD_PREFIX: u8 = 0x0C;
pub(crate) const STRING_PREFIX: u8 = 0x0D;
pub(crate) const QWORD_PREFIX: u8 = 0x0E;
pub(crate) const SCOPE_OP: u8 = 0x10;
pub(crate) const BUFFER_OP: u8 = 0x11;
pub(crate) const PACKAGE_OP: u8 = 0x12;
pub(crate) const VAR_PACKAGE_OP: u8 = 0x13;
pub(crate) const METHOD_OP: u8 = 0x14;
pub(crate) const EXTERNAL_OP: u8 = 0x15;
pub(crate) const DUAL_NAME_PREFIX: u8 = 0x2E;
pub(crate) const MULTI_NAME_PREFIX: u8 = 0x2F;
pub(crate) const EXT_OP_PREFIX: u8 = 0x5B;
pub(crate) const ROOT_CHAR: u8 = 0x5C;
pub(crate) const PARENT_PREFIX_CHAR: u8 = 0x5E;
pub(crate) const NAME_CHAR: u8 = 0x5F;
pub(crate) const LOCAL0_OP: u8 = 0x60;
pub(crate) const LOCAL1_OP: u8 = 0x61;
pub(crate) const LOCAL2_OP: u8 = 0x62;
pub(crate) const LOCAL3_OP: u8 = 0x63;
pub(crate) const LOCAL4_OP: u8 = 0x64;
pub(crate) const LOCAL5_OP: u8 = 0x65;
pub(crate) const LOCAL6_OP: u8 = 0x66;
pub(crate) const LOCAL7_OP: u8 = 0x67;
pub(crate) const ARG0_OP: u8 = 0x68;
pub(crate) const ARG1_OP: u8 = 0x69;
pub(crate) const ARG2_OP: u8 = 0x6A;
pub(crate) const ARG3_OP: u8 = 0x6B;
pub(crate) const ARG4_OP: u8 = 0x6C;
pub(crate) const ARG5_OP: u8 = 0x6D;
pub(crate) const ARG6_OP: u8 = 0x6E;
pub(crate) const STORE_OP: u8 = 0x70;
pub(crate) const REFOF_OP: u8 = 0x71;
pub(crate) const ADD_OP: u8 = 0x72;
pub(crate) const CONCAT_OP: u8 = 0x73;
pub(crate) const SUBTRACT_OP: u8 = 0x74;
pub(crate) const INCREMENT_OP: u8 = 0x75;
pub(crate) const DECREMENT_OP: u8 = 0x76;
pub(crate) const MULTIPLY_OP: u8 = 0x77;
pub(crate) const DIVIDE_OP: u8 = 0x78;
pub(crate) const SHIFT_LEFT_OP: u8 = 0x79;
pub(crate) const SHIFT_RIGHT_OP: u8 = 0x7A;
pub(crate) const AND_OP: u8 = 0x7B;
pub(crate) const NAND_OP: u8 = 0x7C;
pub(crate) const OR_OP: u8 = 0x7D;
pub(crate) const NOR_OP: u8 = 0x7E;
pub(crate) const XOR_OP: u8 = 0x7F;
pub(crate) const NOT_OP: u8 = 0x80;
pub(crate) const FIND_SET_LEFT_BIT_OP: u8 = 0x81;
pub(crate) const FIND_SET_RIGHT_BIT_OP: u8 = 0x82;
pub(crate) const DEREFOF_OP: u8 = 0x83;
pub(crate) const CONCAT_RES_OP: u8 = 0x84;
pub(crate) const MOD_OP: u8 = 0x85;
pub(crate) const NOTIFY_OP: u8 = 0x86;
pub(crate) const SIZEOF_OP: u8 = 0x87;
pub(crate) const INDEX_OP: u8 = 0x88;
pub(crate) const MATCH_OP: u8 = 0x89;
pub(crate) const CREATE_DWORD_FIELD_OP: u8 = 0x8A;
pub(crate) const CREATE_WORD_FIELD_OP: u8 = 0x8B;
pub(crate) const CREATE_BYTE_FIELD_OP: u8 = 0x8C;
pub(crate) const CREATE_BIT_FIELD_OP: u8 = 0x8D;
pub(crate) const OBJECT_TYPE_OP: u8 = 0x8E;
pub(crate) const CREATE_QWORD_FIELD_OP: u8 = 0x8F;
pub(crate) const LAND_OP: u8 = 0x90;
pub(crate) const LOR_OP: u8 = 0x91;
pub(crate) const LNOT_OP: u8 = 0x92;
pub(crate) const LEQUAL_OP: u8 = 0x93;
pub(crate) const LGREATER_OP: u8 = 0x94;
pub(crate) const LLESS_OP: u8 = 0x95;
pub(crate) const TO_BUFFER_OP: u8 = 0x96;
pub(crate) const TO_DECIMAL_STRING_OP: u8 = 0x97;
pub(crate) const TO_HEX_STRING_OP: u8 = 0x98;
pub(crate) const TO_INTEGER_OP: u8 = 0x99;
pub(crate) const TO_STRING_OP: u8 = 0x9C;
pub(crate) const COPY_OBJECT_OP: u8 = 0x9D;
pub(crate) const MID_OP: u8 = 0x9E;
pub(crate) const CONTINUE_OP: u8 = 0x9F;
pub(crate) const IF_OP: u8 = 0xA0;
pub(crate) const ELSE_OP: u8 = 0xA1;
pub(crate) const WHILE_OP: u8 = 0xA2;
pub(crate) const NOOP_OP: u8 = 0xA3;
pub(crate) const RETURN_OP: u8 = 0xA4;
pub(crate) const BREAK_OP: u8 = 0xA5;
pub(crate) const BREAKPOINT_OP: u8 = 0xCC;
pub(crate) const ONES_OP: u8 = 0xFF;

/* Extended AML encoded byte token. */
// Extended byte 0x5B
pub(crate) const MUTEX_OP: u8 = 0x00;
pub(crate) const EVENT_OP: u8 = 0x01;
pub(crate) const COND_REFOF_OP: u8 = 0x12;
pub(crate) const CREATE_FIELD_OP: u8 = 0x13;
pub(crate) const LOAD_TABLE_OP: u8 = 0x1F;
pub(crate) const LOAD_OP: u8 = 0x20;
pub(crate) const STALL_OP: u8 = 0x21;
pub(crate) const SLEEP_OP: u8 = 0x22;
pub(crate) const ACQUIRE_OP: u8 = 0x23;
pub(crate) const SIGNAL_OP: u8 = 0x24;
pub(crate) const WAIT_OP: u8 = 0x25;
pub(crate) const RESET_OP: u8 = 0x26;
pub(crate) const RELEASE_OP: u8 = 0x27;
pub(crate) const FROM_BCD_OP: u8 = 0x28;
pub(crate) const TO_BCD: u8 = 0x29;
pub(crate) const REVISION_OP: u8 = 0x30;
pub(crate) const DEBUG_OP: u8 = 0x31;
pub(crate) const FATAL_OP: u8 = 0x32;
pub(crate) const TIMER_OP: u8 = 0x33;
pub(crate) const OP_REGION_OP: u8 = 0x80;
pub(crate) const FIELD_OP: u8 = 0x81;
pub(crate) const DEVICE_OP: u8 = 0x82;
pub(crate) const POWER_RES_OP: u8 = 0x84;
pub(crate) const THERMAL_ZONE_OP: u8 = 0x85;
pub(crate) const INDEX_FIELD_OP: u8 = 0x86;
pub(crate) const BANK_FIELD_OP: u8 = 0x87;
pub(crate) const DATA_REGION_OP: u8 = 0x88;
// Extended byte 0x92
pub(crate) const LNOT_EQUAL_OP: u8 = 0x93;
pub(crate) const LLESS_EQUAL_OP: u8 = 0x94;
pub(crate) const LGREATER_EQUAL_OP: u8 = 0x95;
