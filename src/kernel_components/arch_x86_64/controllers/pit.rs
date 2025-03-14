/// Module for PIT management. It allows to configure all three channels of this chip.

use crate::{
    bitflags, critical_section, kernel_components::arch_x86_64::{
        ports::{GenericPort, PortAccessType, SipoPort},
        post::DEBUG_BOARD,
    }, println
};

/// Constant value of PIT frequency in Hz.
pub const PIT_HZ: usize = 1193182;

/// Programmable Interval Timer.
///
/// Internal hardware timer that can be configured via this structure.
///
/// The oscillator used by PIT chip runs at 1.193182 MHz for historical reasons. Generates a pulse
/// on three channels when counter is zero based on current operating mode.
///
/// # Channels
/// - Channel 0: The output from PIT channel 0 will generate interrupts on IRQ0 via PIC controller.
/// - Channel 1: ABSOLETE channel, which was with DMA controller on older systems. Can be ignored.
/// - Channel 2: The output from this channel is connected to the PC speaker, so it's frequency
/// controls the frequency of the dound produced by the speaker.
#[derive(Debug)]
pub struct PIT {
    /// Timer signal on IRQ0 line.
    pub channel0: SipoPort<u8, u16>,
    /// ABSOLETE.
    pub channel1: SipoPort<u8, u16>,
    /// Timer signal connected to PC speaker.
    pub channel2: SipoPort<u8, u16>,
    /// Port for providing configuration commands to the PIT chip.
    command: GenericPort<u8>,
}

impl PIT {
    /// Creates a new instance of PIT for further configuration.
    pub fn new() -> Self {
        Self {
            channel0: SipoPort::<u8, u16>::new(0x40, PortAccessType::READWRITE),
            channel1: SipoPort::<u8, u16>::new(0x41, PortAccessType::READWRITE),
            channel2: SipoPort::<u8, u16>::new(0x42, PortAccessType::READWRITE),
            command: GenericPort::new(0x43, PortAccessType::WRITEONLY),
        }
    }

    /// Reads the current value of one selected channel. To read multiple channels at once, use
    /// readback.
    ///
    /// # Panics
    ///
    /// Will panic if the channel value is out of range.
    pub fn read_latch(&mut self, channel: u8) -> u16 {
        let cmd = PITCommand::from(PITCommand::CHANNEL0.bits() + channel) | PITCommand::LATCH_COUNT_VALUE_CMD;
        unsafe {
            self.command(cmd);
            match channel {
                0 => self.channel0.read(),
                1 => self.channel1.read(),
                2 => self.channel2.read(),
                _ => panic!("PIT channel out of range: {}", channel),
            }
        }
    }

    /// Based on the providing readback command, returns a readback output from the selected
    /// channels.
    ///
    /// The readback can be a current counter value from some/all channels, so it would act as an
    /// atomic read_latch for multiple channels. Another form of readback is the status code, which
    /// is literraly the last command used on one of those ports.
    ///
    /// # Warn
    ///
    /// This function may not work on some target devices at all. The PIT chip must be a 8254 chip,
    /// which is currently used on all non-absolete architectures. Some VMs however may also have
    /// problems virtualizing this command.
    pub fn read_back(
        &mut self,
        c0: bool, 
        c1: bool, 
        c2: bool, 
        cmd: PITReadbackCMD
    ) -> PITReadback {
        let mut raw = PITCommand::READBACK;
        let mut readback;
        if c0 { raw |= PITCommand::R_CHANNEL0 };
        if c1 { raw |= PITCommand::R_CHANNEL1 };
        if c2 { raw |= PITCommand::R_CHANNEL2 };

        match cmd {
            // It has to be this way.
            PITReadbackCMD::ChannelStatus => { 
                raw |= PITCommand::R_LATCH_COUNT_FLAG;
                readback = PITReadback::ChannelStatus([0.into(), 0.into(), 0.into()]);
            },
            PITReadbackCMD::ChannelCounterValue => { 
                raw |= PITCommand::R_LATCH_STATUS_FLAG; 
                readback = PITReadback::ChannelCounterValue([0, 0, 0]);
            },
        }

        unsafe {
            self.command(raw);

            match &mut readback {
                // Read one status byte on each channel.
                PITReadback::ChannelStatus(mut b) => { 
                    if c0 { b[0] = (self.channel0.read() as u8).into() }
                    if c1 { b[1] = (self.channel1.read() as u8).into() }
                    if c2 { b[2] = (self.channel2.read() as u8).into() }
                },
                // Read a whole word on each channel.
                PITReadback::ChannelCounterValue(mut b) => {
                    if c0 { b[0] = self.channel0.read() }
                    if c1 { b[1] = self.channel1.read() }
                    if c2 { b[2] = self.channel2.read() }
                },
            }
        }
        readback
    }

    /// Each channel is a 16-bit timer, which value can be changed with this function. The current
    /// command configuration of the PIT timer won't be affected by this command.
    ///
    /// # Panics
    ///
    /// Will panic if the channel value is out of range.
    pub fn override_timer(&mut self, channel: u8, val: u16) {
        use PITCommand::*;
        let ch = match channel {
            // Sipo ports will ensure the proper writing order.
            0 => { self.channel0.write(val) },
            1 => { self.channel1.write(val) },
            2 => { self.channel2.write(val) },
            _ => panic!("PIT channel out of range: {}", channel),
        };
    }

    /// Sends a raw command byte to the command port.
    ///
    /// # Unsafe
    ///
    /// Wrong commands would not work or do something even worse. It is better to use safe wrappers
    /// to perform specific tasks on the PIT chip channels.
    pub unsafe fn command(&mut self, cmd: PITCommand) {
        self.command.write(cmd.bits());
    }
}

/// PIT Readback Command
///
/// A special command that allows to read the current configuration on a certain channel/channels.
/// It can also be used as a quicker alternative to Latch Command
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PITReadbackCMD {
    /// Read the current channel status (last command used).
    ChannelStatus,
    /// Read the current counter value.
    ChannelCounterValue,
}

/// PIT Readback output.
///
/// Can be either a configured status on a certain channel or the current counter value on this
/// channel. Unused channels will be 0 values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PITReadback { 
    /// Read the current channel status (last command used).
    ChannelStatus([PITCommand; 3]),
    /// Read the current counter value.
    ChannelCounterValue([u16; 3]),
}

bitflags! {
    /// PIT Command byte that must be written to it's command port.
    ///
    /// The command port expects the following byte structure:
    ///
    /// # Select Channel
    ///
    /// Selects which channel is being configured, and must be valid on every write of command register. 
    /// The READBACK variant is a very special case, it which the rest of the command structure is
    /// different (written below). It allows to read a current configuration state from one of the
    /// channels.
    ///
    /// # Access Mode
    ///
    /// Specifies the order data will be provided to the channels data port, because 16-bit value
    /// is expected, while all ports are 8-bit. It can also specify the Counter Latch command to
    /// the CTC timer. If this command is used, the rest LSB bytes must be zero. It allows to read the
    /// current count value on each channel properly.
    ///
    /// Because all ports are 8-bit, reading current count is not atomic and may lead to completely
    /// wrong values. This command allows to read the current count value properly, so that the
    /// full word value will be available under a port until it is read fully by the OS.
    ///
    /// # Operating Mode
    ///
    /// Three bits that define 6 different operation modes for the selected channel.
    ///
    /// # Operating Format
    ///
    /// Defines if the selected PIT channel will operate on binary 16-bit value or 4-bit BCD value.
    /// It is preffered and much much easier to use binary format, which allows for larger
    /// range of frequencies.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PITCommand: u8 {
        /* Select Channel */

        /// Select the Channel 0 (IRQ0).
        const CHANNEL0                          = 0b00 << 6,
        /// Select the Channel 1 (Why would you do that?).
        const CHANNEL1                          = 0b01 << 6,
        /// Select the Channel 2 (PC Speaker).
        const CHANNEL2                          = 0b10 << 6,
        /// Provide a readback command. (See bellow).
        const READBACK                          = 0b11 << 6,

        /* Access Mode */

        /// Use counter latch command (Rest of the LSB bytes must be 0).
        const LATCH_COUNT_VALUE_CMD             = 0b00 << 4,
        /// Expect low byte only
        const LOW_BYTE_ONLY                     = 0b01 << 4,
        /// Expect high byte only
        const HIGH_ONLY                         = 0b01 << 4,
        /// Expect full word
        const FULL_WORD                         = 0b01 << 4,

        /* Operating Mode */

        /// When used the timer’s output goes low and waits for you to set a starting value 
        /// (reload value). Once set, the timer begins counting down on each clock pulse (1.193182 MHz). 
        /// When the countdown reaches zero, the output goes high and stays high until you reset the timer 
        /// or set a new reload value. The timer keeps counting after zero, but the output remains unaffected. 
        /// You can change the reload value anytime, and the timer will use it on the next clock pulse.
        ///
        /// Generates outputs only on Channel 0.
        const INT_ON_TERMINAL_COUNT             = 0b000 << 1,
        /// In this mode, the timer waits for a rising edge on the gate input before starting. This mode isn't 
        /// usable for PIT channels 0 or 1 since their gate inputs can't be changed. Besides that
        /// works the same as the previous mode.
        const HARDWARE_RETRIGGERABLE_ONE_SHOT   = 0b001 << 1,
        /// This mode divides the input frequency to produce a steady output signal. The timer starts counting 
        /// down after a reload value is set, with the output going low briefly when the count reaches 1 before 
        /// resetting. If the gate input goes low, counting pauses, and resumes when the gate returns high. 
        /// Reload values can be changed anytime, but only take effect after the current count resets. This mode 
        /// is typically used for accurate timing, but it's unsuitable for generating sounds.
        const RATE_GENERATOR                    = 0b010 << 1,
        /// Current mode works like one above, but outputs a square wave instead of a short pulse. The timer flips 
        /// its output state each time the frequency divider reaches zero, creating a 50% duty cycle (even though 
        /// odd reload values can cause slight timing imbalances). When the reload register is set, the timer starts 
        /// counting down. Each count decrements twice per input signal cycle to compensate for the flip-flop. 
        ///
        /// Reload values can be changed anytime but only take effect after the current cycle completes. It is is 
        /// commonly used for regular IRQ0 ticks, but avoid using a divisor of one.
        const SQUARE_WAVE_GENERATOR             = 0b011 << 1,
        /// In this mode the timer acts as a retriggerable delay, generating a pulse when the countdown hits zero. 
        /// Initially, the output is high. After setting the reload value, the timer starts counting down. When the 
        /// count reaches zero, the output goes low for one input signal cycle (0.8381 µS), then continues counting 
        /// without resetting the output. If the gate input goes low, the countdown pauses but the output remains 
        /// unaffected. Reload values can be changed anytime and take effect after the next input signal edge.
        const SOFTWARE_TRIGGERED_STROBE         = 0b100 << 1,
        /// Works like previous mode but starts counting after a rising edge of the gate input, making it unsuitable 
        /// for channels 0 or 1 (where the gate can't change).
        const HARDWARE_TRIGGERED_STROBE         = 0b101 << 1,

        /* BCD/Binary mode */
        const BINARY16BIT                       = 0,
        const BCD4BIT                           = 1,

        /*  Readback Command */

        /// If CLEAR, then any/all selected channels will have their current count copied
        /// into their latch register. 
        const R_LATCH_COUNT_FLAG                = 1 << 5,
        /// If CLEAR, then any/all selected channels will return a status byte on the next read
        /// from their corresponding port.  
        const R_LATCH_STATUS_FLAG               = 1 << 4,
        /// Selects channel 0 with readback command.
        const R_CHANNEL0                        = 1 << 3,
        /// Selects channel 1 with readback command.
        const R_CHANNEL1                        = 1 << 2,
        /// Selects channel 2 with readback command.
        const R_CHANNEL2                        = 1 << 1,
        // Bit 0 is reserved and must be 0.
    }
}
