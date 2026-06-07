//! Task Virtualization and Scheduling.

mod thread;
mod process;
mod pmu;

/// Process ID Type.
pub type Pid = u32; 
/// Thread ID Type.
pub type Tid = u32;
/// Priority type both for a thread and a process.
pub type TaskPriority = u32;
