/// This module defines static programs compatible with notOS system.

pub use shell::shell;

/// Basic notOS shell module.
pub mod shell {
    use alloc::sync::Arc;

    use crate::{kernel_components::{keyboard_interface::KeyboardInterface, task_virtualization::Thread}, print};

    /// Small shell program that allows to write commands and receive output. 
    ///
    /// Keyboard interface is being used to communicate with kernel and read data obtained from
    /// user's keyboard.
    pub fn shell(t: &mut Thread) {
        // Creating a keyboard interface to communicate with kernel buffer.
        let mut k_interface = KeyboardInterface::new();

        // Providing one of the handlers for click event.
        k_interface.on_click(t, |_, c| c.map(|c| print!("{}", c)));

        loop {}
    }
}
