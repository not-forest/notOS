use core::mem;

/// Module that allows to manage RTC chip and manipulate with it's 64 bytes of CMOS RAM.

use crate::{
    critical_section, 
    kernel_components::arch_x86_64::{
        ports::{GenericPort, PortAccessType},
        post::DEBUG_BOARD,
    },
};

/// Real Time Clock
///
/// Structure that allows to manipulate on RTC chip and it's internal power static memory. RTC
/// keeps track of the date and time, even when the computer's power is off. CMOS memory exists
/// outside the normal address space and can be reachable from this chip via reading and writing
/// values into two ports.
///
/// # Warn
///
/// When programming this chip all interrupts must be disabled, including the NMI. Both ports must
/// be used to prevent 'undefined' state of the chip, which preserves on reboot. Use safe functions
/// to prevent such behavior. Any action on port 0x70 must be followed by an action on port 0x71.
///
/// # NMI
///
/// All configuration functions will turn off NMI interrupts and do not restore it, until manually
/// enabled via 'enable_nmi' method.
pub struct RTC {
    /// Port 0x70, which is used to select an index within the CMOS memory to read/write from
    /// and/or enabling or disabling NMIs. The CMOS memory is 64 bytes long, therefore values
    /// higher that this will mirror it's smaller counterparts, like in a circular buffer. 
    index: GenericPort<u8>,
    /// Port 0x71, which is used to read/write actual values within the CMOS memory. Even though it
    /// is RW, writing anything other than to calibrate the RTC is a very bad idea.
    data: GenericPort<u8>,
}

impl RTC {
    /// Creates a new instance of RTC.
    pub const fn new() -> Self {
        Self {
            index: GenericPort::new(0x70, PortAccessType::WRITEONLY),
            data: GenericPort::new(0x71, PortAccessType::READWRITE),
        }
    }

    /// Checking the current state of RTC clock's battery.
    ///
    /// Will return true if the battery is charged and RTC is working. If the battery is dead or
    /// disconnected, will return false.
    pub fn is_powered(&self) -> bool {
        self.read(CMOSAddr::RTC_STATUS_D) >> 7 > 0
    }

    /// Enables NMI interrupts.
    pub fn enable_nmi(&mut self) {
        self.read(unsafe{mem::transmute(1u8 << 7)});
    }

    /// Reads a value written inside the CMOS memory under a specific address provided.
    ///
    /// This function is always safe, because it ensures that all interrupts are off. The OS should
    /// not shutdown during this function, because it writes value to the index port and reads the
    /// requested byte from the data port. This operation must be atomic.
    pub fn read(&self, addr: CMOSAddr) -> u8 {
        critical_section!(|| {
            self.index.write(addr as u8);
            DEBUG_BOARD.write(0); // Small delay.
            self.data.read()
        })
    }

    /// Writes some byte to the CMOS memory under a specific address provided.
    ///
    /// # Unsafe
    ///
    /// As mentioned above, only writes to the RTC configuration (status registers A/B) are safe.
    /// Writing values to other memory fields are most likely to create a mess in the system.
    pub unsafe fn write(&mut self, addr: CMOSAddr, byte: u8) {
        critical_section!(|| {
            self.index.write(addr as u8);
            DEBUG_BOARD.write(0); // Small delay.
            self.data.write(byte);
        })
    }
}

/// Defines indexes of the CMOS RAM.
///
/// Those values must be used to read or write data from the RTC's memory. Most of them are read
/// only, except for RTC status registers A and B. 
///
/// Not all addresses are consistent, and most of them are chip-specific, therefore a custom byte 
/// should be used to match a specific need. For example a RTC register that provides information 
/// about current century might exist on the chip. To obtain a proper index, one should find it
/// inside the FADT ACPI table. If it is some value other than zero, than this value is the index
/// of this register. 
#[repr(u8)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CMOSAddr {
    /* RTC Clock Related Registers */

    /// Current time (seconds) [R]
    RTC_SECONDS,
    ///
    RTC_SECOND_ALARM,
    /// Current time (minutes) [R]
    RTC_MINUTES,
    /// 
    RTC_MINUTE_ALARM,
    /// Current time (hours) [R]
    RTC_HOURS,
    RTC_HOUR_ALARM,
    /// Current date (day of the week) [R]
    RTC_DAY_OF_WEEK,
    /// Current date (day of the month) [R]
    RTC_DAY_OF_MONGTH,
    /// Current date (current month) [R]
    RTC_MONTH,
    /// Current date (current year) [R]
    RTC_YEAR,
    /// RTC's A status register. [RW]
    ///
    /// Allows to configure RTC's frequency by changing the interrupt rate. Also holds bits for 22
    /// stage divider.
    RTC_STATUS_A = 0x0a,
    /// RTC's B status register. [RW]
    ///
    /// Allows to configure different flags and modes for the RTC.
    RTC_STATUS_B = 0x0b,
    /// RTC's C status register. [R]
    ///
    /// A read-only register that holds information in form of flags about different interrupts.
    RTC_STATUS_C = 0x0c,
    /// RTC's D status register. [R]
    ///
    /// A read-only register with one flag, that defines the current state of RTC's battery.
    RTC_STATUS_D = 0x0d,

    /* Non Clock Registers */

}

