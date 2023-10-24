/// Protection rings implementation for hierarchial mechanism of protection.
/// 
/// protection rings, are mechanisms to protect data and functionality from faults (by improving
/// fault tolerance) and malicious behavior (by providing computer security). This is generally
/// hardware-enforced by some CPU architectures that provide different CPU modes at the hardware
/// or microcode level. Rings are arranged in a hierarchy from most privileged to the least privileged.

/// ## Privilege level of the protection rings implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PrivilegeLevel {
    /// Privilege level 0 (ring 0) - Kernel level of the hierarchy that is used by
    /// the kernel itself to handle critical tasks that requires the direct access to
    /// the architecture and it's functions. Interrupts, scheduling, memory management
    /// and other related tasks that must be highly protected are within this level.
    KernelLevel = 0,
    /// Privilege level 1 (ring 1) - System level of the hierarchy that provides access to
    /// the kernel functions from the lower level of the system. This level has moderate
    /// privilege and provides access to a more limited scope of computer resources and kernel
    /// functions.
    SystemLevel = 1,
    /// Privilege level 2 (ring 2) - Driver level of the hierarchy that provides access to
    /// periphery devices and provides the support of driver management. Every program that is not
    /// a driver, must fell to this level or higher to modify other drivers or change them.
    /// Reading driver info is supported from the user level.
    DriverLevel = 2,
    /// Privilege level 3 (ring 3) - User level of the hierarchy which is the smallest privilege
    /// level of the system. Most of the software is running at that level and must first
    /// request for a higher level, before doing something with system resources.s
    UserLevel = 3,
}

impl PrivilegeLevel {
    /// Returns a privilege level from a number
    #[inline]
    pub const fn from_u8(num: u8) -> Self {
        match num {
            0 => PrivilegeLevel::KernelLevel,
            1 => PrivilegeLevel::SystemLevel,
            2 => PrivilegeLevel::DriverLevel,
            3 => PrivilegeLevel::UserLevel,
            _ => panic!("The provided privilege level is out of range."),
        }
    }
}