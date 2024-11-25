/// This module provides the command words for interacting with the Programmable Interrupt Controller (PIC).
/// Proper operation of the PIC requires sending specific command sequences during initialization. However, 
/// additional commands can be used during runtime for configuration and management.
/// 
/// # Initialization Command Words (ICWs)
/// 
/// ICWs are used to configure the PIC during its initialization phase. The process begins with 
/// ICW1 and ICW2, which are mandatory. ICW1 also determines whether ICW4 will be required. 
/// The initialization sequence is strictly ordered as ICW1 → ICW2 → ICW3 → ICW4. These commands 
/// cannot be repeated individually. To modify the PIC's behavior, a complete reinitialization is necessary.
/// 
/// # Operational Command Words (OCWs)
/// 
/// OCWs are used to manage the PIC after initialization. Unlike ICWs, OCWs are optional, can be issued 
/// in any order, and may be repeated multiple times as needed during the PIC's operation.

use crate::bitflags;

const ICW1_CONST: u8 = 1 << 4;
const OCW3_CONST: u8 = 1 << 3;

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
        ///
        /// # x86
        /// 
        /// Always needed in x86
        const IC4 =                      1                  | ICW1_CONST,
        /// If enabled the single mode will be on. If not, the cascade mode will be on.
        ///
        /// # x86
        /// 
        /// Most PCs have two chained pics, so it shall be used only on archaic systems.
        const SNGL =                     1 << 1             | ICW1_CONST,
        /// Call address interval. If enabled, the interval of 4, if disabled, the interval of 8.
        ///
        /// # x86
        ///
        /// Unused in 8086 mode.
        const ADI =                      1 << 2             | ICW1_CONST,
        /// If enabled, the level triggered mode will be used. If disabled, the edge triggered mode.
        ///
        /// # Note
        /// 
        /// When used, interrupt request must be removed before the EOI comand is issued or the CPU
        /// interrupts is enabled to prevent second interrupt from occuring.
        const LTIM =                     1 << 3             | ICW1_CONST,
        /// Interrupt vector addresses. A7, A6, A5 (8085 mode only).
        ///
        /// # Note
        ///
        /// This value is a bit shift offset.
        const INTERRUPT_VECTOR_ADDRESS = 5,
    };

    /// ICW2 commands are compulsory for initialization.
    /// 
    /// It stores the information regarding the interrupt vector address. a write command
    /// following the ICW1 with A0 = 1 is interpreted as ICW2. It is used to write the high
    /// order byte of the interrupt vector address of all the interrupts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW2: u8 {
        /// The interrupt vector address in 8085 mode. 
        ///
        /// # Note
        ///
        /// This value is a bit mask.
        const INTERRUPT_VECTOR_ADDRESS_8085 = 0b111,
        /// The interrupt vector address in 8086/8088 mode.
        ///
        /// # Note
        ///
        /// This value is a bit mask
        const INTERRUPT_VECTOR_ADDRESS_8086_8088 = 0b11111000,
    };

    /// ICW3 for Master PIC configuration.
    /// 
    /// Defines which interrupt request lines (IR0 to IR7) are connected to a slave.
    #[allow(non_camel_case_types)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW3_MASTER: u8 {
        /// Slave on IR0
        const SLAVE0 = 1,
        /// Slave on IR1
        const SLAVE1 = 1 << 1,
        /// Slave on IR2. (Used in x86)
        const SLAVE2 = 1 << 2,
        /// Slave on IR3
        const SLAVE3 = 1 << 3,
        /// Slave on IR4
        const SLAVE4 = 1 << 4,
        /// Slave on IR5
        const SLAVE5 = 1 << 5,
        /// Slave on IR6
        const SLAVE6 = 1 << 6,
        /// Slave on IR7
        const SLAVE7 = 1 << 7, 
    };

    /// ICW3 for Slave PIC configuration.
    /// 
    /// Defines the slave PIC ID for the interrupt lines from the master.
    #[allow(non_camel_case_types)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW3_SLAVE: u8 {
        /// Connected to master's IR0
        const MASTER0 = 0,
        /// Connected to master's IR1
        const MASTER1 = 1,
        /// Connected to master's IR2
        const MASTER2 = 2,
        /// Connected to master's IR3
        const MASTER3 = 3,
        /// Connected to master's IR4
        const MASTER4 = 4,
        /// Connected to master's IR5
        const MASTER5 = 5,
        /// Connected to master's IR6
        const MASTER6 = 6,
        /// Connected to master's IR7
        const MASTER7 = 7,
    };


    /// ICW4 is loaded only if it is set in the ICW1. It specifies:
    /// - whether to use special fully nested mode or non special fully nested mode;
    /// - whether to use buffered mode or non buffered mode;
    /// - whether to use automatic EOI or Normal EOI.
    /// - CPU used is 8086/8088 or 8085.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ICW4: u8 {
        /// A 8066/8088 mode is used when set. Otherwise, uses 8050 mode.
        const MPM = 1,
        /// If set, using auto EOI. If not, use normal EOI.
        const AEOI = 1 << 1,
        /// Two bits which indicate about the buffered mode:
        /// - non buffered mode (0x0 | 0x1);
        const NON_BUFFERED_MODE = 0 << 2,
        /// - buffered mode slave (0x2);
        const BUFFERED_MODE_SLAVE = 2 << 2,
        /// - buffered mode master (0x3);
        const BUFFERED_MODE_MASTER = 3 << 2,
        /// If set, the special fully nested mode is used, else, the not special mode is used.
        const SFNM = 1 << 4,
    };

    /// OCW1 is used to set and reset the mask bits in IMR.
    ///
    /// All IRQs, which are masked off won't be issued.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OCW1: u8 {
        const MASK_IRQ_0 = 1,
        const MASK_IRQ_1 = 1 << 1,
        const MASK_IRQ_2 = 1 << 2,
        const MASK_IRQ_3 = 1 << 3,
        const MASK_IRQ_4 = 1 << 4,
        const MASK_IRQ_5 = 1 << 5,
        const MASK_IRQ_6 = 1 << 6,
        const MASK_IRQ_7 = 1 << 7,
    };

    /// OCW2 is used for selecting the mode of operation of 8259.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OCW2: u8 {
        /// The first three bits are interrupt level on which action need to be performed.
        ///
        /// # Note
        ///
        /// This value is a bit shift offset.
        const LEVEL = 0,

        /// Sets that the command is not EOI specific (i.e highest priority cleared first).
        const NON_SPECIFIC_EOI_COMMAND              = 0b001 << 5,
        /// Specific EOI command should be used.
        const SPECIFIC_EOI_COMMAND                  = 0b011 << 5,

        /// Automatic rotation on non specific EOI command.        
        const ROTATE_ON_NON_SPECIFIC_EOI_COMMAND    = 0b101 << 5,
        /// Automatic rotation set.
        const ROTATE_IN_AUTOMATIC_EOI_MODE_SET      = 0b100 << 5,
        /// Automatic rotation clear.        
        const ROTATE_IN_AUTOMATIC_EOI_MODE_CLEAR    = 0x000 << 5,
        /// Rotate on the specific EOI command. This combination requires level to be set.
        const ROTATE_ON_SPECIFIC_EOI_COMMAND        = 0b111 << 5,
        
        /// Sets the priority command. Level will be used.
        const SET_PRIORITY_COMMAND                  = 0b110 << 5,
        /// Does nothing.
        const NO_OPERATION                          = 0b010 << 5,
    };

    /// OCW3 is used to read the status of internal registers, managing the special mask and
    /// polling commands.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OCW3: u8 {
        /// Read the Interrupt Request Register (IRR).
        ///
        /// Holds an IR1 bit vector with all interrupt events which are
        /// awaiting to be services. Highest level interrupt is reset when
        /// the CPU acknowledges it.
        const READ_REG_IRR = 0b10                           | OCW3_CONST,
        /// Read the Interrupt Service Register (ISR).
        ///
        /// Tracks IRQ line currently being services. Updated by EOI command.
        const READ_REG_ISR = 0b11                           | OCW3_CONST,
        
        /// When set, poll the command. Do not poll otherwise.
        const POLL = 1 << 2                                 | OCW3_CONST,

        /// Resets the special mask.
        const RESET_SPECIAL_MASK = 0b10 << 5                | OCW3_CONST,
        /// Sets the special mask.
        const SET_SPECIAL_MASK = 0b10 << 5                  | OCW3_CONST, 
    };
}
