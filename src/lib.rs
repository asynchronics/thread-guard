#![doc = include_str!("../README.md")]
#![warn(missing_docs, missing_debug_implementations, unreachable_pub)]
#![forbid(unsafe_code)]

use std::fmt;
use std::thread::{JoinHandle, Result as ThreadResult};

type PreAction<T, S> = Box<dyn FnOnce(&mut S, &JoinHandle<T>) + Send + 'static>;
type PostAction<T> = Box<dyn FnOnce(ThreadResult<T>) + Send + 'static>;

/// A thread guard.
///
/// A thread guard that automatically joins the thread in the destructor.
///
/// Additionally, custom pre-actions and post-actions can be defined to execute
/// before and after thread joining, respectively. The thread can also be
/// explicitly joined using the `join` method. In this case, the pre-action is
/// executed before the join, and the thread result is returned to the caller.
pub struct ThreadGuard<T, S = ()> {
    /// The thread handle.
    handle: Option<JoinHandle<T>>,

    /// Data used by the pre-action.
    pre_action_data: S,

    /// An action called before the thread join.
    pre_action: Option<PreAction<T, S>>,

    /// An action processing the thread result executed on drop.
    post_action: Option<PostAction<T>>,
}

impl<T> ThreadGuard<T> {
    /// Creates a new `ThreadGuard`.
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
            pre_action_data: (),
            pre_action: None,
            post_action: None,
        }
    }
}

impl<T, S> ThreadGuard<T, S> {
    /// Creates a new `ThreadGuard` with the specified pre-action and
    /// post-action.
    pub fn with_actions<F, G>(
        handle: JoinHandle<T>,
        pre_action_data: S,
        pre_action: F,
        post_action: G,
    ) -> Self
    where
        for<'a> F: FnOnce(&mut S, &JoinHandle<T>) + Send + 'a,
        for<'a> G: FnOnce(ThreadResult<T>) + Send + 'a,
    {
        Self {
            handle: Some(handle),
            pre_action_data,
            pre_action: Some(Box::new(pre_action)),
            post_action: Some(Box::new(post_action)),
        }
    }

    /// Joins the guarded thread.
    pub fn join(mut self) -> ThreadResult<T> {
        // Shall never be `None`.
        let handle = self.handle.take().unwrap();
        if let Some(action) = self.pre_action.take() {
            action(&mut self.pre_action_data, &handle);
        }
        handle.join()
    }
}

impl<T, S> Drop for ThreadGuard<T, S> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            if let Some(action) = self.pre_action.take() {
                action(&mut self.pre_action_data, &handle);
            }
            let result = handle.join();
            if let Some(action) = self.post_action.take() {
                action(result);
            }
        }
    }
}

impl<T, S> fmt::Debug for ThreadGuard<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ThreadGuard").finish_non_exhaustive()
    }
}
