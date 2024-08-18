/// Module that allows to manage RTC chip and manipulate with it's 64 bytes of CMOS RAM. It also
/// allows to access the NMI enable bit and several other hardware specific information bytes,
/// which are mapped to CMOS chip memory.

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
/// # Interrupts
///
/// RTC interrupt pin is connected to IRQ8 line. The chip can generate three different types of 
/// interrupts: 
/// - update-ended: 
/// This is the simplest type - interrupt is generated after each clock update exactly every 1 second.
/// - alarm:
/// This is a second type - it generates interrupt at time specified in alarm registers. For
/// example 00:00.00 will cause one interrupt when it is 0 AM. FF:FF.00 will generate alarm
/// interrupt every minute etc. 
/// - periodic:
/// The frequency of this interrupt is programmable from 2 to 8192 per second (if real time frequency).
///
/// To use RTC interrupt first install interrupt service routine and remap PIC controller vector, then 
/// program RTC status registers A and B. More than one interrupt type can be enabled at the same time 
/// (configured in status register B), in that case your interrupt handler should check which type has 
/// occurred (by reading status register C).
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
    /// A boolean that defines if NMI interrupt must be enabled or not. Disabled by default.
    nmi: bool,
}

impl RTC {
    /// Creates a new instance of RTC.
    pub const fn new() -> Self {
        Self {
            index: GenericPort::new(0x70, PortAccessType::WRITEONLY),
            data: GenericPort::new(0x71, PortAccessType::READWRITE),
            nmi: false,
        }
    }

    /// Reads the status A register value.
    pub fn status_a(&self) -> RTCStatusA {
        self.read(CMOSAddr::RTC_STATUS_A).into()
    }

    /// Reads the status B register value.
    pub fn status_b(&self) -> RTCStatusB {
        self.read(CMOSAddr::RTC_STATUS_B).into()
    }

    /// Reads the status C register value.
    ///
    /// Must be used in the end of IRQ8 handler function so that new interrupts can occur. On each
    /// read the register will be also be cleared fully.
    pub fn status_c(&self) -> RTCStatusC {
        self.read(CMOSAddr::RTC_STATUS_C).into()
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
        self.nmi = true;
        self.read(0.into());
    }

    /// Disables NMI interrupts.
    pub fn disable_nmi(&mut self) {
        self.nmi = false;
        self.read(0.into());
    }

    /// Reads a value written inside the CMOS memory under a specific address provided.
    ///
    /// This function is always safe, because it ensures that all interrupts are off. The OS should
    /// not shutdown during this function, because it writes value to the index port and reads the
    /// requested byte from the data port. This operation must be atomic.
    pub fn read(&self, addr: CMOSAddr) -> u8 {
        critical_section!(|| {
            self.index.write(addr.bits() | (self.nmi as u8) << 7);
            DEBUG_BOARD.write(0); // Small delay.
            self.data.read()
        })
    }

    /// Writes some byte to the CMOS memory under a specific address provided.
    ///
    /// # Unsafe
    ///
    /// As mentioned above, only writes to the RTC configuration (status registers A/B) are safe.
    /// Writing values to other memory fields are most likely to create a mess in the system. When
    /// writing data, NMI will always be disabled.
    pub unsafe fn write(&mut self, addr: CMOSAddr, byte: u8) {
        critical_section!(|| {
            self.index.write(addr.bits());
            DEBUG_BOARD.write(0); // Small delay.
            self.data.write(byte);
        })
    }

    /// Writes some byte to the CMOS memory under a specific address preserving all bytes specified
    /// in 'preserve' variable.
    ///
    /// All bits specified in 'preserve' byte variable will be preserved when writing 'byte' to the
    /// CMOS memory. The address byte resets each time we read or write values, therefore we have
    /// to set address twice on each read and write.
    ///
    /// # Unsafe
    ///
    /// As mentioned above, only writes to the RTC configuration (status registers A/B) are safe.
    /// Writing values to other memory fields are most likely to create a mess in the system.
    pub unsafe fn write_preserved(&mut self, addr: CMOSAddr, byte: u8, preserve: u8) {
        let b = self.read(addr);
        self.write(addr, (byte & !preserve) | (b & preserve));
    }
}

/// Converts the BCD format to regular binary.
///
/// Binary format is exactly what you would expect the time and date value to be. If the time is 
/// 1:59:48 AM, then the value of hours would be 1, minutes would be 59 = 0x3b, and seconds would 
/// be 48 = 0x30. In BCD format the same values would be represented as so: 1:59:48 has hours = 1, 
/// minutes = 0x59 = 89, seconds = 0x48 = 72.
///
/// # Use Case
///
/// Only use this function if your chip doesn't allow to modify the status register B within the
/// CMOS memory space. Change the DATA_MODE bit in the status register to force chip to represent
/// values in proper format.
pub fn bcd2bin(bcd: u8) -> u8 {
    ((bcd & 0xF0) >> 1) + ( (bcd & 0xF0) >> 3) + (bcd & 0xf)
}

bitflags! {  
    /// Defines indexes of the CMOS RAM.
    ///
    /// Those values must be used to read or write data from the RTC's memory. Usually only status
    /// register A and B used for configuration purposes and first 10 addresses for obtaining data
    /// from the clock about the current time.
    ///
    /// Not all addresses are consistent, and most of them are chip-specific, therefore a custom byte 
    /// should be used to match a specific need. For example a RTC register that provides information 
    /// about current century might exist on the chip. To obtain a proper index, one should find it
    /// inside the FADT ACPI table. If it is some value other than zero, than this value is the index
    /// of this register.
    #[allow(non_camel_case_types)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct CMOSAddr: u8 {
        /* RTC Clock Related Registers */

        /// Current time (seconds) [RW]
        const RTC_SECONDS                                   = 0x00,
        /// Alarm value for seconds.
        const RTC_SECOND_ALARM                              = 0x01,
        /// Current time (minutes) [RW]
        const RTC_MINUTES                                   = 0x02,
        /// Alarm value for minutes.
        const RTC_MINUTE_ALARM                              = 0x03,
        /// Current time (hours) [RW]
        const RTC_HOURS                                     = 0x04,
        /// Alarm value for hours.
        const RTC_HOUR_ALARM                                = 0x05,
        /// Current date (day of the week) [RW]
        const RTC_DAY_OF_WEEK                               = 0x06,
        /// Current date (day of the month) [RW]
        const RTC_DAY_OF_MONGTH                             = 0x07,
        /// Current date (current month) [RW]
        const RTC_MONTH                                     = 0x08,
        /// Current date (current year) [RW]
        const RTC_YEAR                                      = 0x09,
        /// RTC's A status register. [RW]
        ///
        /// Allows to configure RTC's frequency by changing the interrupt rate. Also holds bits for 22
        /// stage divider.
        const RTC_STATUS_A                                  = 0x0a,
        /// RTC's B status register. [RW]
        ///
        /// Allows to configure different flags and modes for the RTC.
        const RTC_STATUS_B                                  = 0x0b,
        /// RTC's C status register. [R]
        ///
        /// A read-only register that holds information in form of flags about different interrupts.
        /// This register must be read after IRQ8 interrupt, otherwise another one won't be called. 
        const RTC_STATUS_C                                  = 0x0c,
        /// RTC's D status register. [R]
        ///
        /// A read-only register with one flag, that defines the current state of RTC's battery.
        const RTC_STATUS_D                                  = 0x0d,

        /* Non Clock Registers */
        /// Diagnostic information from POST about RTC and CMOS.
        const POST_DIAGNOSTIC_STATUS                        = 0x0e,
        /// This byte is read upon startup after CPU reset in order to determine if the reset cause 
        /// (to get out of protected mode etc.)
        const SHUTDOWN_STATUS                               = 0xef,

        /* Memory/Drives/Diskettes */
        /// Old way of defining amount of memory in connected diskette.   
        const DISKETTE_DRIVE_TYPE                           = 0x10,
        // 0x11 is reserved.
        /// Old way of defining amount of memory in connected disk.
        const HARD_DRIVE_TYPE                               = 0x12,
        // 0x13 is reserved.
        /// 
        const EQUIPMENT                                     = 0x14,
        /// Low bytes of base memory size in kbytes.
        const BASE_MEMORY_LOW                               = 0x15,
        /// High bytes of base memory size in kbytes.
        const BASE_MEMORY_HIGH                              = 0x16,        
        /// Extended memory size above 1M. in kbytes.
        const EXTENDED_MEMORY_LOW                           = 0x17,
        /// Extended memory size above 1M. in kbytes.
        const EXTENDED_MEMORY_HIGH                          = 0x18,
        /// Disk 0 type if (CMOS addr 12H & 0fH) is 0fH
        const DISK_0_TYPE                                   = 0x19,
        /// Disk 1 type if (CMOS addr 12H & f0H) is f0H
        const DISK_1_TYPE                                   = 0x1a,
        // 0x1b - 0x2d are reserved.
        /// Checksum of CMOS addresses 10H through 20H
        const CHECKSUM_LOW                                  = 0x2e,
        /// Checksum of CMOS addresses 10H through 20H
        const CHECKSUM_HIGH                                 = 0x2f,
        // 0x30 - 0x31 are reserved.
        /// Current century. Not always there, please check the FADT table entry.
        const CENTURY                                       = 0x32,
        /// Miscellaneous flags
        const MISCELLANEOUS                                 = 0x33,
        // 0x34 - 0x3f are reserved.
    };

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
    pub struct RTCStatusA: u8 {
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
        const TIME_FASTEST          = 0b000 << 4,
        /// Second biggest prescaled frequency.
        const TIME_FAST             = 0b001 << 4, 
        /// Divides the clock perfectly to obtain one real time second. DEFAULT VALUE.
        const TIME_REAL             = 0b010 << 4,
        /// Fourth smallest prescaled frequency.
        const TIME_SLOW             = 0b011 << 4,
        /// Third smallest prescaled frequency.
        const TIME_SLOTH            = 0b100 << 4,
        /// Second smallest prescaled frequency.
        const TIME_SNAIL            = 0b110 << 4,
        /// Smallest prescaled frequency.
        const TIME_MATRIX           = 0b111 << 4,
        
        /// Update In Progress (UIP) flag
        ///
        /// When set an update cycle is in progress and the clock/calendar cannot be accessed. When clear, 
        /// at least 244 microseconds are available to access clock/calendar bytes. This bit is
        /// supposed to be read-only.
        const UIP                   = 1 << 7,
    };

    /// RTC Status B register. [RW]
    ///
    /// Seconds RTC configuration register that enables or disables certain features based on the
    /// bitmask used. Allows to enable different interrupt sources, change the output data mode and more.
    ///
    /// # Warn
    /// 
    /// Some RTC CMOS chips do not allow to change this status register, therefore obtained values
    /// format should be converted to a desired representation software way.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct RTCStatusB: u8 {
        /// Enables two special updates: last Sunday in April time will go 01:59:59 -> 03:00:00 and last 
        /// Sunday in October 01:59:59 -> 01:00:00. This bit is unset by default.
        const DAYLIGHT_SAVINGS          = 1 << 0,
        /// Controls hour representation (24/12). When set, a 24 hour format is used. When unset -
        /// 12 hour format. This bit is set by default.
        const HOUR_SELECTION            = 1 << 1,
        /// Defines bits format for representing time. If unset, all time and date registers will
        /// hold values written in BCD format. If set, uses a regular binary format.
        /// 
        /// Binary format is exactly what you would expect the time and date value to be. If the time is 
        /// 1:59:48 AM, then the value of hours would be 1, minutes would be 59 = 0x3b, and seconds would 
        /// be 48 = 0x30. In BCD format the same values would be represented as so: 1:59:48 has hours = 1, 
        /// minutes = 0x59 = 89, seconds = 0x48 = 72.
        const DATA_MODE                 = 1 << 2,
        /// UNUSED. Enables a square wave output on the SQW pin at the frequency specified in rate
        /// selection bits (3 - 0) in status register A. This pin is not connected to anything in
        /// x86 architecture.
        const SQUARE_WAVE_OUTPUT        = 1 << 3,
        /// If this bit is set, interrupt will be asserted once each second after the end of update
        /// cycle. This bit is automatically cleared if CYCLE_UPDATE bit is set.
        const UPDATE_ENDED_INTERRUPT    = 1 << 4,
        /// Interrupt will be asserted once for each second that the current time matches the alarm time.
        const ALARM_INTERRUPT           = 1 << 5,
        /// When set, periodic interrupt will occur at frequency specified in rate selection bits
        /// (3 - 0) in status register A.
        const PERIODIC_INTERRUPT        = 1 << 6,
        /// When set, any current update in progress is aborted. During this you can initialize the
        /// clock, date and alarms manually by writing data to them. When set, the UPDATE_ENDED_INTERRUPT 
        /// bit is cleared. When cleared, the update cycle is continued.
        const CYCLE_UPDATE_ABORT        = 1 << 7,
    };

    /// RTC Status C register [R]
    ///
    /// A read only register that must be read each time the IRQ8 is handled by the OS. It allows
    /// to define which type of interrupt occured in the chip and caused the IRQ8. All four bit
    /// flags are cleared when this register is read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct RTCStatusC: u8 {
        // Bits (3 - 0) are reserved and always 0.
        /// When set the update-ended alarm interrupt has occurred.
        const UPDATE_ENDED_INTERRUPT          = 1 << 4,
        /// When set the alarm interrupt has occurred.
        const ALARM_INTERRUPT                 = 1 << 5,
        /// When set the periodic interrupt has occurred.
        const PERIODIC_INTERRUPT              = 1 << 6,
        /// Set when one of the interrupts enabled in status register B has occured.
        const INTERRUPT_REQUEST               = 1 << 7,
    };
}

impl Default for RTCStatusA {
    fn default() -> Self {
        RTCStatusA::TIME_REAL | RTCStatusA::INTSEC1024
    }
}

impl Default for RTCStatusB {
    fn default() -> Self {
        RTCStatusB::HOUR_SELECTION
    }
}
