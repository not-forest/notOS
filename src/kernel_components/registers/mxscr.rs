/// This register contains control and status information for the SSE registers. 
/// 
/// Some of the bits in this register are editable. You cannot dive in these values.
 
use crate::bitflags;
use core::arch::asm;

bitflags! {
    /// This register contains control and status information for the SSE registers. 
    /// 
    /// Some of the bits in this register are editable. You cannot dive in these values
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct MxCsr: u32 {
        const FLUSH_TO_ZERO =                          1 << 15,
        const TO_NEAREST_ROUNDING_MODE =               0x0000,
        const TOWARD_NEGATIVE_INFINITY_ROUNDING_MODE = 0x2000,
        const TOWARD_POSITIVE_INFINITY_ROUNDING_MODE = 0x4000,
        const TOWARD_ZERO_ROUNDING_MODE =              0x6000,
        const PRECISION_MASK =                         1 << 12,
        const UNDERFLOW_MASK =                         1 << 11,
        const OVERFLOW_MASK =                          1 << 10,
        const DIVIDE_TO_ZERO_MASK =                    1 << 9,
        const DENORMAL_MASK =                          1 << 8,
        const INVALID_OPERATION_MASK =                 1 << 7,
        const DENORMALS_ARE_ZERO =                     1 << 6,
        const PRECISION_FLAG =                         1 << 5,
        const UNDERFLOW_FLAG =                         1 << 4,
        const OVERFLOW_FLAG =                          1 << 3,
        const DIVIDE_BY_ZERO_FLAG =                    1 << 2,
        const DENORMAL_FLAG =                          1 << 1,
        const INVALID_OPERATION_FLAG =                 1,
    }
}

impl Default for MxCsr {
    /// The default MXCSR value at reset.
    #[inline]
    fn default() -> Self {
        use MxCsr::*;
        INVALID_OPERATION_MASK |
        DENORMAL_MASK          |
        DIVIDE_TO_ZERO_MASK    |
        OVERFLOW_MASK          |
        UNDERFLOW_MASK         |
        PRECISION_MASK         
    }
}

impl MxCsr {
    /// Reads the value of MXCSR register currently.
    #[inline]
    pub fn read() -> MxCsr {
        let mut mxcsr = 0;

        unsafe {
            asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack, preserves_flags));
        }
        MxCsr::from_bits_truncate(mxcsr).into()
    }

    /// Writes MXCSR with new value.
    #[inline]
    pub fn write(mxcsr: MxCsr) {
        unsafe {
            asm!("ldmxcsr [{}]", in(reg) &mxcsr, options(nostack, readonly));
        }
    }
}