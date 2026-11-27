//! Class, which can handle completion of hydration cycle and produce some
//! of tasks. In current implementation triggers deferred postLoad event.
//!
//! The listeners invoker and the entity manager are the arguments of the
//! calls here, not the handler's own: the unit of work keeps the handler
//! in its state and invokes the listeners without holding that state.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: doctrine/orm src/Internal/HydrationCompleteHandler.php

use std::sync::Arc;

use php_value::{Object, Thrown};

use crate::entity_manager::EntityManager;
use crate::event::ListenersInvoker;
use crate::event::{LifecycleEventArgs, POST_LOAD_EVENT_ARGS};
use crate::events;

/// What the handler asks of the `ListenersInvoker`, for class metadata
/// of type `C`.
pub trait PostLoadListenersInvoker<C> {
    /// `getSubscribedSystems($class, $eventName)`.
    fn get_subscribed_systems(&self, class: &C, event_name: &str) -> i32;

    /// `invoke($class, $eventName, $entity, $event, $invoke)`.
    fn invoke(
        &self,
        class: &C,
        event_name: &str,
        entity: &Object,
        event: &LifecycleEventArgs,
        invoke: i32,
    ) -> Result<(), Thrown>;
}

/// `Doctrine\ORM\Internal\HydrationCompleteHandler`.
#[derive(Debug)]
pub struct HydrationCompleteHandler<C> {
    deferred_post_load_invocations: Vec<(C, i32, Object)>,
}

impl<C> Default for HydrationCompleteHandler<C> {
    fn default() -> Self {
        Self {
            deferred_post_load_invocations: Vec::new(),
        }
    }
}

impl<C> HydrationCompleteHandler<C> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Method schedules invoking of postLoad entity to the very end of
    /// current hydration cycle.
    pub fn defer_post_load_invoking(
        &mut self,
        listeners_invoker: &dyn PostLoadListenersInvoker<C>,
        class: C,
        entity: Object,
    ) {
        let invoke = listeners_invoker.get_subscribed_systems(&class, events::POST_LOAD);

        if invoke == ListenersInvoker::INVOKE_NONE {
            return;
        }

        self.deferred_post_load_invocations
            .push((class, invoke, entity));
    }

    /// This method should be called after any hydration cycle completed.
    ///
    /// Method fires all deferred invocations of postLoad events.
    pub fn hydration_complete(
        &mut self,
        listeners_invoker: &dyn PostLoadListenersInvoker<C>,
        em: &Arc<EntityManager>,
    ) -> Result<(), Thrown> {
        let to_invoke = std::mem::take(&mut self.deferred_post_load_invocations);

        for (class, invoke, entity) in to_invoke {
            listeners_invoker.invoke(
                &class,
                events::POST_LOAD,
                &entity,
                &LifecycleEventArgs::new(POST_LOAD_EVENT_ARGS, entity.clone(), em.clone()),
                invoke,
            )?;
        }
        Ok(())
    }
}
