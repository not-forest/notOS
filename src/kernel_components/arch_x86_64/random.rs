// Random things using x86_64's RDRAND opcode.

use core::arch::x86_64 as arch;

/// Container enum that decides which random source will be used on acquired hardware.
/// The None option will disappear after the implementation of PseudoRd
#[derive(Clone, Copy, Debug)]
pub enum RandomSource {
    RdRand(RdRand), // The real random numbers from hardware that supports rdrand.
    RdSeed(RdSeed), // The real random seed-like numbers from hardware that supports rdseed.
    PseudoRd, // TODO! Pseudo random number generator.
    None, // None if something went wrong with pseudo random gen.
}

impl RandomSource {
    pub fn new() -> Self {
        if let Some(rdrand) = RdRand::new() {
            RandomSource::RdRand(rdrand)
        } else if let Some(rdseed) = RdSeed::new() {
            RandomSource::RdSeed(rdseed)
        } else {
            RandomSource::None
        }
    }
}

/// A true random numbers provided via hardware that supports RDRAND.
#[derive(Clone, Copy, Debug)]
pub struct RdRand(());

/// A true random seed numbers provided via hardware that supports RDSEED.
#[derive(Clone, Copy, Debug)]
pub struct RdSeed(());

/// The Random is a wrapper over random sources that uses the corresponding random source to generate random responses.
#[derive(Clone, Copy, Debug)]
pub struct Random(RandomSource);

impl Random {
    /// Creates a new instance of some random source. This process is automatic and will pick the best choice
    /// of source, based on hardware. This struct must be used for weaker implementations. For some specific cases is better
    /// to use RdRand and RdSeed.
    #[inline(always)]
    pub fn new() -> Self {
        Self(RandomSource::new())
    }

    /// Returns a tandom float between 0 and 1.
    #[inline]
    pub fn rand(&self) -> Option<f64> {
        match self.0 {
            RandomSource::RdRand(rdrand) => {
                let random_u64 = rdrand.get_u64()?;
                Some(random_u64 as f64 / u64::MAX as f64)
            }
            RandomSource::RdSeed(rdseed) => {
                let random_seed = rdseed.get_u64_seed()?;
                Some(random_seed as f64 / u64::MAX as f64)
            }
            _ => None,
        }
    }
}

impl RdRand {
    /// Creates the instance of RdRand if it is supported on hardware.
    #[inline(always)]
    pub fn new() -> Option<Self> {
        let cpuid = unsafe { arch::__cpuid(0x1) };
        if cpuid.ecx & (1 << 30) != 0 {
            Some(Self(()))
        } else {
            None
        }
    }

    /// Uniformly sampled u64. Returns a random u64 in the range 0..u64::MAX
    #[inline]
    pub fn get_u64(self) -> Option<u64> {
        let mut res: u64 = 0;
        unsafe {
            match arch::_rdrand64_step(&mut res) {
                1 => Some(res),
                x => {
                    debug_assert_eq!(x, 0, "rdrand64 returned non-binary value");
                    None
                }
            }
        }
    }

    /// Uniformly sampled u32. Returns a random u64 in the range 0..u32::MAX
    #[inline]
    pub fn get_u32(self) -> Option<u32> {
        let mut res: u32 = 0;
        unsafe {
            match arch::_rdrand32_step(&mut res) {
                1 => Some(res),
                x => {
                    debug_assert_eq!(x, 0, "rdrand32 returned non-binary value");
                    None
                }
            }
        }
    }

    /// Uniformly sampled u16. Returns a random u64 in the range 0..u16::MAX
    #[inline]
    pub fn get_u16(self) -> Option<u16> {
        let mut res: u16 = 0;
        unsafe {
            match arch::_rdrand16_step(&mut res) {
                1 => Some(res),
                x => {
                    debug_assert_eq!(x, 0, "rdrand16 returned non-binary value");
                    None
                }
            }
        }
    }
}

impl RdSeed {
    /// Creates the instance of RdSeed if it is supported on hardware.
    #[inline(always)]
    pub fn new() -> Option<Self> {
        let cpuid = unsafe { arch::__cpuid(0x1) };
        if cpuid.ecx & (1 << 30) != 0 {
            Some(Self(()))
        } else {
            None
        }
    }

    /// Generate a random seed in the u64 set.
    #[inline]
    pub fn get_u64_seed(self) -> Option<u64> {
        let mut seed: u64 = 0;
        unsafe {
            match arch::_rdseed64_step(&mut seed) {
                1 => Some(seed),
                x => {
                    debug_assert_eq!(x, 0, "rdseed64 returned non-binary value");
                    None
                }
            }
        }
    }

    /// Generate a random seed in the u32 set.
    #[inline]
    pub fn get_u32_seed(self) -> Option<u32> {
        let mut seed: u32 = 0;
        unsafe {
            match arch::_rdseed32_step(&mut seed) {
                1 => Some(seed),
                x => {
                    debug_assert_eq!(x, 0, "rdseed32 returned non-binary value");
                    None
                }
            }
        }
    }

    /// Generate a random seed in the u16 set.
    #[inline]
    pub fn get_u16_seed(self) -> Option<u16> {
        let mut seed: u16 = 0;
        unsafe {
            match arch::_rdseed16_step(&mut seed) {
                1 => Some(seed),
                x => {
                    debug_assert_eq!(x, 0, "rdseed16 returned non-binary value");
                    None
                }
            }
        }
    }

}