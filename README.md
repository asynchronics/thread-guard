# Thread guard

This crate contains a simple thread guard.

The thread guard is used to ensure that the thread in question is always
joined.

Pre-action and post-action can be defined to execute before and after
thread joining, respectively. The pre-action is typically used to send a
message to the thread, triggering its exit, while the post-action handles
the thread's result.

In the simplest case, the guard can be used without pre-action and
post-action:

```rust
use std::thread;
use thread_guard::ThreadGuard;

let guard = ThreadGuard::new(
    thread::spawn(|| {
        // Do something and exit.
    }));
```

The post-action can be used to handle the thread result:

```rust
use std::thread;
use thread_guard::ThreadGuard;

let guard = ThreadGuard::with_post_action(
    thread::spawn(|| {
        1
    }),
    |r| {println!("The thread exited with the result {:?}", r)});
```

If the thread needs to be signaled to exit, a pre-action can be used:

```rust
use std::sync::mpsc::{channel, RecvError};
use std::thread;
use thread_guard::ThreadGuard;

let (tx, rx) = channel();
let guard = ThreadGuard::with_pre_action(
    thread::spawn(move || -> Result<(), RecvError>{
        rx.recv()?;
        Ok(())
    }),
    move |_| {let _ = tx.send(());});
```

Finally, a pre-action may need some data that outlives the appropriate
closure. An example is provided in the `examples` directory.
