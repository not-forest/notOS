/// This module is required for PIC to work properly. It contains the command words that
/// will program the 8259 controller in the needed way.
/// 
/// The module consists of both initialization command words (ICW) and operation command 
/// words (OCW).
/// 
/// # ICW
/// 
/// The ICW commands are given during the initialization of the 8259. The ICW1 and ICW2
/// commands are compulsory for initialization. The ICW1 also decides ift he ICW4 will be
/// needed. The sequence of the commands is fixed i.e. ICW1 -> ICW2 -> ICW3 -> ICW4. Any 
/// of the ICW commands cannot be repeated, but the entire initialization can be repeated. 
/// This means that you will have to reprogram the controller fully if you wish to change 
/// it's behavior even a little.
/// 
/// # OCW
/// 
/// OCW is given when the CPU starts to use the controller. They are not compulsory for 8259
/// and can be repeated many times in any sequence.

use crate::bitflags;

bitflags! {
    /// ICW1 commands are compulsory for initialization
    /// 
    /// It specifies:
    /// - single or multiple 8259As in system;
    /// - 4 or 8 bit interval between the interrupt vector locations;
    /// - the address bits of A7-A5 of the call instruction;
    /// - edge triggered or level triggered interrupts;
    /// - ICW4 is needed or not
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW1: u8 {
        /// If enabled the ICW4 is needed.
        const IC4 =                      1,
        /// If enabled the single mode will be on. If not, the cascade mode will be on.
        const SNGL =                     1 << 1,
        /// Call address interval. If enabled, the interval of 4, if disabled, the interval of 8.
        const ADI =                      1 << 2,
        /// If enabled, the level triggered mode will be used. If disabled, the edge triggered mode.
        const LTIM =                     1 << 3,
        /// Enables the chip. Required always.
        const ENABLE =                   1 << 4,
        /// Interrupt vector addresses. A7, A6, A5.
        const INTERRUPT_VECTOR_ADDRESS = 0xe0,
    };

    /// ICW2 commands are compulsory for initialization.
    /// 
    /// It stores the information regarding the interrupt vector address. a write command
    /// following the ICW1 with A0 = 1 is interpreted as ICW2. It is used to write the high
    /// order byte of the interrupt vector address of all the interrupts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW2: u8 {
        /// The interrupt vector address in 8085 mode.        
        const INTERRUPT_VECTOR_ADDRESS_8085 = 0xff,
        /// The interrupt vector address in 8086/8088 mode.
        const INTERRUPT_VECTOR_ADDRESS_8086_8088 = 0xf8,
    };

    /// ICW3 is required only if there is more than one 8259 chip in the system and if they
    /// are cascaded. To ignore this one, the single mode must be set in ICW1. An ICW3 loads
    /// a slave register in the PIC. For master, each bit in ICW3 is used to specify whether
    /// it has a slave chip attached to it on it's corresponding interrupt request input.
    /// 
    /// If the chip is master, all of its bits can be used. For slave device, only the first
    /// three bits are used.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW3: u8 {
        const SLAVE0 = 1,
        const SLAVE1 = 1 << 1,
        const SLAVE2 = 1 << 2,
        const SLAVE3 = 1 << 3,
        const SLAVE4 = 1 << 4,
        const SLAVE5 = 1 << 5,
        const SLAVE6 = 1 << 6,
        const SLAVE7 = 1 << 7,        
        /// The first three bits representing the slave id. Use it if the device is a slave.
        const SLAVE_ID = 0x7,
    };

    /// ICW4 is loaded only if it is set in the ICW1. It specifies:
    /// - whether to use special fully nested mode or non special fully nested mode;
    /// - whether to use buffered mode or non buffered mode;
    /// - whether to use automatic EOI or Normal EOI.
    /// - CPU used is 8086/8088 or 8085.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW4: u8 {
        /// Sets a 8066/8088 mode if set. If not set, uses standard 8050 mode.
        const NPM = 1,
        /// If set, using auto EOI. If not, use normal EOI.
        const AEOI = 1 << 1,
        /// Two bits which indicate about the buffered mode:
        /// - non buffered mode (0x0, 0x1);
        /// - buffered mode slave (0x2);
        /// - buffered mode master (0x3);
        const BUFFERED_MODE = 3 << 2,
        /// If set, the special fully nested mode is used, else, the not special mode is used.
        const SFNM = 1 << 4,
    };

    /// OCW1 is used to set and reset the mask bits in IMR.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OCW1: u8 {
        const FULL_MASK = 0xFF,
    };

    /// OCW2 is used for selecting the mode of operation of 8259.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OCW2: u8 {
        /// The first three bits are interrupt level, that specify the level to be
        /// acted upon when the SL bit is active.
        const LEVEL = 0x7,
        // The last three bits are R, SL and EOI. They control the rotate and end of interrupt
        // modes. The list of all possible combinations are defined as so:
        /// Sets that the command is not EOI specific. It is an end of interrupt.
        const NON_SPECIFIC_EOI_COMMAND = 0x20,
        /// Specific EOI command should be used.
        const SPECIFIC_EOI_COMMAND = 0x60,
        /// Automatic rotation on non specific EOI command.        
        const ROTATE_ON_NON_SPECIFIC_EOI_COMMAND = 0xa0,
        /// Automatic rotation set.
        const ROTATE_IN_AUTOMATIC_EOI_MODE_SET = 0x80,
        /// Automatic rotation clear.        
        const ROTATE_IN_AUTOMATIC_EOI_MODE_CLEAR = 0x0,
        /// Rotate on the specific EOI command. This combination requires the level to be set.
        const ROTATE_ON_SPECIFIC_EOI_COMMAND = 0xe0,
        /// Sets the priority command. Level will be used.
        const SET_PRIORITY_COMMAND = 0xc0,
        /// Does nothing.
        const NO_OPERATION = 0x40,
    };

    /// OCW3 is used to read the status of the registers and to set or reset the special
    /// mask and polled modes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OCW3: u8 {
        /// Read register command
        /// 
        /// Reads the IR register on the next pulse.
        const READ_IR_REG = 0x2,
        /// Read register command
        /// 
        /// Reads the IS register on the next pulse.
        const READ_IS_REG = 0x3,

        /// This bit must always be used with other values when using OCW3.
        const ENABLE = 1 << 3,

        /// If set, polls a command. Does not otherwise.
        /// 
        /// When the 8259 is not in the Polled mode, after it is set up for. an IRR status 
        /// read operation, all Read commands with A0=1 cause the 8259 to send the IRR status word.
        const POLL_COMMAND = 1 << 2,

        /// Special mask command
        /// 
        /// Resets the special mask.
        const RESET_SPECIAL_MASK = 0x40,
        /// Special mask command
        /// 
        /// Sets the special mask.
        const SET_SPECIAL_MASK = 0x60,
    };
}