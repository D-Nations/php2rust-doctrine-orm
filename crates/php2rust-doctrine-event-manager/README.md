# Doctrine Event Manager

Private workspace crate: a port of `doctrine/event-manager` 2.1.1
(`dda33921`). MIT; Doctrine source and tests copyright the Doctrine
Project. Unofficial, not affiliated with or endorsed by Doctrine.

```rust
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use php2rust_doctrine_event_manager::{
    EventArgs, EventDispatcher, EventListener, EventListenerRegistry, EventManager,
    ListenerError, undefined_method,
};

struct PostFlushArgs {
    count: usize,
}
impl EventArgs for PostFlushArgs {}

#[derive(Default)]
struct Counter(AtomicUsize);

impl EventListener for Counter {
    fn call(&self, event: &str, args: &dyn EventArgs) -> Result<(), ListenerError> {
        match event {
            "postFlush" => {
                let args = (args as &dyn Any).downcast_ref::<PostFlushArgs>();
                self.0.fetch_add(args.map_or(0, |a| a.count), Ordering::SeqCst);
                Ok(())
            }
            _ => Err(undefined_method("Counter", event)),
        }
    }
}

let counter = Arc::new(Counter::default());
let mut evm = EventManager::new();
evm.add_event_listener(&["postFlush"], counter.clone());
evm.dispatch_event("postFlush", Some(&PostFlushArgs { count: 3 })).unwrap();
assert_eq!(3, counter.0.load(Ordering::SeqCst));
```

| Contract | Source | Evidence |
|---|---|---|
| `EventManager`: listeners per event keyed by `ListenerId` (`spl_object_id()`: the `Arc`'s object); adding the same object again keeps its place, removing it and adding it again puts it last; an event keeps its place in `getAllListeners()` once its listeners are removed; `hasListeners()` is false for it | `EventManager.php`, `EventListenerRegistry.php`, `EventListenerIntrospector.php` | EventManagerTest (8); recorded scenario (`tests/data/event_manager.json`, oracle/recorders/event_manager.php, `tests/vectors.rs`) |
| `dispatchEvent()`: nothing for an event never registered (the empty `EventArgs` is not created); otherwise each listener's method named after the event (`EventListener::call()`) in order, with the given arguments or the single empty instance; a listener's error ends the dispatch and is returned; `undefined_method()` is PHP's `Error` text | `EventManager.php`, `EventDispatcher.php`, `EventArgs.php` | EventManagerTest; recorded scenario |
| `addEventSubscriber()` / `removeEventSubscriber()` with `getSubscribedEvents()` | `EventManager.php`, `EventSubscriber.php`, `EventSubscriberRegistry.php` | EventManagerTest |
| The interfaces as traits; `EventManagerInterface` combines them | `EventManagerInterface.php` and the above | Compiled use |

Not reproduced, and so not claimed:

- PHP turns a numeric event name such as `"12"` into an int array key;
  `getAllListeners()` gives every name as a string.
- The single empty `EventArgs` instance is one per thread.
- Listeners are called on a copy of the event's list, as PHP's `foreach`
  does; the manager is not locked while they run.
