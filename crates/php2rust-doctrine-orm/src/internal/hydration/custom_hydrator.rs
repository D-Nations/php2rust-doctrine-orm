//! `AbstractHydrator` as a base for hydrators of the application: the
//! custom hydration modes (`Configuration::addCustomHydrationMode()`) and
//! `EntityManager::newHydrator()` for them. A hydrator is an
//! [`AbstractHydrator`] implementation, registered under its class name
//! with [`register_hydrator_class`] (Rust has no classes to instantiate by
//! name). [`hydrate_all`] and [`to_iterable`] are upstream's final
//! methods: they register the hydrator as an `onClear` listener of the
//! entity manager's event manager while it hydrates, and remove it at the
//! cleanup.
//!
//! The listener calls the hydrator's `onClear()` when the hydrator next
//! resumes (in `toIterable()`, before the next row): the hydrator is
//! borrowed by its caller while it runs. The built-in hydrators are not
//! [`AbstractHydrator`] implementations: the queries hydrate with
//! `hydration::hydrate()`.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: doctrine/orm src/Internal/Hydration/AbstractHydrator.php

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::Result as DbalResult;
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, ListenerError,
};

use super::abstract_hydrator::hydration_exception;
use crate::entity_manager::EntityManager;
use crate::events;
use crate::query::ResultSetMapping;

/// What a hydrator works on while it hydrates: `$this->em`,
/// `$this->statement()`, `$this->resultSetMapping()` and `$this->hints`.
pub struct HydrationContext<'s, 'c> {
    pub em: Arc<EntityManager>,
    stmt: &'s mut DbalResult<'c>,
    rsm: &'s ResultSetMapping,
    pub hints: PhpArray,
}

impl<'c> HydrationContext<'_, 'c> {
    /// `statement()`.
    pub fn statement(&mut self) -> &mut DbalResult<'c> {
        self.stmt
    }

    /// `resultSetMapping()`.
    pub fn result_set_mapping(&self) -> &ResultSetMapping {
        self.rsm
    }
}

/// `Doctrine\ORM\Internal\Hydration\AbstractHydrator`'s overridable part.
pub trait AbstractHydrator: Send {
    /// `$hydrator::class`.
    fn class(&self) -> &str;

    /// Executes one-time preparation tasks, once each time hydration is
    /// started through [`hydrate_all`] or [`to_iterable`].
    fn prepare(&mut self, _context: &mut HydrationContext<'_, '_>) -> Result<(), Thrown> {
        Ok(())
    }

    /// Hydrates all rows from the current statement instance at once.
    fn hydrate_all_data(&mut self, context: &mut HydrationContext<'_, '_>)
    -> Result<Value, Thrown>;

    /// Hydrates a single row from the current statement instance.
    fn hydrate_row_data(
        &mut self,
        _context: &mut HydrationContext<'_, '_>,
        _row: &PhpArray,
        _result: &mut PhpArray,
    ) -> Result<(), Thrown> {
        Err(hydration_exception(
            "hydrateRowData() not implemented by this hydrator.".to_owned(),
        ))
    }

    /// Executes one-time cleanup tasks at the end of a row iteration.
    fn cleanup_after_row_iteration(&mut self) {}

    /// When executed in a `hydrate()` loop we have to clear internal state to
    /// decrease memory consumption.
    fn on_clear(&mut self) {}
}

/// `new $class($em)` for a hydrator class.
pub type HydratorConstructor =
    Arc<dyn Fn(&Arc<EntityManager>) -> Box<dyn AbstractHydrator> + Send + Sync>;

fn hydrator_classes() -> &'static RwLock<HashMap<String, HydratorConstructor>> {
    static CLASSES: OnceLock<RwLock<HashMap<String, HydratorConstructor>>> = OnceLock::new();
    CLASSES.get_or_init(Default::default)
}

/// Registers the constructor of a hydrator class, for the custom
/// hydration modes that name it.
pub fn register_hydrator_class(class_name: &str, constructor: HydratorConstructor) {
    hydrator_classes()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(class_name.to_owned(), constructor);
}

/// `new $class($em)`: PHP's `Error` for a class that is not registered.
pub(crate) fn new_hydrator_of_class(
    class_name: &str,
    em: &Arc<EntityManager>,
) -> Result<Box<dyn AbstractHydrator>, Thrown> {
    let constructor = hydrator_classes()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(class_name)
        .cloned();
    match constructor {
        Some(constructor) => Ok(constructor(em)),
        None => Err(Thrown::new(
            "Error",
            format!("Class \"{class_name}\" not found"),
            0,
        )),
    }
}

/// The hydrator as the `onClear` listener: it notes the event for the
/// hydrator.
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

fn add_listener(em: &EntityManager) -> Arc<OnClearListener> {
    let listener = Arc::new(OnClearListener::default());
    let as_listener: Arc<dyn EventListener> = listener.clone();
    em.with_event_manager(|evm| evm.add_event_listener(&[events::ON_CLEAR], as_listener));
    listener
}

/// `cleanup()`: frees the statement and removes the listener.
fn cleanup(context: &mut HydrationContext<'_, '_>, listener: &Arc<OnClearListener>) {
    context.stmt.free();
    let as_listener: Arc<dyn EventListener> = listener.clone();
    context
        .em
        .with_event_manager(|evm| evm.remove_event_listener(&[events::ON_CLEAR], &as_listener));
}

/// `$hydrator->hydrateAll($stmt, $resultSetMapping, $hints)`: hydrates
/// all rows returned by the passed statement instance at once.
pub fn hydrate_all(
    hydrator: &mut dyn AbstractHydrator,
    em: &Arc<EntityManager>,
    stmt: &mut DbalResult<'_>,
    result_set_mapping: &ResultSetMapping,
    hints: PhpArray,
) -> Result<Value, Thrown> {
    let listener = add_listener(em);
    let mut context = HydrationContext {
        em: em.clone(),
        stmt,
        rsm: result_set_mapping,
        hints,
    };
    hydrator.prepare(&mut context)?;

    let result = hydrator.hydrate_all_data(&mut context);
    cleanup(&mut context, &listener);

    result
}

/// `$hydrator->toIterable($stmt, $resultSetMapping, $hints)`: initiates a
/// row-by-row hydration, as PHP's generator does: nothing runs before the
/// first `next()`, and a drop before the end cleans up as the generator's
/// `finally` does.
pub fn to_iterable<'h, 's, 'c>(
    hydrator: &'h mut dyn AbstractHydrator,
    em: &Arc<EntityManager>,
    stmt: &'s mut DbalResult<'c>,
    result_set_mapping: &'s ResultSetMapping,
    hints: PhpArray,
) -> ToIterable<'h, 's, 'c> {
    ToIterable {
        hydrator,
        context: HydrationContext {
            em: em.clone(),
            stmt,
            rsm: result_set_mapping,
            hints,
        },
        listener: None,
        finished: false,
        pending: VecDeque::new(),
    }
}

/// The generator of [`to_iterable`].
pub struct ToIterable<'h, 's, 'c> {
    hydrator: &'h mut dyn AbstractHydrator,
    context: HydrationContext<'s, 'c>,
    /// Set once the generator started.
    listener: Option<Arc<OnClearListener>>,
    finished: bool,
    /// `yield from $result`'s remaining values.
    pending: VecDeque<Value>,
}

impl ToIterable<'_, '_, '_> {
    fn finish(&mut self) {
        if !self.finished {
            self.finished = true;
            if let Some(listener) = &self.listener {
                cleanup(&mut self.context, listener);
            }
        }
    }

    fn step(&mut self) -> Result<Option<Value>, Thrown> {
        if self.listener.is_none() {
            self.listener = Some(add_listener(&self.context.em));
            // prepare() runs before the generator's try: no cleanup
            if let Err(e) = self.hydrator.prepare(&mut self.context) {
                self.finished = true;
                return Err(e);
            }
        }
        if let Some(listener) = &self.listener
            && listener.cleared.swap(false, Ordering::SeqCst)
        {
            self.hydrator.on_clear();
        }
        loop {
            if let Some(value) = self.pending.pop_front() {
                return Ok(Some(value));
            }
            let row = self
                .context
                .stmt
                .fetch_associative()
                .map_err(|e| crate::exception::from_dbal(&e))?;

            let Some(row) = row else {
                return Ok(None);
            };

            let mut result = PhpArray::new();

            self.hydrator
                .hydrate_row_data(&mut self.context, &row, &mut result)?;

            self.hydrator.cleanup_after_row_iteration();
            if result.len() == 1 {
                if self.context.rsm.index_by_map.is_empty() {
                    // yield end($result)
                    return Ok(result.iter().last().map(|(_, value)| value.clone()));
                }
                // yield from $result
                self.pending
                    .extend(result.iter().map(|(_, value)| value.clone()));
            } else if matches!(result.iter().next(), Some((_, Value::Object(_)))) {
                return Ok(Some(Value::Array(result)));
            } else {
                // array_merge(...$result)
                let mut merged = PhpArray::new();
                for (_, part) in result.iter() {
                    if let Value::Array(part) = part {
                        for (key, value) in part.iter() {
                            match key {
                                Key::Int(_) => {
                                    merged.push(value.clone()).map_err(|_| {
                                        Thrown::new("Error", "Cannot add element to the array", 0)
                                    })?;
                                }
                                Key::Str(_) => {
                                    merged.insert(key.clone(), value.clone());
                                }
                            }
                        }
                    }
                }
                return Ok(Some(Value::Array(merged)));
            }
        }
    }
}

impl Iterator for ToIterable<'_, '_, '_> {
    type Item = Result<Value, Thrown>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.step() {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => {
                self.finish();
                None
            }
            Err(e) => {
                self.finish();
                Some(Err(e))
            }
        }
    }
}

impl Drop for ToIterable<'_, '_, '_> {
    fn drop(&mut self) {
        self.finish();
    }
}
