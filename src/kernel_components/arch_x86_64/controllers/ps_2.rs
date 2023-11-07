/// A module that provides interactions with PS/2 controller.

use crate::{
    kernel_components::arch_x86_64::ports::{GenericPort, PortAccessType}, 
    bitflags, Vec
};

/// A struct representing a 8042 PS/2 controller.
/// 
/// The PS/2 controller is often called a "Keyboard controller", which is usually used to
/// read the data from the data port, which holds info about the pressed key from the PS/2
/// Keyboard.
/// 
/// # Data port
/// 
/// The data port is used for reading data that was received from a PS/2 device or from the PS/2
/// controller itself. Usually iy used with PS/2 Keyboard, thats why it is usually being used
/// as a port for reading the pressed key, after the keyboard interrupt was caused.
/// 
/// # Status register
/// 
/// The status register contains flags that show the state of the controller. More info can
/// be found in 'SRFlags'.
/// 
/// # Command register 
/// 
/// The PS/2 controller accepts commands and performs them. These commands are not the same
/// that are sent to a PS/2 device. More info can be found in 'CRFlags'.
pub struct PS2 {
    /// Holds the information about the pressed key.
    data_port: GenericPort<u8>,
    /// Status register contains various flags that show the state of the PS/2 controller.
    status_register: GenericPort<u8>,
    /// The command port is used for sending commands to the PS/2 devices.
    command_register: GenericPort<u8>,
}

impl PS2 {
    /// Creates a new instance of PS2.
    /// 
    /// Each instance will always be the same, because this controller does not have any
    /// reprogrammable part.
    pub const fn new() -> Self {
        Self {
            data_port: GenericPort::new(0x60, PortAccessType::READWRITE),
            status_register: GenericPort::new(0x64, PortAccessType::READONLY),
            command_register: GenericPort::new(0x64, PortAccessType::WRITEONLY),
        }
    }

    /// Reads the value from the data port of the PS/2 controller.
    #[inline]
    pub fn read_data(&self) -> u8 {
        self.data_port.read()
    }

    /// Reads the value from the status register in the PS/2 controller.
    #[inline]
    pub fn read_status(&self) -> u8 {
        self.status_register.read()
    }

    /// Decodes the input value as a vector of SRFlags.
    #[inline]
    pub fn decode_status(value: u8) -> Vec<SRFlags> {
        let mut output = Vec::new();

        for bit in SRFlags::as_array() {
            if bit.is_in(value) {
                output.push(bit);
            }
        }

        output
    }

    /// Writes the command to the command port of the PS/2 controller. Note that these commands
    /// are not the commands that are sent to a PS/2 devices. Those commands are only for the controller
    /// itself.
    /// 
    /// # Note
    /// 
    /// If the command is two-bytes long, the second half is being written to the data port (0x60)
    /// after making sure that the controller is ready for it. If there is a response byte, then the
    /// response byte needs to be read from the data port (0x60).
    #[inline]
    pub unsafe fn write_command(&mut self, command: PSControllerCommand) -> Option<ResponseByte> {
        if command.is_doubled() {
            self.command_register.write(command.primary);

            while SRFlags::OUTPUT_BUFFER_STATUS.is_in(self.status_register.read()) {}

            self.data_port.write(command.secondary)
        } else {
            self.command_register.write(command.primary);
        }

        
        if command.has_response() {
            while !SRFlags::OUTPUT_BUFFER_STATUS.is_in(self.status_register.read()) {}
            
            Some(
                ResponseByte::new(self.data_port.read())
            )
        } else {
            None
        }
    }
}

/// This struct represents the command that must be given to the PS/2 controller.
/// 
/// Some commands are structured our of two different bytes. This struct hides this
/// complicity by its methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PSControllerCommand {
    /// The main command
    primary: u8,
    /// The "second" command.
    secondary: u8,
}

impl PSControllerCommand {
    /// Creates a custom controller command.
    #[inline]
    pub const fn new(primary: u8, secondary: u8) -> Self {
        Self { primary, secondary }
    }

    /// Creates the new command that can read the controller configurations byte.
    /// 
    /// The output can be parsed by 'PSControllerConfigurations::parse()' for more readable
    /// format.
    #[inline]
    pub const fn read_controller_configurations() -> Self {
        Self { primary: 0x20, secondary: 0 }
    }

    /// Reads the nth byte from the local RAM.
    /// 
    /// The range of the index must be from 1 to 32. With index = 1, this function is equal to
    /// reading the PS/2 controller configuration. The other memory regions of the internal RAM
    /// has no standard purpose, so it is platform specific.
    #[inline]
    pub const fn read_nth_byte(index: u8) -> Self {
        let pr = 0x20 + index - 1;
        assert!(pr <= 0x3f || index == 0, "The index is out of range.");

        Self {
            primary: pr,
            secondary: 0, 
        }
    }

    /// Overwrites the configuration of the PS/2 controller.
    #[inline]
    pub const fn write_configuration_byte(config: PSControllerConfiguration) -> Self {
        Self {
            primary: 0x60,
            secondary: config.bits(),
        }
    }

    /// Overwrites the chosen byte on a chosen index with own byte.
    /// 
    /// The range of the index must be from 1 to 32. With index = 1, this function is equal
    /// to writing the configuration byte of the controller configurations.
    #[inline]
    pub const fn write_byte(index: u8, byte: u8) -> Self {
        let pr = 0x20 + index - 1;
        assert!(pr <= 0x3f || index == 0, "The index is out of range.");

        Self { 
            primary: pr,
            secondary: byte, 
        }
    }

    /// Disables the second PS/2 port.
    #[inline]
    pub const fn disable_second_port() -> Self {
        Self {
            primary: 0xa7,
            secondary: 0,
        }
    }

    /// Enables the second PS/2 port.
    #[inline]
    pub const fn enable_second_port() -> Self {
        Self {
            primary: 0xa8,
            secondary: 0,
        }
    }

    /// Test the second PS/2 port.
    /// 
    /// The return value can be:
    /// - 0x00 (test passed);
    /// - 0x01 (clock line stuck low);
    /// - 0x02 (clock line stuck high);
    /// - 0x03 (data line stuck low);
    /// - 0x04 (data line stuck high);
    #[inline]
    pub const fn test_second_port() -> Self {
        Self {
            primary: 0xa9,
            secondary: 0,
        }
    }

    /// Disables the first PS/2 port.
    #[inline]
    pub const fn disable_first_port() -> Self {
        Self {
            primary: 0xad,
            secondary: 0,
        }
    }

    /// Enables the first PS/2 port.
    #[inline]
    pub const fn enable_first_port() -> Self {
        Self {
            primary: 0xae,
            secondary: 0,
        }
    }

    /// Test the first PS/2 port.
    /// 
    /// The return value can be:
    /// - 0x00 (test passed);
    /// - 0x01 (clock line stuck low);
    /// - 0x02 (clock line stuck high);
    /// - 0x03 (data line stuck low);
    /// - 0x04 (data line stuck high);
    #[inline]
    pub const fn test_first_port() -> Self {
        Self {
            primary: 0xab,
            secondary: 0,
        }
    }

    /// Tests the PS/2 controller itself.
    /// 
    /// The return value can be:
    /// - 0x55 (test passed);
    /// - 0xfc (test failed);
    #[inline]
    pub const fn test_ps_controller() -> Self {
        Self {
            primary: 0xaa,
            secondary: 0,
        }
    }

    /// Diagnostic dump
    /// 
    /// Reads all bytes of internal RAM. The output is undefined.
    #[inline]
    pub const fn diagnostic_dump() -> Self {
        Self {
            primary: 0xac,
            secondary: 0,
        }
    }

    /// Reads the input port of the controller.
    /// 
    /// The output is no standard or well defined.
    #[inline]
    pub const fn read_input_port() -> Self {
        Self {
            primary: 0xc0,
            secondary: 0,
        }
    }

    /// Writes the given byte to the controllers output port.
    /// 
    /// # Warn
    /// 
    /// This must be done after checking that the output buffer is empty.
    #[inline]
    pub const fn write_to_output_port(byte: u8) -> Self {
        Self {
            primary: 0xd1,
            secondary: byte,
        }
    }

    /// Writes the given byte to the controllers first output port.
    /// 
    /// This action works as if the controller received a new data from the keyboard. The
    /// data will be new byte that you provided.
    #[inline]
    pub const fn write_to_first_output_port(byte: u8) -> Self {
        Self {
            primary: 0xd2,
            secondary: byte,
        }
    }

    /// Writes the given byte to the controllers first input port.
    /// 
    /// Sends a byte to the first input port.
    #[inline]
    pub const fn write_to_first_input_port(byte: u8) -> Self {
        Self {
            primary: 0xd4,
            secondary: byte,
        }
    }

    /// Writes the given byte to the controllers second output port.
    /// 
    /// This action works as if the controller received a new data from the mouse. The
    /// data will be new byte that you provided.
    #[inline]
    pub const fn write_to_second_output_port(byte: u8) -> Self {
        Self {
            primary: 0xd3,
            secondary: byte,
        }
    }

    /// Writes the given byte to the controllers second input port.
    /// 
    /// Sends a byte to the second input port.
    #[inline]
    pub const fn write_to_second_input_port(byte: u8) -> Self {
        Self {
            primary: 0xd5,
            secondary: byte,
        }
    }

    #[inline]
    pub const fn read_output_port() -> Self {
        Self {
            primary: 0xd0,
            secondary: 0,
        }
    }

    /// Copies the first 4 bits of the input port to the end of status register.
    #[inline]
    pub const fn copy_0_3_to_status() -> Self {
        Self {
            primary: 0xc1,
            secondary: 0,
        }
    }

    /// Copies the last 4 bits of the input port to the end of status register.
    #[inline]
    pub const fn copy_4_7_to_status() -> Self {
        Self {
            primary: 0xc2,
            secondary: 0,
        }
    }

    /// Pulse output line low for 6 ms.
    /// 
    /// Bits 0 to 3 are used as a mask:
    /// - 0 (pulse line);
    /// - 1 (don't pulse line);
    /// and correspond to 4 different output lines.
    #[inline]
    pub const fn pulse(value: u8) -> Self {
        assert!(value <= 0x0f, "Value is out of range.");

        Self {
            primary: 0xf0 + value,
            secondary: 0,
        }
    }

    /// Checks if the following command will generate a response.
    #[inline]
    pub fn has_response(&self) -> bool {
        match self.primary {
            0x20..=0x3f => true,
            0xa9 => true,
            0xaa => true,
            0xab => true,
            0xac => true,
            0xc0 => true,
            0xd0 => true,
            _ => false,
        }
    }

    /// Says if the command is two bytes long or not.
    #[inline]
    pub fn is_doubled(&self) -> bool {
        self.secondary == 0
    }
}

/// A single byte that corresponds to the output of the various commands that can be written
/// to the PS/2 controller's command port.
#[repr(transparent)]
pub struct ResponseByte(u8);

impl ResponseByte {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}

bitflags! {
    /// Flags that corresponds the status register (SR) in the PS/2 controller.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SRFlags: u8 {
        /// Tells the status of the output buffer. Where 0 - empty and 1 - full.
        const OUTPUT_BUFFER_STATUS =    1,
        /// Tells the status of the input buffer. Where 0 - empty and 1 - full.
        const INPUT_BUFFER_STATUS =     1 << 1,
        /// Meant to be cleared on reset and set by firmware is the system passes
        /// self tests (POST).
        const SYSTEM_FLAG =             1 << 2,
        /// Command or data. 0 means that data written to input buffer is data for PS/2
        /// device. 1 means that the data written to input is data for PS/2 controller.        
        const COMMAND_DATA =            1 << 3,
        /// Keyboard lock. Chipset specific error that will not occur on modern systems.
        const KEYBOARD_LOCK =           1 << 4,
        /// May be the time out or the second PS/2 port output buffer is full error.        
        const UNKNOWN =                 1 << 5,
        /// Time-out error.
        const TIMEOUT =                 1 << 6,
        /// Parity error.       
        const PARITY_ERROR =            1 << 7,
    };

    /// The controller configuration byte.
    /// 
    /// It can be read by the 0x20 command and hold information about the PS/2 controller
    /// configuration.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PSControllerConfiguration: u8 {
        /// Says if the interrupts on the first port are enabled.
        const FIRST_PORT_INTERRUPT_ENABLE = 1,
        /// Says if the interrupts on the second port are enabled.        
        const SECOND_PORT_INTERRUPT_ENABLE = 1 << 1,
        /// If set, the system passed POST. Cannot be unset, because it means that your
        /// OS shouldn't be able to run.
        const SYSTEM_FLAG = 1 << 2,
        // the fourth bit must be zero.
        /// First clock port enable
        const FIRST_PS_2_CLOCK_PORT_ENABLE = 1 << 4,
        /// Second clock port enable        
        const SECOND_PS_2_CLOCK_PORT_ENABLE = 1 << 5,
        /// First translation port enable.
        const FIRST_PS_2_PORT_TRANSLATION = 1 << 6,
        // The last bit must be zero.
    };
}
