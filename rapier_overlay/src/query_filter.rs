use bevy::prelude::*;

use crate::*;
use rapier::{
    dynamics::RigidBodyHandle,
    geometry::{Collider, ColliderHandle, InteractionGroups},
    pipeline::QueryFilterFlags,
};

type QueryPredicate<'a> = &'a dyn Fn(Entity, &Collider) -> bool;

/// See [rapier::pipeline::QueryFilter]
#[derive(Copy, Clone, Default)]
pub struct QueryFilter<'a> {
    pub flags: QueryFilterFlags,
    pub groups: Option<InteractionGroups>,
    pub exclude_collider: Option<ColliderHandle>,
    pub exclude_rigid_body: Option<RigidBodyHandle>,
    pub predicate: Option<QueryPredicate<'a>>,
}

impl<'a> QueryFilter<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exclude_sensors(mut self) -> Self {
        self.flags |= QueryFilterFlags::EXCLUDE_SENSORS;
        self
    }

    pub fn exclude_solids(mut self) -> Self {
        self.flags |= QueryFilterFlags::EXCLUDE_SOLIDS;
        self
    }

    pub fn exclude_dyanmic(mut self) -> Self {
        self.flags |= QueryFilterFlags::EXCLUDE_DYNAMIC;
        self
    }

    pub fn exclude_fixed(mut self) -> Self {
        self.flags |= QueryFilterFlags::EXCLUDE_FIXED;
        self
    }

    pub fn exclude_kinematic(mut self) -> Self {
        self.flags |= QueryFilterFlags::EXCLUDE_KINEMATIC;
        self
    }

    pub fn groups(mut self, groups: InteractionGroups) -> Self {
        self.groups = Some(groups);
        self
    }

    pub fn exclude_collider(mut self, collider: ColliderHandle) -> Self {
        self.exclude_collider = Some(collider);
        self
    }

    pub fn exclude_rigid_body(mut self, rigid_body: RigidBodyHandle) -> Self {
        self.exclude_rigid_body = Some(rigid_body);
        self
    }

    pub fn predicate(mut self, predicate: &'a impl Fn(Entity, &Collider) -> bool) -> Self {
        self.predicate = Some(predicate);
        self
    }
}

impl<'a> std::fmt::Debug for QueryFilter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryFilter")
            .field("flags", &self.flags)
            .field("groups", &self.groups)
            .field("exclude_collider", &self.exclude_collider)
            .field("exclude_rigid_body", &self.exclude_rigid_body)
            .field("predicate", &self.predicate.as_ref().map(|_| ()))
            .finish()
    }
}

/// Store in a new variable $out a rapier::pipeline::QueryFilter from the
/// QueryFilter in $query using the rapier context in $ctx
///
/// is a macro for the lifetime of the created closure to be the one where
/// the macro is called
macro_rules! to_rapier_query {
    ($out: ident = $query: ident, $ctx: ident) => {
        let mapped_query_predicate = $query.predicate.map(|pred| {
            move |handle: ColliderHandle, collider: &Collider| -> bool {
                let Some(&entity) = $ctx.entities2colliders.get_by_right(&handle)
                else {
                    log::warn!("Collider has no registered entity");
                    return false;
                };

                pred(entity, collider)
            }
        });

        let $out = RapierQFilter {
            flags: $query.flags,
            groups: $query.groups,
            exclude_collider: $query.exclude_collider,
            exclude_rigid_body: $query.exclude_rigid_body,
            predicate: mapped_query_predicate.as_ref().map(|f| f as _),
        };
    };
}
pub(crate) use to_rapier_query;

impl From<QueryFilterFlags> for QueryFilter<'static> {
    fn from(flags: QueryFilterFlags) -> Self {
        Self {
            flags,
            ..default()
        }
    }
}
