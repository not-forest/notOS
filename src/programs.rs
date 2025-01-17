/// This module defines static programs compatible with notOS system.

pub use shell::shell;

/// Basic notOS shell module.
pub mod shell {
    use alloc::vec::Vec;
    use alloc::{sync::Arc, string::String};

    use crate::{debug, print, println, Color};
    use crate::kernel_components::{
        arch_x86_64::interrupts::interrupt, 
        keyboard_interface::KeyboardInterface, 
        task_virtualization::Thread,
        drivers::keyboards::Key,
        sync::Mutex
    };

    fn shell_greetings() {
        println!(Color::GREEN; concat!("\n\n\n", 
                                       "# Welcome to \"notOS\" shell.\n",
                                       "# This software is an internal part of notOS kernel.\n",
                                       "#\n",
                                       "# THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED", 
                                       "INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR", 
                                       "PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE",
                                       "FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,",
                                       "ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.",
                                       "\n\n\n"))
    }

    /// Small shell program that allows to write commands and receive output. 
    ///
    /// Keyboard interface is being used to communicate with kernel and read data obtained from
    /// user's keyboard.
    pub fn shell(t: &mut Thread) {
        // Creating a keyboard interface to communicate with kernel buffer.
        let mut k_interface = KeyboardInterface::new();
        let mut shell_ptr = 0u8;
        let mut shell_buf = Arc::new(Mutex::new(Vec::<char>::with_capacity(256))); // TODO! Swap for user variable.

        shell_greetings();
        let shell_arc = shell_buf.clone();
        // Providing one of the handlers for click event.
        k_interface.on_click(t, move |_, c| c.map(|c| {
            let shell_buf = shell_arc.clone();
            match c.key.key {
                Key::Backspace => if c.key.is_pressed() { shell_buf.lock().push('\x7e') }, // Forces to clean one char
                _ => {
                    match c.chr {
                        Some(chr) => shell_buf.lock().push(chr),
                        None => ()
                    }
                }
            }
        }));

        loop {
            while shell_ptr < shell_buf.lock().len() as u8 {
                shell_buf.lock()
                    .last()
                    .map(|c| print!("{}", c));
                shell_ptr += 1;
            }

            interrupt::wait_for_interrupt();
        }
    }
}
