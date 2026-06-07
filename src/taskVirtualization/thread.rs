//! Thread Definitions and Data Types.
//!
//!

use std::fmt::Debug;
use std::boxed::Box;

use super::{Pid, TaskPriority, Tid};

/// Thread Context Trait
///
/// Abstracts context switching behavior from architecture specific code.
pub trait ThreadCtx : Sized + Send + Sync + Debug {
    /// Saves current thread's context.
    ///
    /// ## Architecture Specific
    ///
    /// This usually implies saving different CPU registers within the structure
    /// that this trait implement. This function is always architecture-specific,
    /// since different hardware would require saving different amount/types of
    /// internal registers (state).
    fn save(&mut self);
    /// Restores context from current thread.
    ///
    /// ## Architecture Specific
    ///
    /// This function is always architecture-specific, since registers (state) must
    /// be restored back to hardware, which will differ on different platforms.
    ///
    /// ## Returns
    ///
    /// This function never returns. In fact, this function must force context switching,
    /// which will change PC register value for any platform.
    fn restore(&mut self) -> !;
}

/// Thread's State.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ThreadState {
    /// Initiated, yet never ran. This thread's function must be called for the first time.
    INIT        = 0,
    /// Thread is currently utilizing CPU resources and performing tasks. 
    RUNNING     = 1,
    /// Waiting for dispatch in the ready queue.
    READY       = 2,
    /// Set for wait until manually not switched to [`ThreadState::READY`].
    IDLE        = 3,
    /// Thread has completed executing code, but its context is not cleared.
    FINAL       = 4,
}

/// OS Thread Structure
///
/// Defines a set of parameters to describe a unit of program execution, which
/// preserves state in its context and allows kernel to run, suspend and
/// schedule CPU resources for each.
#[derive(Debug, Clone)]
pub(crate) struct Thread<Ctx> where 
    Ctx : ThreadCtx,
{
    /// Thread's unique ID within the process.
    pub(crate) tid: Tid,
    /// Parent process' ID.
    pub(crate) pid: Pid,
    /// Inner priority of the thread. Only viable in the space within one process.
    pub(crate) priority: TaskPriority,
    /// Thread's current logical state.
    pub(crate) state: ThreadState,

    /// Pointer to thread's saved context.
    ///
    /// [`ThreadCtx`] can be dropped before [`Thread`] is dropped. This way information
    /// about execution will not be missed within the kernel, meanwhile it allows not
    /// holding inner context data inefficiently.
    ctx: Box<Option<Ctx>>,
}

impl<Ctx : ThreadCtx> Thread<Ctx> {
    /// Creates a new instance of [`Thread`].
    ///
    /// **Parameters**
    ///
    /// * `tid`: Thread's ID. Must not repeat within one process.
    /// * `pid`: Parent process' ID.
    /// * `ctx`: Thread's internal context. Must be initialized and prepared beforehand.
    pub fn new(
        pid: Pid,
        tid: Tid,
        priority: TaskPriority,
        ctx: Ctx,
    ) -> Self {
        Self {
            tid, pid, priority,
            state: ThreadState::INIT,
            ctx: Box::new(Some(ctx)), 
        }
    }

    pub fn sleep(&mut self) {
        match self.state {
            ThreadState::IDLE | ThreadState::FINAL => (),
            s @ _ => {
                self.state = ThreadState::IDLE;
                if s == ThreadState::RUNNING { self.r#yield(); }
            },
        }
    }

    /// Yields the control of CPU.
    ///
    /// Since this is a hardware-specific feature, we are using a sophisticated macro
    /// to abstract that behavior. Since yielding is part of cooperative multitasking,
    /// context is being saved right away, before task-switching.
    ///
    /// ## Note
    ///
    /// Scheduler would not save thread's context when yielding is used.
    pub fn r#yield(&mut self) {
        self.ctx.take().map_or_else(
            || panic!("Yielded on thread with no context"), 
            |mut context| context.save()
        );

        todo!("Use task switching macro there!");
    }
}
