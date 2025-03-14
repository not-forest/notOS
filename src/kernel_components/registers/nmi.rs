/// Module for low level management of NMI related registers.

use crate::kernel_components::arch_x86_64::ports::{GenericPort, PortAccessType};
use crate::bitflags;


bitflags! {
    /// NMI Status and Control.
    ///
    /// Used to perform operations related to non maskable interrupt, but also allows to manage the
    /// PC Speaker, connected to the second timer output of PIT timer.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct NMI_STS_CNT: u8 {
        /// Enables PIT's second output when set to 1, which is connected to the PC Speaker. 
        /// This will cause the speaker to beep with the frequency PIT's second timer output
        /// if the SPKR_DAT_EN bit is also set.
        const TIM_CNT2_EN =     1 << 0,
        /// PC Speaker output will be equivalent to the output signal value of the PIT's second
        /// timer output. It's output will be zero, if this bit is set to zero.
        const SPKR_DAT_EN =     1 << 1,
        /// When set, system errors triggered by PCI won't cause an NMI interrupt.
        const PCI_SETT_EN =     1 << 2,
        /// Controls whether I/O channel check NMI are enabled or not. Disabled when set, otherwise
        /// the system will generate NMI if an I/O channel check error occurs.
        const IOCHK_NMI_EN =    1 << 3,
        /// Reflects the current state of PIT's second timer's output.
        const TIM2_OUT_STS =    1 << 5,
        /// Indicates whether an I/O channel check NMI has been triggered, typically by ISA device
        /// using serialized IRQ.
        const IOCHK_NMI_STS =   1 << 6,
        /// Indicates whether a system error NMI has been triggered. This may be caused by the
        /// following:
        /// - errors from secondaty PCI buses;
        /// - errors from PCI ports;
        /// - DMI errors.
        const SERR_NMI_STS =    1 << 7,
    }
}

impl NMI_STS_CNT {
    /// Read the current value of NMI_STS_CNT register.
    ///
    /// This register is mapped behind the I/O port on offset 61.
    #[inline]
    pub fn read() -> Self {
        let p = GenericPort::<u8>::new(0x61, PortAccessType::READWRITE);
        p.read().into()
    }

    /// Writes NMI_STS_CNT with new value.
    ///
    /// This register is mapped behind the I/O port on offset 61.
    #[inline]
    pub fn write(nmi_sts_cnt: Self) {
        let p = GenericPort::<u8>::new(0x61, PortAccessType::READWRITE);
        p.write(nmi_sts_cnt.bits() & 0x0f); // Top RO nybble shall always be 0 according to Intel datasheet. 
    }
}

impl Default for NMI_STS_CNT {
    fn default() -> Self {
        NMI_STS_CNT::TIM2_OUT_STS
    }
}
