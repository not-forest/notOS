/// notOS Process.
///
/// This is abstraction over jobs/tasks done in the OS.

use crate::{Pid, TaskPriority};

#[derive(Debug, Clone)]
pub struct Process {
    pid: Pid,
    priority: TaskPriority,
}

impl Process {

}
