/// Instruction for manipulating with Transition Lookaside Buffer.
 
use crate::VirtualAddress;
use core::arch::asm;

/// Process-Context Identifier implementation structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pcid(u16);

impl Pcid {
    /// Creates a new PCID. The bounds must be smaller than 4096.
    pub const fn new(pcid: u16) -> Result<Pcid, &'static str> {
        if pcid < 4096 {
            Ok(Pcid(pcid))
        } else {
            Err("The PCID bounds are not satisfied. Expected pcid < 4096.")
        }
    }

    /// Get the nested value
    pub const fn get(&self) -> u16 {
        self.0
    }
}

/// PCID commands to execute.
#[derive(Debug)]
pub enum PcidCommand {
    /// The logical processor invalidates mappings—except global translations—for the linear address and PCID specified.
    Address(VirtualAddress, Pcid),

    /// The logical processor invalidates all mappings—except global translations—associated with the PCID.
    Single(Pcid),

    /// The logical processor invalidates all mappings—including global translations—associated with any PCID.
    All,

    /// The logical processor invalidates all mappings—except global translations—associated with any PCID.
    AllExceptGlobal,
}

/// Flushing the given address in the TLB via 'invlpg' asm instruction.
#[inline]
pub fn flush(addr: VirtualAddress) {
    unsafe {
        asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Invalidate the TLB completely. This function reloads the CR3 register.
#[inline]
pub fn flush_all() {
    use crate::kernel_components::registers::control::Cr3;
    let (frame, flags) = Cr3::read();
    unsafe {
        Cr3::write(frame, flags)
    }
}

/// Invalidate the TLB entries associated with a specific process of context.
/// 
/// It is designed to improve TLB invalidation efficiency in situations where 
/// multiple address spaces (processes or contexts) are in use simultaneously.
#[inline]
pub unsafe fn flush_pcid(command: PcidCommand) {
    use PcidCommand::*;
    let kind: u64;
    let (mut addr, mut pcid) = (0, 0);

    match command {
        Address(address, pc_id) => {
            kind = 0;
            addr = address as u64;
            pcid = pc_id.get();
        },

        Single(pc_id) => {
            kind = 1;
            pcid = pc_id.get();
        },
    
        All =>             kind = 2,
        AllExceptGlobal => kind = 3,
    }

    unsafe {
        asm!("invpcid {0}, [{1}]", in(reg) kind, in(reg) &(addr, pcid), options(nostack, preserves_flags));
    }
}