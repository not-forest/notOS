//! notOS Task Virtualization and Scheduling.
//!
//!
#![no_std]
#![allow(non_snake_case)]

extern crate no_std_compat as std;

pub mod thread;
pub mod process;

/// Process ID Type.
pub type Pid = u32; 
/// Thread ID Type.
pub type Tid = u32;
/// Priority type both for a thread and a process.
pub type TaskPriority = u32;
