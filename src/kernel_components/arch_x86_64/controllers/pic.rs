/// A module for managing the PIC and working with software interrupts.
/// 
/// This is a virtualization of the Intel 8259 programmable interrupt controller. For real
/// OS development is better to use the APIC instead.

use alloc::vec::Vec;

use crate::{single, bitflags};
use crate::kernel_components::sync::Mutex;
use crate::kernel_components::arch_x86_64::ports::{GenericPort, PortAccessType};
use crate::kernel_components::arch_x86_64::post::DEBUG_BOARD;
use super::pic_command_words::{ICW1, ICW3, ICW4, OCW2, OCW3};

/// A struct, representing the individual PIC chip.
/// 
/// This can be either master or slave chip. Each chip has it's command port and data port.
/// The offset is used to handle different interrupt events.
#[derive(Debug, Clone, Copy)]
pub struct ChipPIC {
    /// An offset that hold an information about the current IRQ. Both master and slave pic
    /// chip has 8 inputs.
    offset: u8,
    /// I/O port where the commands must be send.   
    pub command: GenericPort<u8>,
    /// I/O port where the data can be send and received.
    pub data: GenericPort<u8>,
}

impl ChipPIC {
    /// Creates a new isntance of a PIC chip.
    /// 
    /// # Note
    /// 
    /// This is only a single chip. The whole pic consists of two those chips, each having
    /// their own ports and interrupts inputs.
    #[inline]
    pub const unsafe fn new(offset: u8, command_port: u16, data_port: u16) -> Self {
        Self {
            offset,
            command: GenericPort::new(command_port, PortAccessType::READWRITE),
            data: GenericPort::new(data_port, PortAccessType::READWRITE),
        }
    }

    /// Checks if we are in charge of handling the specified interrupt.
    #[inline]
    pub const fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        self.offset <= interrupt_id && interrupt_id < self.offset + 8
    }

    /// Notify us that an interrupt has been handled and that we're ready for more.
    /// 
    /// # Unsafe
    /// 
    /// This function is unsafe, because we have to make sure that we are not handling
    /// an interrupt at that moment.
    #[inline]
    pub unsafe fn end_of_interrupt(&mut self) {
        self.command.write(
            OCW2::NON_SPECIFIC_EOI_COMMAND.into()
        );
    }

    /// Disables the 8259 chip.
    /// 
    /// # Unsafe
    /// 
    /// This function is unsafe, as it must be used only if you will not use the PIC chip anymore.
    #[inline]
    pub unsafe fn disable(&mut self) {
        self.data.write(u8::MAX);
    }

    /// Reads the value of the ISR.
    /// 
    /// The interrupt status register inside the PIC chip, shows the info about which interrupts are
    /// being serviced at that moment. The value will be flushed after the end_of_interrupt method.
    #[inline]
    pub fn read_isr(&mut self) -> u8 {
        self.command.write(
            (OCW3::ENABLE | OCW3::READ_IS_REG).into()
        );
        self.command.read()
    }

    /// Reads the value of the IRR.
    /// 
    /// The interrupt request register shows the requested interrupts that have been raised
    /// but are not being served yet. The value will be flushed after the end_of_interrupt method.
    pub fn read_irr(&mut self) -> u8 {
        self.command.write(
            (OCW3::ENABLE | OCW3::READ_IR_REG).into()
        );
        self.command.read()
    }
}

/// A static structure of a PIC controller.
/// 
/// This structure uses the first 16 IDT entries by default because the PIC controller is
/// using those values after the reboot. This can be changed before the initialization of 
/// the PIC controller with reinit() and reinit_chained() methods.
single! {
    pub PROGRAMMABLE_INTERRUPT_CONTROLLER: Mutex<PIC> = unsafe { Mutex::new(PIC::new_unchecked(0, 8)) } 
}

/// A struct, that represents a PIC controller, which is a two chained pic chip. One chip
/// is called master and another one is called slave. Both master and slave chip can handle
/// 8 interrupts types.
/// 
/// This controller is not suitable for a multithreaded environment, therefore it is better
/// to use APIC for that sort of purposes. Reprogram this controller to not use the first
/// interrupts slots in the IDT. If this controller will not be reprogrammed, the software
/// interrupts will overlap with the CPU exceptions and cause a general protection fault.
#[derive(Debug, Clone, Copy)]
pub struct PIC {
    pub master: ChipPIC,
    pub slave: ChipPIC,
}

impl PIC {
    /// Creates a new instance of PIC controller.
    /// 
    /// The master offset and slave offset are two offsets that are pointing to the first
    /// interrupt vector of each 8259 chip.
    /// 
    /// # Panics
    /// 
    /// This function will panic if the provided offsets will overlap with each other or
    /// collide with CPU exceptions.
    #[inline]
    pub const fn new(master_offset: u8, slave_offset: u8) -> Self {
        assert!(master_offset >= 32 || slave_offset >= 32, "Both master and slave offsets must not overlap with CPU exceptions.");
        assert!(master_offset.abs_diff(slave_offset) >= 8, "The master and slave offsets are overlapping with each other.");

        unsafe { PIC::new_unchecked(master_offset, slave_offset) }
    }

    /// Creates a new instance of a PIC chip
    /// 
    /// The offsets must point to the the chosen 16 entries from the IDT that will be used 
    /// for the software interrupts.
    /// 
    /// # Panics
    /// 
    /// This function will panic if the provided offsets will overlap with cpu exceptions. It
    /// will always prevent the overlapping between master and slave chips, because it makes
    /// an offset for them sequentially.
    #[inline]
    pub const fn new_chained(pic_offset: u8) -> Self {
        PIC::new(pic_offset, pic_offset + 8)
    }

    /// Returns an offset, which was used when master chip was mapped within the IDT. If chips were
    /// mapped in a chained way, this function is enough to get about all 16 interrupts location.
    #[inline]
    pub const fn get_master_offset(&self) -> u8 {
        self.master.offset
    }

    /// Returns an offset, which was used when slave chip was mapped within the IDT. Must only be
    /// used if the chips were not mapped in a chained way.
    #[inline]
    pub const fn get_slave_offset(&self) -> u8 {
        self.slave.offset
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
    #[inline]
    pub const unsafe fn new_unchecked(master_offset: u8, slave_offset: u8) -> Self {
        Self {
            master: ChipPIC::new(master_offset, 0x20, 0x21),
            slave: ChipPIC::new(slave_offset, 0xa0, 0xa1),
        }
    }

    /// Reinitialized the PIC controller to change the offset indexes.
    /// 
    /// # Warn
    /// 
    /// No changes will take place until the remap() method is not being used.
    #[inline]
    pub fn reinit(&mut self, master_offset: u8, slave_offset: u8) -> &mut Self {
        *self = PIC::new(master_offset, slave_offset);
        self
    }

    /// Reinitialized the PIC controller to change the offset indexes. This function creates
    /// a PIC with chained master and slave interrupt vectors.
    /// 
    /// # Warn
    /// 
    /// No changes will take place until the remap() method is not being used.
    #[inline]
    pub fn reinit_chained(&mut self, pic_offset: u8) -> &mut Self {
        *self = PIC::new_chained(pic_offset);
        self
    }

    /// Remaps the PIC controller to a new place in the IDT table.
    /// 
    /// 
    #[inline]
    pub fn remap(&mut self) {
        // A lot of writes are needed to remap the PIC, because it have to be reinitialized
        // fully, as all ICW commands must be written in a right order and they must not repeat.

        // Saving the values that was before the data change.
        let master_mask = self.master.data.read();
        let slave_mask = self.slave.data.read();

        // Initialize the master chip. (ICW1)
        self.master.command.write(
            (ICW1::ENABLE | ICW1::IC4).into()
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.

        // Initialize the slave chip. (ICW1)
        self.slave.command.write(
            (ICW1::ENABLE | ICW1::IC4).into()
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.


        // Writing the offset to the master chip. (ICW2)
        self.master.data.write(
            self.master.offset
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.

        // Writing the offset to the slave chip. (ICW2)
        self.slave.data.write(
            self.slave.offset
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.


        // Telling that the slave is connected to the 3rd interrupt input. (ICW3)
        self.master.data.write(
            ICW3::SLAVE2.into()
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.

        // Writing the slave that it has the ID of 2. (ICW3)
        self.slave.data.write(
            2
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.


        // Using 8086 mode for the master chip. (ICW4)
        self.master.data.write(
            ICW4::NPM.into()
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.

        // Using 8086 mode for the slave chip. (ICW4)
        self.slave.data.write(
            ICW4::NPM.into()
        );
        DEBUG_BOARD.write(0); // Cause a mini sleep to wait for the controller to perform a task.
    
        // Writing the masks back.
        self.master.data.write(master_mask);
        self.slave.data.write(slave_mask);
    }

    /// Disables the PIC controller
    /// 
    /// This function is needed to change from PIC to APIC. By default the CPU is using a
    /// PIC controller, so it should be disabled to move on.
    #[inline]
    pub fn disable(&mut self) {
        unsafe {
            self.master.disable();
            self.slave.disable();
        }
    }

    /// Returns the bits of interrupts vectors that are being served at that moment by the
    /// IDT handler function. Those values will be flushed after the end of interrupt.
    ///
    /// The interrupt status register inside the PIC chip, shows the info about which interrupts are
    /// being serviced at that moment. The value will be flushed after the end_of_interrupt method.
    #[inline]
    pub fn get_serviced_interrupts(&mut self) -> u16 {
        let master = self.master.read_isr() as u16;
        let slave = self.slave.read_isr() as u16;

        ((slave << 8 ) | master)
    }

    /// Returns the bits of interrupts vectors that are being raised at that moment by the
    /// 8259 chip, but yet to be served by the IDT handler function. Those values will be flushed after the end of interrupt.
    ///
    /// The interrupt request register shows the requested interrupts that have been raised
    /// but are not being served yet. The value will be flushed after the end_of_interrupt method.
    #[inline]
    pub fn get_requested_interrupts(&mut self) -> u16 {
        let master = self.master.read_isr() as u16;
        let slave = self.slave.read_isr() as u16;

        ((slave << 8 ) | master)
    }

    /// Decodes the given interrupt from the given value.
    /// 
    /// Returns the vector with all interrupt types that exist in the input value.
    #[inline]
    pub fn decode_interrupts(value: u16) -> Vec<PICInterruptType> {
        let mut output = Vec::new();

        for int_type in PICInterruptType::as_array() {
            if int_type.is_in(value) {
                output.push(int_type);
            }
        }

        output
    }
}

bitflags! {
    /// Interrupts types from the PIC controller.
    /// 
    /// Each interrupt has it's inner interrupt vector. The PIC controller consists of the master
    /// chip and the slave chip. Both of them are 8259 chips which must be configured properly for
    /// interacting with IDT.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PICInterruptType: u16 {
        // Master

        /// Represents the interrupt for the system timer, which generates periodic timer interrupts. 
        /// This interrupt is crucial for task scheduling and time management in the OS.
        const TIMER =               1,
        /// Indicates the interrupt generated by the keyboard controller. It's used to 
        /// handle keyboard input events, enabling user interaction with the system.
        const KEYBOARD =            1 << 1,
        /// Denotes the interrupt from the slave PIC.
        const CASCADE =         1 << 2,
        /// Represents an interrupt generated by the second serial port. It's used for serial 
        /// communication with external devices.
        const SERIAL_PORT_2 =       1 << 3,
        ///  Indicates an interrupt from the first serial port, often used for serial 
        /// communication with external devices.
        const SERIAL_PORT_1 =       1 << 4,
        /// This interrupt is related to the parallel ports, specifically ports 2 and 3. 
        /// Parallel ports are used for connecting printers and other parallel devices.
        const PARALLEL_PORT_2_3 =   1 << 5,
        /// Represents an interrupt associated with the floppy disk controller. It's used 
        /// for handling disk-related operations, such as reading and writing data to floppy 
        /// disks.
        const FLOPPY_DISK =         1 << 6,
        /// Denotes the interrupt generated by the first parallel port, used for connecting 
        /// parallel devices like printers.
        const PARALLEL_PORT_1 =     1 << 7,
        // Slave

        /// Represents the interrupt generated by the real-time clock (RTC) hardware. It's 
        /// essential for tracking time and date, scheduling tasks, and maintaining system time.
        const REAL_TIME_CLOCK =     1 << 8,
        /// Indicates an interrupt related to the Advanced Configuration and Power Interface 
        /// (ACPI). ACPI is used for power management and configuration of various system components.
        const ACPI =                1 << 9,
        /// Free interface
        const AVAILABLE_1 =         1 << 10,
        /// Free interface
        const AVAILABLE_2 =         1 << 11,
        /// Denotes the interrupt for a mouse controller. It's needed for handling mouse 
        /// input, enabling cursor movement and interaction.
        const MOUSE =               1 << 12,
        /// Represents an interrupt related to a co-processor, such as a math coprocessor 
        /// or accelerator. These devices can be used for specialized mathematical computations.
        const CO_PROCESSOR =        1 << 13,
        /// Indicates the interrupt for the primary ATA (Advanced Technology Attachment) 
        /// controller, typically used for managing hard disk drives.
        const PRIMARY_ATA =         1 << 14,
        /// Represents the interrupt for the secondary ATA controller. Like the primary ATA, 
        /// it's used for managing additional hard disk drives or storage devices.
        const SECONDARY_ATA =       1 << 15,
    }
}
