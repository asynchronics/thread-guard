#![doc = include_str!("../README.md")]
#![warn(missing_docs, missing_debug_implementations, unreachable_pub)]
#![forbid(unsafe_code)]

use std::any::Any;
use std::fmt;
use std::thread::{JoinHandle, Result as ThreadResult};

/// A thread guard.
///
/// A thread guard that automatically joins the thread in the destructor.
///
/// Additionally, custom pre-actions and post-actions can be defined to execute
/// before and after thread joining, respectively. The thread can also be
/// explicitly joined using the `join` method. In this case, the pre-action is
/// executed before the join, and the thread result is returned to the caller.
pub struct ThreadGuard<T> {
    drop_action: Option<Box<dyn FnOnce(bool) -> ThreadResult<T> + Send + 'static>>,
}

impl<T: 'static> ThreadGuard<T> {
    /// Creates a new `ThreadGuard`.
    pub fn new(handle: JoinHandle<T>) -> Self {
        let drop_action = Box::new(move |_run_post_action| handle.join());

        Self {
            drop_action: Some(drop_action),
        }
    }

    /// Creates a new `ThreadGuard` with the specified pre-action and
    /// post-action.
    pub fn with_actions<S, F, G>(
        handle: JoinHandle<T>,
        mut pre_action_data: S,
        pre_action: F,
        post_action: G,
    ) -> Self
    where
        S: Send + 'static,
        for<'a> F: FnOnce(&mut S, &JoinHandle<T>) + Send + 'a,
        for<'a> G: FnOnce(ThreadResult<T>) + Send + 'a,
    {
        let drop_action = Box::new(move |run_post_action| {
            pre_action(&mut pre_action_data, &handle);
            let result = handle.join();
            if run_post_action {
                post_action(result);

                return Err(Box::new(()) as Box<dyn Any + Send>);
            }

            result
        });

        Self {
            drop_action: Some(drop_action),
        }
    }

    /// Joins the guarded thread.
    pub fn join(mut self) -> ThreadResult<T> {
        self.drop_action.take().unwrap()(false)
    }
}

impl<T> Drop for ThreadGuard<T> {
    fn drop(&mut self) {
        let _ = self.drop_action.take().unwrap()(true);
    }
}

impl<T> fmt::Debug for ThreadGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ThreadGuard").finish_non_exhaustive()
    }
}
