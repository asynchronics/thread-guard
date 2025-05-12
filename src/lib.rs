#![doc = include_str!("../README.md")]
#![warn(missing_docs, missing_debug_implementations, unreachable_pub)]
#![forbid(unsafe_code)]

use std::any::Any;
use std::fmt;
use std::mem;
use std::thread::{JoinHandle, Result as ThreadResult};

/// A thread guard.
///
/// A thread guard that automatically joins the thread in the destructor.
///
/// Additionally, custom pre-actions and post-actions can be defined to execute
/// before and after thread joining, respectively. The thread can also be
/// explicitly joined using the `join` method. In this case, the pre-action is
/// executed before the join, and the thread result is returned to the caller.
pub struct ThreadGuard<T>(
    Option<(
        JoinHandle<T>,
        Box<dyn FnOnce(bool, JoinHandle<T>) -> ThreadResult<T> + Send>,
    )>,
);

impl<T> ThreadGuard<T> {
    /// Creates a new `ThreadGuard`.
    pub fn new(handle: JoinHandle<T>) -> Self {
        let action =
            Box::new(move |_run_post_action, join_handle: JoinHandle<T>| join_handle.join());

        Self(Some((handle, action)))
    }

    /// Creates a new `ThreadGuard` with the specified pre-action.
    pub fn with_pre_action<U, F>(handle: JoinHandle<T>, pre_action: F) -> Self
    where
        for<'a> F: FnOnce(&JoinHandle<T>) -> U + Send + 'a,
    {
        Self::with_actions(handle, pre_action, |_, _| {})
    }

    /// Creates a new `ThreadGuard` with the specified post-action.
    pub fn with_post_action<F>(handle: JoinHandle<T>, post_action: F) -> Self
    where
        for<'a> F: FnOnce(ThreadResult<T>) + Send + 'a,
    {
        Self::with_actions(handle, |_| {}, |_, result| post_action(result))
    }

    /// Creates a new `ThreadGuard` with the specified pre-action and
    /// post-action.
    pub fn with_actions<U, F, G>(handle: JoinHandle<T>, pre_action: F, post_action: G) -> Self
    where
        for<'a> F: FnOnce(&JoinHandle<T>) -> U + Send + 'a,
        for<'a> G: FnOnce(U, ThreadResult<T>) + Send + 'a,
    {
        let action = Box::new(move |run_post_action, join_handle| {
            let arg = pre_action(&join_handle);
            let result = join_handle.join();
            if run_post_action {
                post_action(arg, result);

                // This is a bit ugly. Another possibility would be for `action`
                // to return an `Option<ThreadResult<T>>` but then we will have
                // yet another infallible `unwrap`.
                return Err(Box::new(()) as Box<dyn Any + Send>);
            }

            result
        });

        Self(Some((handle, action)))
    }

    /// Joins the guarded thread.
    pub fn join(mut self) -> ThreadResult<T> {
        let (handle, action) = self.0.take().unwrap();
        mem::forget(self);

        action(false, handle)
    }
}

impl<T> Drop for ThreadGuard<T> {
    fn drop(&mut self) {
        let (handle, action) = self.0.take().unwrap();
        let _ = action(true, handle);
    }
}

impl<T> fmt::Debug for ThreadGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ThreadGuard").finish_non_exhaustive()
    }
}
