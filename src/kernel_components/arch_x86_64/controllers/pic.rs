/// Support for the 8259 Programmable Interrupt Controller, which handles basic I/O interrupts.  
/// In multicore mode, we would apparently need to replace this with an APIC interface.
///
/// A single PIC handles up to eight vectored priority interrupts for the CPU. By cascading 8259
/// chips, we can increase interrupts up to 64 interrupt lines, however we only have two chained
/// instances that can handle 16 lines. Can be programmed either in edge triggered, or in level
/// triggered mode. PIC uses CHANNEL0 from the PIT (Programmable Interval Timer), the frequency of
/// which can be adjusted based on it's configuration. Individual bits of IRQ register within the 
/// PIC can be masked out by the software.

use crate::bitflags;
use crate::kernel_components::arch_x86_64::ports::{GenericPort, PortAccessType};
use crate::kernel_components::sync::Mutex;
use crate::kernel_components::arch_x86_64::post::DEBUG_BOARD;
use super::pic_command_words::*;

/// Defines the PIC IRQ mappings (hardwired lines) for the PIC controller.
///
/// The PIC can be configured either as a master or a slave device. This will change the upcoming
/// ICW3 command during the initialization.
///
/// The master shall define it's IRQ line on which the slave is connected. The slave shall define 
/// it's IRQ line on which it is connected to the master. Only one master can be used in the whole
/// connection. This allows up to 64 IRQ lines, when 8 slaves are 
///
/// # x86
///
/// For regular PC systems two PICs are chained together in a MASTER <-> SLAVE pair, allowing to
/// use up to 16 different IRQs.
#[derive(Debug, Clone, Copy)]
pub enum PicIRQMapping {
    /// Master PIC chip mapping.
    ///
    /// Defines all interrupt request bits on which the slave PIC is connected. 
    Master(Option<ICW3_MASTER>),
    /// Slave PIC chip mapping.
    ///
    /// Defines one IRQ interrupt request line, which it is connected to.
    Slave(ICW3_SLAVE),
}

/// **PIC Chip** - Programmable Interrupt Controller.
/// 
/// In x86 this can be either master or slave chip. Each chip has it's command port and data port.
/// The offset is used to handle different interrupt events.
#[derive(Debug)]
pub struct Pic {
    /// The base offset to which our interrupts are mapped.
    pub offset: u8,
    /// Current operation mode for this specific PIC.
    op_mode: PicOperationMode,
    /// Automatic EOI flags.
    automatic_interrupts: bool,
    /// The processor I/O port on which we send commands.
    command: GenericPort<u8>,
    /// The processor I/O port on which we send and receive data.
    data: GenericPort<u8>,
}

impl Pic {
    /// Creates a new instance of a PIC chip.
    ///
    /// According to the datasheet, PIC should be fully reinitialized with all four initialization
    /// words again to exit this mode.
    ///
    /// # Offset
    ///
    /// Offset must be provided to not collide with x86 exceptions. Changing an offset means
    /// reinitializing the PIC again completely from the very start. When several PICs are used,
    /// their offsets shall not collide with each others. 
    ///
    /// # Warn
    ///
    /// This does not include the initialization of the PIC chip. Use the [´Pic::init´] function to
    /// perform a proper initialization.
    /// 
    /// # Note
    /// 
    /// This only creates a single PIC chip. Classic x86/x64 PC includes 2 chained PICs within. This function
    /// is a public API only due to a possibility of only one PIC chip on some really archaic PC/XT systems.
    pub const unsafe fn new(offset: u8, command_port: u16, data_port: u16) -> Self {
        Self {
            offset,
            op_mode: PicOperationMode::FullyNested, // Used after initialization.
            automatic_interrupts: false,
            command: GenericPort::new(command_port, PortAccessType::READWRITE),
            data: GenericPort::new(data_port, PortAccessType::READWRITE),
        }
    }

    /// Initialization of the PIC controller.
    ///
    /// All initialization words shall be provided to the controller in a very strict order:
    /// - ICW1 → command port;
    /// - ICW2 → data port;
    /// - ICW3 → command port, IF (ICW1 bit D1 == 0) ELSE ignored;
    /// - ICW4 → data port;
    ///
    /// For swapping to a different configuration this whole process must be repeated from the very
    /// start. After this function OCW commands can be sent to the PIC controller. 
    ///
    /// For using PICs configuration on regular x86 PCs, use [´ChainedPics´] structure, which provides 
    /// even safer function.
    ///
    /// # PIC Mapping
    ///
    /// This decides if the PIC is a master or a slave device. It also defines which lines are
    /// connected to which depending on it's value.
    ///
    /// # Automatic Interrupts
    ///
    /// With this flag enabled PIC will automatically perform a EOI operation at the trailing edge of the
    /// last interrupt acknowledge pulse from the CPU. This setting can only be used with a single
    /// master chip. Basically that means that the end of interrupt command is not necessary after
    /// a corresponding handler function handles the interrupt, however it does not work well with
    /// chained PICs.
    ///
    /// Note that from a system standpoint, this mode should be used only when a nested multilevel interrupt 
    /// structure is not required within a single 8259A.
    pub unsafe fn init(&mut self, pic_map: PicIRQMapping, automatic_interrupts: bool) {
        unsafe {
            // Saving the previous irq masking.
            let mask = self.mask_read();
            // Generating initialization commands based on the chosen operation mode.
            let icw1 = 
                ICW1::IC4 | 
                match pic_map {
                    // Using single mode when only one chip is presented.
                    PicIRQMapping::Master(opt) => if opt.is_none() { 
                        ICW1::SNGL 
                    } else { ICW1::empty() },
                    _ => ICW1::empty(),
                };
            // Only implementing the x86 compatible version here.
            let icw2 = self.offset & ICW2::INTERRUPT_VECTOR_ADDRESS_8086_8088.bits();
            // In most PC systems two PICs are used. One PIC or more than two is also allowed.
            let icw3 = pic_map;
            // In x86 systems only this bit is used.
            let icw4 = 
                ICW4::MPM | 
                if automatic_interrupts { ICW4::AEOI } else { ICW4::empty() };

            // A short delay is required between each write due to the slowness of the controller.
            /* ICW1 command. */
            self.command.write(icw1.bits());
            DEBUG_BOARD.write(0);
            /* ICW2 command. */
            self.data.write(icw2);
            DEBUG_BOARD.write(0);
            /* ICW3 command. (If some) */
            match icw3 {
                // Master might be alone or in a chained configuration.
                PicIRQMapping::Master(opt) => opt.map(|some| { 
                    self.command.write(some.bits());
                    DEBUG_BOARD.write(0);
                }).unwrap_or(()),
                // If slave, at least one more chip should exist.
                PicIRQMapping::Slave(some) => {
                    self.command.write(some.bits());
                    DEBUG_BOARD.write(0);
                },
            }
            /* ICW4 command. */
            self.data.write(icw4.bits());
            DEBUG_BOARD.write(0);

            // Restoring the mask.
            self.mask_write(mask);
        }
    }

    /// Checks if the provided IRQ id from the IDT matches this PIC.
    ///
    /// Each PIC may only handle up to 8 interrupts.
    pub fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        (self.offset..self.offset + 8).contains(&interrupt_id)
    }

    /// Reads the value of current operation mode used on this PIC.
    pub fn operation_mode_current(&self) -> PicOperationMode {
        self.op_mode
    }

    /// Changes the current operation mode of this PIC.
    ///
    /// This sends the OCW2 command and configures the current operation mode of the PIC logic.
    /// Refer to [´PicOperationMode´] enum for more details.
    ///
    /// # Warn.
    ///
    /// When switching from polled mode a mask must be restored to the previously used one.
    pub fn operation_mode_change(&mut self, new_op_mode: PicOperationMode) {
        use PicOperationMode::*;

        unsafe {
            /* Default behaviour when switching the mode. */
            let fully_nested_arm = |s: &mut Self| // Restoring the disturbed fully nested structure. 
                s.set_lowest_priority(7);
            let automatic_rotation_arm = |s: &mut Self| 
                if s.automatic_interrupts {
                    s.command.write(OCW2::ROTATE_IN_AUTOMATIC_EOI_MODE_SET.bits());
                };
            let special_mask_arm = |s: &mut Self| s.command.write(OCW3::SET_SPECIAL_MASK.bits());
            let polled_mode_arm = |s: &mut Self| {
                s.mask_write(OCW1::all());
                s.command.write(OCW3::POLL.bits());
            };

            match self.op_mode {
                FullyNested => match new_op_mode {
                    FullyNested => return,
                    AutomaticRotation => automatic_rotation_arm(self),
                    SpecialMask => special_mask_arm(self),
                    PolledMode => polled_mode_arm(self),
                },
                AutomaticRotation => {
                    if self.automatic_interrupts {
                        self.command.write(OCW2::ROTATE_IN_AUTOMATIC_EOI_MODE_CLEAR.bits());
                    }
                    match new_op_mode {
                        FullyNested => fully_nested_arm(self),
                        AutomaticRotation => return,
                        SpecialMask => special_mask_arm(self),
                        PolledMode => polled_mode_arm(self),
                    }
                },
                SpecialMask => {
                    self.command.write(OCW3::RESET_SPECIAL_MASK.bits());
                    match new_op_mode {
                        FullyNested => fully_nested_arm(self),
                        AutomaticRotation => automatic_rotation_arm(self),
                        SpecialMask => return,
                        PolledMode => polled_mode_arm(self)
                    }
                },
                PolledMode => match new_op_mode {
                    FullyNested => fully_nested_arm(self),
                    AutomaticRotation => automatic_rotation_arm(self),
                    SpecialMask => special_mask_arm(self),
                    PolledMode => return,
                },
            }
        }
        DEBUG_BOARD.write(0); 
        self.op_mode = new_op_mode;
    }

    /// Checks if the provided interrupt vector was caused by PIC properly.
    ///
    /// When an IRQ occurs, the PIC chip tells the CPU (via. the PIC's INTR line) that there's an interrupt, 
    /// and the CPU acknowledges this and waits for the PIC to send the interrupt vector. This creates a race 
    /// condition: if the IRQ disappears after the PIC has told the CPU there's an interrupt but before the 
    /// PIC has sent the interrupt vector to the CPU, then the CPU will be waiting for the PIC to tell it 
    /// which interrupt vector but the PIC won't have a valid interrupt vector to tell the CPU.
    ///
    /// Basically if the ISR bit for this flag is not set, but the interrupt service routine was
    /// executed, that means it is spurious and interrupt must end right away. 
    ///
    /// # Unsafe 
    ///
    /// This function is only unsafe as it shall be only used within the interrupt handler function
    /// at the very start, to make sure that we are not handling a spurious interrupt. It is
    /// completely forbidden to send an EOI, if this function evaluates to true! 
    pub unsafe fn is_spurious(&mut self, id: u8) -> bool {
        assert!(id >= 32 && self.offset + 8 > id, "The provided interrupt vector is outside of scope of this PIC chip."); 

        let irq = ISR::from(1 << id.saturating_sub(self.offset));
        !irq.is_in(self.read_isr().into())
    }

    /// Reads the value of the ISR.
    /// 
    /// The interrupt status register inside the PIC chip, shows the info about which interrupts are
    /// being serviced at that moment. The value will be flushed after the end_of_interrupt method.
    ///
    /// # Note
    ///
    /// Always 0x0 in polled mode.
    pub fn read_isr(&mut self) -> ISR {
        unsafe {
            match self.op_mode {
                // ISR is guaranteed to be empty during in polled mode.
                PicOperationMode::PolledMode => return ISR::empty(),
                PicOperationMode::SpecialMask => {
                    self.command.write(
                        OCW3::READ_REG_ISR.bits() | OCW3::SET_SPECIAL_MASK.bits()
                    );
                },
                _ => {
                    self.command.write(
                        OCW3::READ_REG_ISR.bits()
                    );
                }
            }
            ISR::from(self.command.read())
        }
    }

    /// Reads the value of the IRR.
    /// 
    /// The interrupt request register shows the requested interrupts that have been raised
    /// but are not being acknowledged yet. The value will be flushed after the end_of_interrupt method.
    pub fn read_irr(&mut self) -> IRR {
        unsafe {
            match self.op_mode {
                PicOperationMode::SpecialMask => {
                    self.command.write(
                        OCW3::READ_REG_IRR.bits() | OCW3::SET_SPECIAL_MASK.bits()
                    );
                },
                _ => {
                    self.command.write(
                        OCW3::READ_REG_IRR.bits()
                    );
                }
            }
            if self.op_mode == PicOperationMode::PolledMode {
                self.command.write(OCW3::POLL.bits())
            }
            IRR::from(self.command.read())
        }
    }

    /// Read the current PIC mask.
    ///
    /// The mask defines all IRQ lines, that shall be ignored and not sent to the CPU. 
    pub fn mask_read(&mut self) -> OCW1 {
        unsafe {
            OCW1::from(self.data.read())
        }
    }

    /// Poll the interrupt with highest priority.
    ///
    /// The value returned is a binary code of the highest priority level requesting service. Will
    /// return None if the current mode is not [´PicOperationMode::PolledMode´].
    ///
    /// # Note
    ///
    /// The interrupt is immediately acknowledged after the first read, according to the datasheet:
    /// When poll command is issued, the 8259 treats the next RD pulse as an interrupt acknowledge.
    pub fn poll(&mut self) -> Option<u8> {
        match self.op_mode {
            PicOperationMode::PolledMode => unsafe { Some(self.command.read()) },
            _ => None
        }
    }

    /// Masks the requested IRQ lines.
    ///
    /// Sends the OCW1 command and masks unused IRQ lines.
    ///
    /// # Huge Warn
    ///
    /// On special mask mode, this inhibits the priority level, not masks the interrupts
    /// completely. See more info in [´PicOperationMode::SpecialMask´]
    ///
    /// # Unsafe
    ///
    /// Even though masking just disabled some interrupt lines, this function is marked as unsafe
    /// due to undefined behavior that might happen when the OCW1 command is not right.
    pub unsafe fn mask_write(&mut self, ocw1: OCW1) {
        self.data.write(ocw1.bits());
    }

    /// Sends a proper end of interrupt.
    ///
    /// # Special Mask
    ///
    /// Before calling this function in a special mask mode [´PicOperationMode::SpecialMask´], a
    /// mask can be applied to the data port of the PIC to inhibit some interrupts. Priority can
    /// also be changed.
    ///
    /// # Note
    ///
    /// Does nothing if PIC is configured with automatic EOI flag or in poll mode.
    pub fn end_of_interrupt(&mut self) {
        if !self.automatic_interrupts {
            match self.op_mode {
                PicOperationMode::AutomaticRotation => unsafe { 
                    self.command.write(OCW2::ROTATE_ON_NON_SPECIFIC_EOI_COMMAND.bits()); 
                },
                PicOperationMode::PolledMode => (), // Interrupt is acknowledged once the command port is read.
                _ => unsafe { self.non_specified_eoi() },
            }
        }
    }

    /// Sends a proper specific end of interrupt.
    ///
    /// # Special Mask
    ///
    /// Before calling this function in a special mask mode [´PicOperationMode::SpecialMask´], a
    /// mask can be applied to the data port of the PIC to inhibit some interrupts. Priority can
    /// also be changed.
    ///
    /// # Unsafe
    ///
    /// A proper IRQ must be used, or new interrupts won't appear.
    ///
    /// # Note
    ///
    /// Does nothing if PIC is configured with automatic EOI flag or in poll mode.
    pub unsafe fn end_of_interrupt_specific(&mut self, irq: u8) {
        assert!(irq < 8, "Level is written in binary format (0 .. 7).");

        if !self.automatic_interrupts {
            match self.op_mode {
                PicOperationMode::AutomaticRotation => unsafe { 
                    self.command.write(OCW2::ROTATE_ON_SPECIFIC_EOI_COMMAND.bits() | irq << OCW2::LEVEL.bits()); 
                },
                PicOperationMode::PolledMode => (), // Interrupt is acknowledged once the command port is read.
                _ => unsafe { self.specified_eoi(irq) },
            }
        }
    }

    /// Manually change the lowest priority of this PIC.
    ///
    /// The lowest priority can be fixed on some IRQ and thus fixing other priorities, where lower
    /// IRQs grow priority, i.e if the IRQ5 is lowest priority, then IRQ6 is the highest priority 
    /// and IRQ4 is the second lowest priority (it is circular). By default in fully nested mode, 
    /// the IRQ0 is the highest and IRQ7 is the lowest.
    ///
    /// The value is expected in binary format.
    ///
    /// # Note
    ///
    /// Note that PIC will generate a spurious interrupt on IRQ7 regardless of the priority level.
    pub unsafe fn set_lowest_priority(&mut self, level: u8) {
        assert!(level < 8, "Level is written in binary format (0 .. 7).");

        self.command.write(
            OCW2::SET_PRIORITY_COMMAND.bits() | (level << OCW2::LEVEL.bits())
        );
    }

    /// Performs an unsafe specified end of interrupt.
    ///
    /// The value is expected in binary format.
    ///
    /// # Unsafe 
    ///
    /// Specified end of interrupt must be written together with an interrupt level to reset.
    /// Reseting a wrong level will cause the interrupt handler to enter a loop. 
    pub unsafe fn specified_eoi(&mut self, level: u8) {
        self.command.write(
            OCW2::SPECIFIC_EOI_COMMAND.bits() | (level << OCW2::LEVEL.bits())
        );
    }

    /// Performs an unsafe non specified end of interrupt.
    ///
    /// The value is expected in binary format.
    ///
    /// # Unsafe
    ///
    /// Non specific EOI resets the highest ISR bit of those that are set. It is safe to use in
    /// fully nested mode, which is the default and mostly used mode on PC, however will cause
    /// wrong flags being cleared on different operation modes.
    pub unsafe fn non_specified_eoi(&mut self) { 
        self.command.write(
            OCW2::NON_SPECIFIC_EOI_COMMAND.bits()
        );
    }
}

bitflags! {
    /// Read the Interrupt Request Register (IRR).
    ///
    /// Holds an IR1 bit vector with all interrupt events which are
    /// awaiting to be services. Highest level interrupt is reset when
    /// the CPU acknowledges it.
    ///
    /// The interrupt request register shows the requested interrupts that have been raised
    /// but are not being acknowledged yet. The highest priority value will be flushed after 
    /// CPU enters the interrupt handler.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct IRR: u8 {
        const IRQ0 = 1,
        const IRQ1 = 1 << 1,
        const IRQ2 = 1 << 2,
        const IRQ3 = 1 << 3,
        const IRQ4 = 1 << 4,
        const IRQ5 = 1 << 5,
        const IRQ6 = 1 << 6,
        const IRQ7 = 1 << 7,
    };
        
    /// Read the Interrupt Service Register (ISR).    
    ///
    /// Tracks IRQ line currently being services. Updated by EOI command. The interrupt status register 
    /// inside the PIC chip, shows the info about which interrupts are being serviced at that moment. 
    /// The highest priority value will be flushed after the end_of_interrupt method.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ISR: u8 {
        const IRQ0 = 1,
        const IRQ1 = 1 << 1,
        const IRQ2 = 1 << 2,
        const IRQ3 = 1 << 3,
        const IRQ4 = 1 << 4,
        const IRQ5 = 1 << 5,
        const IRQ6 = 1 << 6,
        const IRQ7 = 1 << 7,
    };
}

/// **Operation Mode for PIC Controller**.
///
/// PIC supports several operation mode, most of which are most likely to be ignored on x86
/// architecture, however some of them can be used to obtain some interesting results. See more
/// information for each of them below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PicOperationMode {
    /// Fully Nested Mode (Default Mode)
    ///
    /// This mode is entered after initialization unless another mode is programmed. The interrupt
    /// requests are ordered in priority from 0 through 7, where 0 is the highest priority. When
    /// interrupt is acknowledged the highest priority interrupt will be issued before the rest.
    ///
    /// On a regular x86 PC with two chained PICs, the IRQ0, which is an output of a PIT timer,
    /// will be handled before others IRQ lines. Slave PIC has even smaller priority than first 7
    /// masters IRQ lines, because it is mapped after the master PIC.
    ///
    /// # Use Case
    ///
    /// Use when the default priority level suits your needs. For example, if a PS/2 keyboard interrupt
    /// (IRQ1) will be always services before the real time clock (IRQ8).
    FullyNested,
    /// Automatic Rotation Mode (Equal Priority Mode)
    ///
    /// Rotates the priority by the value specified in the current highest priority interrupt
    /// within the ISR register. Basically each time the highest priority interrupt occur, it will 
    /// then be defined as a lowest priority interrupt. This way giving all interrupt sources an
    /// equal service time from the CPU.
    ///
    /// # Use Case
    ///
    /// Use if you think that all interrupts deserve to be handled equally. This sometimes might
    /// cause troubles with timers, specifically the PIT and RTC.
    AutomaticRotation,
    /// Special Mask Mode (Manual Priority Mode)
    ///
    /// Some applications might want to have a different priority mapping for the full software
    /// control over the sequence of interrupts. During this mode the mask register is now used to 
    /// temporarly disable certain interrupt levels as well as manually changing the priority level.
    ///
    /// # Use Case
    ///
    /// Critical sections that wish to disable some interrupts from the PIC but not all of them, or
    /// some applications with specific timing requirements that require to temporarly inhibit some
    /// of interrupt levels to make sure that lower priority interrupts will meet timings accordigly.
    SpecialMask,
    /// Polled Mode (No interrupts)
    ///
    /// Do not use interrupts to obtain information from the peripherals but only listen for
    /// upcoming changes. After the polled mode is enabled, data bus will provide a binary value of a
    /// highest priority issued interrupt. Each read from the data port will be treated as an
    /// interrupt acknowledge.
    ///
    /// # Use Case
    ///
    /// Probably the most useless one. Since it is very quick to turn this mode on and off, it can
    /// be used to handle several interrupts in one handler by reading all values from the data
    /// port until it will be equal to zero.
    PolledMode,
}

/// A x86 setup of **Chained PICs**.
///
/// In most PCs there are one master and one slace PIC configuration, each having 8 inputs
/// servicing 16 interrupts. This structure allows to easily initialize and control the x86
/// configuration of PICs and configure all 16 interrupts for further handling.
///
/// Provides a minimal set of functions required to properly handle interrupts based on the
/// currently used mode for each PIC.
pub struct ChainedPics {
    initialized: bool,
    pub master: Pic,
    pub slave: Pic,
}

impl ChainedPics {
    /// Creates a new instance of Chained Pics.
    /// 
    /// The master offset and slave offset are two offsets that are pointing to the first
    /// interrupt vector of each 8259 chip.
    /// 
    /// # Panics
    /// 
    /// This function will panic if the provided offsets will overlap with each other or
    /// collide with CPU exceptions.
    pub const fn new(master_offset: u8, slave_offset: u8) -> Self {
        assert!(master_offset >= 32 && slave_offset >= 32, "Both master and slave offsets must not overlap with CPU exceptions.");
        assert!(master_offset.abs_diff(slave_offset) >= 8, "The master and slave offsets are overlapping with each other.");

        unsafe { Self::new_unchecked(master_offset, slave_offset) }
    }

    /// Creates a new instance of a Chained Pics.
    /// 
    /// The offset must point to the the chosen 16 entries from the IDT that will be used 
    /// for the software interrupts.
    /// 
    /// This is a convenience function that maps the PIC1 and PIC2 to a
    /// contiguous set of interrupts. This function is equivalent to
    /// `Self::new(primary_offset, primary_offset + 8)`.
    ///
    /// # Panics
    /// 
    /// This function will panic if the provided offset will overlap with cpu exceptions. It
    /// will always prevent the overlapping between master and slave chips though
    pub const fn new_contiguous(primary_offset: u8) -> Self {
        Self::new(primary_offset, primary_offset + 8)
    }

    /// Returns true if initialized at least once.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Initializes the PICs.
    ///
    /// This performs an initialization that is compatible with most x86 PC devices. Some archaic
    /// devices may use only one PIC. For such possibilities a manual initialization of PIC
    /// structure must be performed.
    pub fn initialize(&mut self) {
        unsafe {
            self.master.init(PicIRQMapping::Master(Some(ICW3_MASTER::SLAVE2)), false);
            self.slave.init(PicIRQMapping::Slave(ICW3_SLAVE::MASTER2), false);
        }
        if !self.is_initialized() { self.initialized = true }
    }


    /// Changes the operation mode for both master and slave PICs.
    ///
    /// This sends the OCW2 command and configures the current operation mode of the PIC logic.
    /// Refer to [´PicOperationMode´] enum for more details. This function only checks the mode of
    /// the master PIC, assuming that slave was not changed manually to something else.
    ///
    /// # Note
    ///
    /// The IRQ mask must be changed after switching from [´PicOperationMode::PolledMode´].
    pub fn operation_mode_change(&mut self, new_op_mode: PicOperationMode) {
        if self.master.operation_mode_current() != new_op_mode {
            self.master.operation_mode_change(new_op_mode);
            self.slave.operation_mode_change(new_op_mode);
        }
    }

    /// Checks if the provided interrupt vector was caused by PIC properly.
    ///
    /// When an IRQ occurs, the PIC chip tells the CPU (via. the PIC's INTR line) that there's an interrupt, 
    /// and the CPU acknowledges this and waits for the PIC to send the interrupt vector. This creates a race 
    /// condition: if the IRQ disappears after the PIC has told the CPU there's an interrupt but before the 
    /// PIC has sent the interrupt vector to the CPU, then the CPU will be waiting for the PIC to tell it 
    /// which interrupt vector but the PIC won't have a valid interrupt vector to tell the CPU.
    ///
    /// Here we read if the interrupt is written within the ISR register, to be sure that we are
    /// dealing with real interrupt. If a spurious interrupt was also sent from the slave PIC, the
    /// master shall clear this flag, because he also thinks that it was a legit IRQ.
    ///
    /// # Important
    ///
    /// This is also the reason why sometimes an unimplemented handler functions are causing general protection 
    /// faults. PIC will cause an interrupt on the lowest priority IRQ, and the interrupt service
    /// routine for something like hard disk controller is most likely not implemented in the stage
    /// of configuring PICs.
    ///
    /// # Note (Fully Nested Mode)
    ///
    /// Spurious interrupts can only happen when the lowest priority IRQ are called. The fake interrupt number 
    /// is the lowest priority interrupt number for the corresponding PIC chip (IRQ 7 for the master PIC, and 
    /// IRQ 15 for the slave PIC).
    ///
    /// **Basically this means that you shall only check for spurious IRQs when it is a parallel
    /// port interrupt (IRQ7) or secondary ATA channel interrupt (IRQ15)**
    ///
    /// # Note (Rotations)
    ///
    /// When modes with rotations are used: [´PicOperationMode::AutomaticRotation´],
    /// [´PicOperationMode::SpecialMask´], the spurious interrupt can still only be found on IRQ7
    /// for the master PIC or IRQ15 for the slave PIC.
    ///
    /// # Note (Polled Mode)
    ///
    /// Makes no sense in polled mode. Note also that in polled mode the ISR is always zero, so
    /// this function will always mark proper IRQs as spurious.
    ///
    /// # Unsafe 
    ///
    /// This function is unsafe only because it shall be used within the interrupt handler function
    /// at the very start, to make sure that we are not handling a spurious interrupt. It is
    /// completely forbidden to send an end of interrupt after this function. 
    pub unsafe fn is_spurious(&mut self, vec_id: u8) -> bool {
        assert!(vec_id >= 32, "Cannot be one of the CPU exceptions."); 

        if self.slave.handles_interrupt(vec_id) {
            if self.slave.is_spurious(vec_id) { 
                self.master.end_of_interrupt(); 
                true 
            } else { false } 
        } else if self.master.handles_interrupt(vec_id) { 
            self.master.is_spurious(vec_id) 
        } else {
            panic!("Provided interrupt is out of scope for both PICs.")
        }
    }

    /// Notify a proper PIC chip that the interrupt was succesfully handled and shall be cleared
    /// within the ISR register.
    ///
    /// # Important
    ///
    /// To prevent spurious interrupts on lowest priority IRQs, use [´ChainedPics::is_spurious´]
    /// and jump to the end of interrupt handler function if it returns true. If some interrupt was
    /// caused by a hardware|software error, it should not be handled.
    ///
    /// **PIC must not receive a EOI command, when it is a spurious interrupts. It can clear
    /// other interrupt's flag, which is a bigger trouble.**
    ///
    /// Lower priority interrupts vary based on the current mode. The function mentioned above
    /// handles all logic required for each.
    ///
    /// # Unsafe 
    ///
    /// This command must be used at the end of every interrupt that was issued by any of two PICs.
    /// Make sure that this is a last command withon the interrupt service routine.
    pub unsafe fn notify_end_of_interrupt(&mut self, vec_id: u8) {
        assert!(vec_id >= 32, "Cannot be one of the CPU exceptions."); 

        if self.slave.handles_interrupt(vec_id) {
            self.slave.end_of_interrupt();
            self.master.end_of_interrupt();
        } else
        if self.master.handles_interrupt(vec_id) {
            self.master.end_of_interrupt();
        }
    }

    /// Disable both PICs interrupts.
    ///
    /// # Note
    ///
    /// This must be used when switching to APIC controller for handling interrupts.
    pub fn disable(&mut self) {
        unsafe { self.write_mask(IrqMask::all()) };
    }

    /// Enables both PICs interrupts.
    ///
    /// They are enabled by default after the initialization.
    ///
    /// # Warn
    ///
    /// This is not the initialization. Please see [´ChainedPics::initialize´]
    pub fn enable(&mut self) {
        unsafe { self.write_mask(IrqMask::empty()); }
    }

    /// Disables the slave PIC fully, i.e IRQ8 ... IRQ15.
    pub fn disable_slave(&mut self) {
        unsafe {
            let mask = self.master.mask_read();
            self.master.mask_write(
                mask | OCW1::MASK_IRQ_2
            );
        }
    }

    /// Enables the slave PIC, i.e IRQ8 ... IRQ15.
    pub fn enable_slave(&mut self) {
        unsafe {
            let mask = self.master.mask_read();
            self.master.mask_write(
                mask & !OCW1::MASK_IRQ_2
            );
        }
    }

    /// Gets the current IRQ mask.
    pub fn get_mask(&mut self) -> IrqMask {
        IrqMask::from(
            u16::from_le_bytes([
                self.master.mask_read().bits(), self.slave.mask_read().bits()
            ])
        )
    }

    /// Masks the IRQ lines of chained PICs.
    ///
    /// # Unsafe
    ///
    /// Even though masking just disabled some interrupt lines, this function is marked as unsafe
    /// due to undefined behavior that might happen when the OCW1 command is not right.
    pub unsafe fn write_mask(&mut self, mask: IrqMask) {
        let bytes = mask.bits().to_le_bytes();
        unsafe {
            self.master.mask_write(OCW1::from(bytes[0]));
            self.slave.mask_write(OCW1::from(bytes[1]));
        }
    }

    /// Perform something on the master PIC.
    pub fn with_master<F>(&mut self, f: F) where
        F: FnOnce(&mut Pic)
    {
        f(&mut self.master)
    }

    /// Perform something on the slave PIC.
    pub fn with_slave<F>(&mut self, f: F) where
        F: FnOnce(&mut Pic) {
        f(&mut self.slave)
    }

    /// Creates a new instance of PIC controller.
    /// 
    /// The master offset and slave offset are two offsets that are pointing to the first
    /// interrupt vector of each 8259 chip.
    /// 
    /// # Unsafe
    /// 
    /// This function will not check if the chosen offsets overlap with each other or do they
    /// overlap with CPU exceptions.
    pub const unsafe fn new_unchecked(master_offset: u8, slave_offset: u8) -> Self {
        Self {
            initialized: false,
            master: Pic::new(master_offset, 0x20, 0x21),
            slave: Pic::new(slave_offset, 0xa0, 0xa1),
        }
    }

    /// Reads the interrupt masks of both PICs.
    #[deprecated(since = "1.0.0", note = "Use [´get_mask´] to get a convenient 16-bit [´IrqMask´] structure instead.")]
    pub unsafe fn read_masks(&mut self) -> [u8; 2] {
        [self.master.mask_read().bits(), self.slave.mask_read().bits()]
    }
    
    /// Writes the interrupt masks of both PICs.
    #[deprecated(since = "1.0.0", note = "Use [´set_mask´] to apply the mask conveniently via [´IrqMask´] structure.")]
    pub unsafe fn write_masks(&mut self, mask1: u8, mask2: u8) {
        self.master.mask_write(OCW1::from(mask1));
        self.slave.mask_write(OCW1::from(mask2));
    }
}

bitflags! {
    /// IRQ Flags for 16 PIC Interrupts.
    ///
    /// These represent the 16 possible IRQ lines that the PIC can handle. Each line corresponds to a specific hardware 
    /// interrupt source.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct IrqMask: u16 {
        /// **IRQ0** - System timer interrupt.
        /// Triggered by the system timer (PIT). Essential for task switching and system ticks.
        const IRQ0_TIMER = 1 << 0,

        /// **IRQ1** - PS/2 Keyboard interrupt.
        /// Generated when a key is pressed or released on the primary keyboard.
        const IRQ1_PS2_KEYBOARD = 1 << 1,

        /// **IRQ3** - Serial port 2 (COM2) interrupt.
        /// Triggered by activity on the second serial port.
        const IRQ3_SERIAL_PORT2 = 1 << 3,

        /// **IRQ4** - Serial port 1 (COM1) interrupt.
        /// Triggered by activity on the first serial port.
        const IRQ4_SERIAL_PORT1 = 1 << 4,

        /// **IRQ5** - Parallel port 2 interrupt (or sound card).
        /// Often used for parallel port 2, but may be reassigned to other devices like a sound card.
        const IRQ5_PARALLEL_PORT2 = 1 << 5,

        /// **IRQ6** - Diskette drive (floppy disk controller) interrupt.
        /// Used for floppy disk read/write operations.
        const IRQ6_DISKETTE_DRIVE = 1 << 6,

        /// **IRQ7** - Parallel port 1 interrupt.
        /// Commonly associated with parallel port 1, typically used for printers.
        const IRQ7_PARALLEL_PORT1 = 1 << 7,

        /// **IRQ8** - Real-Time Clock (RTC) interrupt.
        /// Generated by the RTC for timekeeping purposes.
        const IRQ8_RTC = 1 << 8,

        /// **IRQ9** - CGA vertical retrace interrupt (or general use).
        /// Historically used for CGA video cards. Now typically available for general-purpose use.
        const IRQ9_CGA_VERTICAL_RETRACE = 1 << 9,

        /// **IRQ10** - Free for general-purpose use (first available line).
        /// Not assigned to specific hardware by default.
        const IRQ10_FREE_1 = 1 << 10,

        /// **IRQ11** - Free for general-purpose use (second available line).
        /// Not assigned to specific hardware by default.
        const IRQ11_FREE_2 = 1 << 11,

        /// **IRQ12** - PS/2 Mouse interrupt.
        /// Triggered by activity on the PS/2 mouse.
        const IRQ12_PS2_MOUSE = 1 << 12,

        /// **IRQ13** - Floating Point Unit (FPU) interrupt.
        /// Used for floating-point arithmetic errors or related conditions.
        const IRQ13_FPU = 1 << 13,

        /// **IRQ14** - Primary ATA channel interrupt.
        /// Handles interrupts from devices on the primary ATA (IDE) bus, such as the main hard drive.
        const IRQ14_PRIMARY_ATA = 1 << 14,

        /// **IRQ15** - Secondary ATA channel interrupt.
        /// Handles interrupts from devices on the secondary ATA (IDE) bus, such as additional drives.
        const IRQ15_SECONDARY_ATA = 1 << 15,
    };
}
