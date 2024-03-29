use bevy::prelude::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChunkMeshGenSettings {
    pub subdivs: u32,
    pub collisions: bool,
}

#[derive(Debug, Clone)]
pub struct ChunkSvoData {
    pub entity: Entity,
}

impl Default for ChunkSvoData {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
        }
    }
}

impl svo::InternalData for ChunkSvoData {}

impl svo::Data for ChunkSvoData {
    type Internal = Self;
}

impl svo::AggregateData for ChunkSvoData {
    fn aggregate<'a>(
        _children: [svo::EitherDataRef<Self>; 8]
    ) -> Self::Internal {
        Self::default()
    }
}
