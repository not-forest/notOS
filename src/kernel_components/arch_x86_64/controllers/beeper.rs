/// x86 architecture provides a legacy PC Speaker device, which is the most primitive way of
/// creating simple sounds (beeps). This module provides a low level controller for this peripheral, 
/// which can be used to create produce simple sound combinations.
///
/// Beeper shall only be used with no sound card available, or due to the lack of driver
/// implementation.

use super::pit::{PIT, PITCommand, PIT_HZ};
use super::super::ports::{GenericPort, PortAccessType};
use crate::kernel_components::registers::nmi::NMI_STS_CNT;
use crate::critical_section;

/// Modes of operation for PC Speaker device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeeperModeOfOperation {
    /// Manual operation via software calls. 
    MANUAL,
    /// PC Speaker is directly connected to the second output of the PIT timer.
    PIT,
    /// Controlled via PWM signal feed to the PC Speaker. This requires one CPU thread to
    /// constantly provide the PWM signal to the beeper.
    PWM,
}

/// Beeper (PC Speaker)
///
/// Primitive legacy sound device for creating simple beeps or squeaks. Can be used with several
/// modes of operation, to perform simple sound combinations. More in [´BeeperModeOfOperation´]
#[derive(Debug)]
pub struct PCBeeper {
    mop: BeeperModeOfOperation,
    pub pit: Option<PIT>,
}

impl PCBeeper {
    /// Creates a new instance of PCBeeper.
    ///
    /// Beeper won't own PIT timer controller until used in PIT operation mode.
    #[inline]
    pub fn new() -> Self {
        Self {
            mop: BeeperModeOfOperation::MANUAL,
            pit: None,
        }
    }

    /// Creates a new instance of PCBeeper with loaded PIT timer controller for use as a driver.
    pub fn new_with_pit(pit: PIT) -> Self {
        Self {
            mop: BeeperModeOfOperation::PIT,
            pit: Some(pit),
        }
    }

    /// Plays a sound with provided frequency. 
    ///
    /// # Warn
    ///
    /// This code overrides the state of PIT's channel 2.
    pub fn play(&mut self, freq: usize) {
        if let Some(pit) = self.pit.as_mut() {
            use PITCommand::*;
            let div = PIT_HZ / freq;

            unsafe {
                pit.command(CHANNEL2 | FULL_WORD | RATE_GENERATOR);
                pit.override_timer(2, div as u16);
            }
        }

        // Any changes to NMI registers shall be atomic.
        critical_section!(|| {
            let nmi = NMI_STS_CNT::read();
            let mask = NMI_STS_CNT::TIM_CNT2_EN | NMI_STS_CNT::SPKR_DAT_EN; 

            // Only enabling PIT's timer 2 output if necessary.
            if ((nmi & mask).is_empty()) {
                crate::println!("0b{:b}", (nmi | mask).bits());
                NMI_STS_CNT::write(nmi | mask);
            }
        });
    }

    /// Forces the beeper to stop making any sound.
    pub fn stop(&mut self) {
        let mask = !(NMI_STS_CNT::TIM_CNT2_EN | NMI_STS_CNT::SPKR_DAT_EN);
        // Any changes to NMI registers shall be atomic.
        critical_section!(|| NMI_STS_CNT::write(NMI_STS_CNT::read() & mask));
    }
}
