//! Commonly used Primitives and Data Structures.
//!
//! Crate defines a large set of type aliases, structures and macros, which
//! are used in the whole project as main building blocks. 

mod lazy;

/// Fault Injection (FI) Prevention Pattern.
///
/// This constant defines the execution anchor used to harden the kernel's critical state
/// machines against physical tampering.
///
/// # Security Mechanics: High Entropy & Bitwise Inversion
///
/// Standard state machines typically rely on sequential integers (`0`, `1`, `2`) or booleans, which
/// are highly vulnerable to **Fault Injection (FI) attacks** (such as voltage glitching or laser pulsing). 
/// Physical glitches statistically bias registers and RAM cells toward uniform states like `0x00` or `0xFF`, 
/// which can accidentally trick a kernel into jumping past locks or assuming initialization is complete.
///
/// # Why It Prevents FI Attacks
///
/// 1. To forge an illegal state transition, an attacker cannot simply drop or raise a voltage line; 
///    By forcing maximum Hamming distance, they would have to flawlessly flip every single alternating 
///    bit across the byte simultaneously (turning all `1`s to `0`s and all `0`s to `1`s).
/// 2. Common glitch outcomes like `0x00` or `0xFF` match neither `0xAA` nor `0x55`. 
///    If an anomaly occurs, the kernel immediately catches the illegal state, aborts the operation, and triggers 
///    a secure system halt rather than entering a compromised state.
/// 3. Alternating Bit Layout `0xAA` represents a perfect alternating sequence of bits (`0b10101010`).
///    Subsequent states in the lifecycle must be configured as the exact 
///    bitwise NOT (~) version of the preceding state (e.g., transitioning from `0xAA` to `0x55` (`0b01010101`)).
pub const FI_PREVENTION_PATTERN: u8 = 0xAA;
