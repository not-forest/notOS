use core::mem;

/// Module that allows to manage RTC chip and manipulate with it's 64 bytes of CMOS RAM.

use crate::{
    bitflags, critical_section, kernel_components::arch_x86_64::{
        ports::{GenericPort, PortAccessType},
        post::DEBUG_BOARD,
    }
};

/// Real Time Clock
///
/// Structure that allows to manipulate on RTC chip and it's internal power static memory. RTC
/// keeps track of the date and time, even when the computer's power is off. CMOS memory exists
/// outside the normal address space and can be reachable from this chip via reading and writing
/// values into two ports. The chip can also generate interrupts, by generating IRQ8 on PIC. It's
/// behavior can be configured via status register A and B.
///
/// The clock's oscillator is set to 32768 Hz by default, which generates a real time second. This
/// frequency can be reprogrammed as well as prescaled via it's status registors, however it will
/// completely ruin it's purpose.
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
    /// This register must be read after IRQ8 interrupt, otherwise another one won't be called. 
    RTC_STATUS_C = 0x0c,
    /// RTC's D status register. [R]
    ///
    /// A read-only register with one flag, that defines the current state of RTC's battery.
    RTC_STATUS_D = 0x0d,

    /* Non Clock Registers */

}

bitflags! {
    /// RTC Status A register.
    ///
    /// A first configuration registor, which can be used to control the frequency of upcoming
    /// interrupts from the chip. 
    ///
    /// # Periodic Interrupt Rate (bits: 3 - 0) [RW]
    ///
    /// Those bits define the amount of interrupts generated by RTC per ?second?. If prescaler bits
    /// (bits: 6 - 4) are untouched, then it is exactly the amount per second. These bits only
    /// allow to change interrupt frequency without affecting clock's ability to count time
    /// properly.
    ///
    /// # Prescaler (bits: 6 - 4) [RW]
    ///
    /// Defines a prescaler value for RTC crystal oscillator. One most likely would wan't it to be
    /// a default value, which is '010'. Bigger values would slow down the clock. Logically,
    /// smaller values would speed it up. From that point, it is not a real time clock anymore.
    /// Such behavior is defined as "time machine" starting from this line.
    ///
    /// # Update In Progress (UIP) Flag (bit: 7) [R]
    ///
    /// When set an update cycle is in progress and the clock/calendar cannot be accessed. When clear, 
    /// at least 244 microseconds are available to access clock/calendar bytes. This bit is
    /// supposed to be read-only.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct RCTStatusA: u8 {
        /* BITS: 3 - 0. Define the periodic interrupt rate. */

        /// Two interrupts per second (if not time machine)
        const INTSEC2               = 0b1111,
        /// Four interrupts per second (if not time machine)
        const INTSEC4               = 0b1110,
        /// Eight interrupts per second (if not time machine)
        const INTSEC8               = 0b1101,
        /// 16 interrupts per second (if not time machine)
        const INTSEC16              = 0b1100,
        /// 32 interrupts per second (if not time machine)
        const INTSEC32              = 0b1011,
        /// 64 interrupts per second (if not time machine)
        const INTSEC64              = 0b1010,
        /// 128 interrupts per second (if not time machine)
        const INTSEC128             = 0b1001,
        /// 256 interrupts per second (if not time machine)
        const INTSEC256             = 0b1000,
        /// 512 interrupts per second (if not time machine)
        const INTSEC512             = 0b0111,
        /// 1024 interrupts per second (if not time machine). DEFAULT VALUE.
        const INTSEC1024            = 0b0110,
        /// 2048 interrupts per second (if not time machine)
        const INTSEC2048            = 0b0101,
        /// 4096 interrupts per second (if not time machine)
        const INTSEC4096            = 0b0100,
        /// 8192 interrupts per second (if not time machine)
        const INTSEC8192            = 0b0011,
        /// No interrupts at all. Even if they are enabled.
        const INTSECNONE            = 0b0000,

        /* BITS: 6 - 4. Define prescaler value. */

        /// Biggest prescaled frequency.
        const TIME_FASTEST          = 0b000,
        /// Second biggest prescaled frequency.
        const TIME_FAST             = 0b001, 
        /// Divides the clock perfectly to obtain one real time second. DEFAULT VALUE.
        const TIME_REAL             = 0b010,
        /// Fourth smallest prescaled frequency.
        const TIME_SLOW             = 0b011,
        /// Third smallest prescaled frequency.
        const TIME_SLOTH            = 0b100,
        /// Second smallest prescaled frequency.
        const TIME_SNAIL            = 0b110,
        /// Smallest prescaled frequency.
        const TIME_MATRIX           = 0b111,
        
        /// Update In Progress (UIP) flag
        ///
        /// When set an update cycle is in progress and the clock/calendar cannot be accessed. When clear, 
        /// at least 244 microseconds are available to access clock/calendar bytes. This bit is
        /// supposed to be read-only.
        const UIP                   = 1 << 7,
    }
}
