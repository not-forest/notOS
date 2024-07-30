/// Package length encoding

use crate::{bitflags, kernel_components::arch_x86_64::acpi::diff::namespace::ACPINamespace};

use super::{aml_parser::Parsed, AMLParserError};

/// A variable-length field used to encode the length of oncoming package.
///
/// 
///  PkgLength := PkgLeadByte |
/// <PkgLeadByte ByteData> |
/// <PkgLeadByte ByteData ByteData> |
/// <PkgLeadByte ByteData ByteData ByteData>
///
/// It can consist of 1 to 4 bytes, depending on the value it represents.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct PkgLength(u32);

impl Parsed for PkgLength {
    fn parse(ptr: &mut usize, bytes: &'static [u8], _: &mut ACPINamespace) -> super::aml_parser::AMLParserResult<Self> {
        let lead_byte = bytes[1]; // Starting from 1 because 0 is the opcode.
        let follow_byte_count = (lead_byte >> 6) & 0b11; // Bits 7-6
        let reserved_bits = (lead_byte >> 4) & 0b11;     // Bits 5-4
        let mut lsb = (lead_byte & 0x0f) as u32;         // Bits 3-0

        if follow_byte_count == 0 {
            // If package size is described with only one byte, then bits 5-0 define the size.
            lsb |= (reserved_bits as u32) << 4;
        } else {
            // Reserved bits must be zero for multi-byte encoding.
            if reserved_bits != 0 && follow_byte_count > 0 {
                return Err(AMLParserError::InvalidResevedBits) 
            }

            // If length is not enough to read following bytes, then something went wrong.
            if follow_byte_count as usize > bytes.len() - 1 {
                return Err(AMLParserError::NotEnoughBytes)
            }

            /// Bits 3-0 are LSB. Next 8 bits of n bytes are the next LSB.
            for i in 2..follow_byte_count + 2 {
                lsb |= (bytes[i as usize + 1] as u32) << (4 + ((i - 2) * 8)); // Each next byte pushed.
            }
        }

        *ptr += follow_byte_count as usize + 2;  // This is end of PkgLength
        Ok(PkgLength(lsb))
    }
}
