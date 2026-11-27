//! `AbstractHydrator::toIterable()`: PHP's generator as a lazy iterable.
//! Nothing runs before the first `next()`: then the hydrator registers
//! itself as an `onClear` listener of the event manager and prepares; each
//! `next()` hydrates the rows up to the next yielded value; the end of the
//! rows runs the cleanup. Dropping the iterable before its end runs the
//! cleanup as the generator's `finally` does, without the deferred eager
//! loads of the object hydrators' cleanup (they would wait on the
//! database, which a drop cannot do).
//!
//! The generator is an async block that parks itself at each `yield`; the
//! value goes through a slot the iterable reads.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: doctrine/orm src/Internal/Hydration/AbstractHydrator.php

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, ListenerError,
};

use super::abstract_hydrator::{Columns, HYDRATION_EXCEPTION};
use super::array_hydrator::ArrayHydrator;
use super::object_hydrator::ObjectHydrator;
use super::{HydrationHints, hydrate_all};
use crate::entity_manager::EntityManager;
use crate::events;
use crate::query::ResultSetMapping;
use crate::unit_of_work::Hints;

/// The generator body: runs the hydration up to its end.
type Generator = Pin<Box<dyn Future<Output = Result<(), Thrown>> + Send>>;

type Slot = Arc<Mutex<Option<(Key, Value)>>>;

/// The hydrator as the `onClear` listener: it notes the event for the
/// hydrator, which resets its state before the next row.
#[derive(Default)]
struct OnClearListener {
    cleared: AtomicBool,
}

impl EventListener for OnClearListener {
    fn call(&self, event_name: &str, _event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        if event_name == events::ON_CLEAR {
            self.cleared.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}

/// `cleanup()`'s part that does not wait: the listener is removed (and,
/// for the object hydrators, `hydrationComplete()` runs) also when the
/// generator is dropped before its end.
struct CleanupGuard {
    em: Arc<EntityManager>,
    listener: Arc<dyn EventListener>,
    hydration_complete: bool,
    done: bool,
}

impl CleanupGuard {
    fn remove_listener(&self) {
        self.em.with_event_manager(|evm| {
            evm.remove_event_listener(&[events::ON_CLEAR], &self.listener);
        });
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        self.remove_listener();
        if self.hydration_complete {
            // A drop cannot report what a postLoad listener throws.
            let _ = self.em.get_unit_of_work().hydration_complete();
        }
    }
}

/// The future of a `yield`: pending once, ready when resumed.
struct Yield(bool);

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}

/// The generator's `yield`s: the keys of `yield from $result`, the
/// generator's own integer keys (from 0) for the others.
struct Yielder {
    slot: Slot,
    index_by: bool,
    auto_key: i64,
}

impl Yielder {
    async fn emit(&self, key: Key, value: Value) {
        *self.slot.lock().unwrap_or_else(PoisonError::into_inner) = Some((key, value));
        Yield(false).await;
    }

    fn next_key(&mut self) -> Key {
        let key = Key::Int(self.auto_key);
        self.auto_key += 1;
        key
    }

    /// The `yield`s of a row's `$result`.
    async fn yield_row(&mut self, result: PhpArray) {
        if result.len() == 1 {
            if self.index_by {
                // yield from $result
                for (key, value) in result.iter() {
                    self.emit(key.clone(), value.clone()).await;
                }
            } else if let Some((_, last)) = result.last() {
                // yield end($result)
                let key = self.next_key();
                self.emit(key, last.clone()).await;
            }
        } else if matches!(result.values().next(), Some(Value::Object(_))) {
            let key = self.next_key();
            self.emit(key, Value::Array(result)).await;
        } else {
            // array_merge(...$result)
            let mut merged = PhpArray::new();
            for value in result.values() {
                if let Value::Array(row) = value {
                    for (k, v) in row.iter() {
                        match k {
                            Key::Int(_) => {
                                let _ = merged.push(v.clone());
                            }
                            Key::Str(_) => merged.insert(k.clone(), v.clone()),
                        }
                    }
                }
            }
            let key = self.next_key();
            self.emit(key, Value::Array(merged)).await;
        }
    }
}

/// The generator `toIterable()` returns.
pub struct Iterable {
    slot: Slot,
    generator: Option<Generator>,
}

impl std::fmt::Debug for Iterable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Iterable")
            .field("finished", &self.generator.is_none())
            .finish_non_exhaustive()
    }
}

impl Iterable {
    /// An iterable that yields `values` (a `COUNT` statement's result).
    pub(crate) fn of(values: Vec<(Key, Value)>) -> Self {
        let slot: Slot = Arc::default();
        let yielder = slot.clone();
        Self {
            slot,
            generator: Some(Box::pin(async move {
                let yielder = Yielder {
                    slot: yielder,
                    index_by: false,
                    auto_key: 0,
                };
                for (key, value) in values {
                    yielder.emit(key, value).await;
                }
                Ok(())
            })),
        }
    }

    /// `$em->newHydrator($hydrationMode)->toIterable($stmt, $rsm,
    /// $hints)` over the statement's rows.
    pub(crate) fn new(
        em: Arc<EntityManager>,
        hydration_mode: i64,
        rows: Vec<PhpArray>,
        rsm: ResultSetMapping,
        hints: Hints,
        extra: HydrationHints,
    ) -> Self {
        let slot: Slot = Arc::default();
        let generator = generate(em, hydration_mode, rows, rsm, hints, extra, slot.clone());
        Self {
            slot,
            generator: Some(Box::pin(generator)),
        }
    }

    /// The next yielded value with its key; `None` at the end.
    pub async fn next_with_key(&mut self) -> Option<Result<(Key, Value), Thrown>> {
        std::future::poll_fn(|cx| {
            let Some(generator) = self.generator.as_mut() else {
                return Poll::Ready(None);
            };
            match generator.as_mut().poll(cx) {
                Poll::Ready(Ok(())) => {
                    self.generator = None;
                    Poll::Ready(None)
                }
                Poll::Ready(Err(e)) => {
                    self.generator = None;
                    Poll::Ready(Some(Err(e)))
                }
                Poll::Pending => {
                    let yielded = self
                        .slot
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .take();
                    match yielded {
                        Some(yielded) => Poll::Ready(Some(Ok(yielded))),
                        None => Poll::Pending,
                    }
                }
            }
        })
        .await
    }

    /// The next yielded value; `None` at the end.
    #[allow(
        clippy::should_implement_trait,
        reason = "an async next(), as a generator's"
    )]
    pub async fn next(&mut self) -> Option<Result<Value, Thrown>> {
        self.next_with_key()
            .await
            .map(|yielded| yielded.map(|(_, value)| value))
    }

    /// `iterator_to_array($iterable, false)`.
    pub async fn to_vec(mut self) -> Result<Vec<Value>, Thrown> {
        let mut values = Vec::new();
        while let Some(value) = self.next().await {
            values.push(value?);
        }
        Ok(values)
    }

    /// The yielded values with their keys, to the end.
    pub async fn to_vec_with_keys(mut self) -> Result<Vec<(Key, Value)>, Thrown> {
        let mut values = Vec::new();
        while let Some(value) = self.next_with_key().await {
            values.push(value?);
        }
        Ok(values)
    }

    /// `iterator_to_array($iterable, true)`.
    pub async fn to_array(self) -> Result<PhpArray, Thrown> {
        let mut array = PhpArray::new();
        for (key, value) in self.to_vec_with_keys().await? {
            array.insert(key, value);
        }
        Ok(array)
    }
}

/// The generator's body.
#[allow(
    clippy::too_many_lines,
    reason = "toIterable() with each hydrator's hydrateRowData()"
)]
async fn generate(
    em: Arc<EntityManager>,
    hydration_mode: i64,
    rows: Vec<PhpArray>,
    rsm: ResultSetMapping,
    mut hints: Hints,
    extra: HydrationHints,
    slot: Slot,
) -> Result<(), Thrown> {
    use crate::query::context::{
        HYDRATE_ARRAY, HYDRATE_OBJECT, HYDRATE_SCALAR, HYDRATE_SIMPLEOBJECT,
    };

    let mut yielder = Yielder {
        slot,
        index_by: !rsm.index_by_map.is_empty(),
        auto_key: 0,
    };

    // $evm->addEventListener([Events::onClear], $this)
    let on_clear = Arc::new(OnClearListener::default());
    let listener: Arc<dyn EventListener> = on_clear.clone();
    em.with_event_manager(|evm| evm.add_event_listener(&[events::ON_CLEAR], listener.clone()));
    let mut guard = CleanupGuard {
        em: em.clone(),
        listener,
        hydration_complete: matches!(hydration_mode, HYDRATE_OBJECT | HYDRATE_SIMPLEOBJECT),
        done: false,
    };

    match hydration_mode {
        HYDRATE_OBJECT => {
            let mut hydrator = ObjectHydrator::new(
                &em,
                &rsm,
                &mut hints,
                extra.defer_eager_load,
                extra.collection,
            );
            if let Err(e) = hydrator.prepare() {
                // prepare() runs before the generator's try: no cleanup.
                guard.done = true;
                return Err(e);
            }
            let mut outcome = Ok(());
            for row in rows {
                if on_clear.cleared.swap(false, Ordering::SeqCst) {
                    hydrator.on_clear();
                }
                let mut result = PhpArray::new();
                if let Err(e) = hydrator.hydrate_row_data(&row, &mut result).await {
                    outcome = Err(e);
                    break;
                }
                hydrator.cleanup_after_row_iteration();
                yielder.yield_row(result).await;
            }
            // finally { $this->cleanup(); }
            guard.remove_listener();
            guard.done = true;
            let cleanup = hydrator.cleanup().await;
            outcome?;
            cleanup?;
        }
        HYDRATE_ARRAY => {
            let mut hydrator = ArrayHydrator::new(&em, &rsm, hints.force_dbal_type_conversion);
            hydrator.prepare();
            for row in &rows {
                let mut result = PhpArray::new();
                hydrator.hydrate_row_data(row, &mut result)?;
                yielder.yield_row(result).await;
            }
        }
        HYDRATE_SCALAR => {
            let mut columns = Columns::new(&em, &rsm, hints.force_dbal_type_conversion);
            for row in &rows {
                let mut result = PhpArray::new();
                let _ = result.push(Value::Array(columns.gather_scalar_row_data(row)?));
                yielder.yield_row(result).await;
            }
        }
        HYDRATE_SIMPLEOBJECT => {
            for row in rows {
                let entities = hydrate_all(&em, vec![row], &rsm, &mut hints, None).await?;
                let mut result = PhpArray::new();
                for entity in entities {
                    let _ = result.push(Value::Object(entity));
                }
                yielder.yield_row(result).await;
            }
        }
        _ => {
            return Err(Thrown::new(
                HYDRATION_EXCEPTION,
                "hydrateRowData() not implemented by this hydrator.",
                0,
            ));
        }
    }
    drop(guard);
    Ok(())
}
