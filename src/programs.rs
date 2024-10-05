/// This module defines static programs compatible with notOS system.

pub use shell::shell;

/// Basic notOS shell module.
pub mod shell {
    use alloc::sync::Arc;

    use crate::{kernel_components::{keyboard_interface::KeyboardInterface, task_virtualization::Thread}, print};

    /// Small shell program that allows to write commands and receive output. 
    ///
    /// It uses keyboard interface to 
    pub fn shell(t: &mut Thread) {
        use crate::kernel_components::structures::thread_safe::ConcurrentQueue;

        // Creating a keyboard interface to communicate with kernel buffer.
        let k_interface = KeyboardInterface::new();

        // Providing one of the handlers for click event.
        k_interface.on_click(t, |_, str| print!("{}", str.unwrap_or("")));

        loop {}
    }
}
