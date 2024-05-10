use bevy_render::color::Color;
use half::f16;

use super::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TerrainCellKind {
    #[default]
    Invalid,
    Air,
    StoneDarker,
    Stone,
    Pink,
    Blue,
}

impl TerrainCellKind {
    pub fn color(&self) -> Color {
        match self {
            TerrainCellKind::Invalid => Color::rgba(0.,0.,0.,0.),
            TerrainCellKind::Air => Color::rgba(1.,1.,1.,0.),
            TerrainCellKind::StoneDarker => Color::rgb(0.6, 0.6, 0.6),
            TerrainCellKind::Stone => Color::rgb(0.3, 0.3, 0.3),
            TerrainCellKind::Pink => Color::rgb(1., 0., 0.69),
            TerrainCellKind::Blue => Color::rgb(0.1059, 0.2570, 0.5451),
        }
    }

    pub fn empty(&self) -> bool {
        match self {
            TerrainCellKind::Invalid => true,
            TerrainCellKind::Air => true,
            _ => false,
        }
    }
}

impl Into<TerrainLeafCell> for TerrainCellKind {
    fn into(self) -> TerrainLeafCell {
        LeafCell::new(TerrainCellData {
            kind: self,
            distance: f16::ZERO,
            empty: self.empty(),
        })
    }
}

impl Into<TerrainCell> for TerrainCellKind {
    fn into(self) -> TerrainCell {
        Into::<TerrainLeafCell>::into(self).into()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct TerrainCellData {
    pub kind: TerrainCellKind,
    /// The only important disances for marching cube are those close to 0
    /// so f16 precision is sufficient
    pub distance: f16,
    /// Wether any children or self has any non-air terrain
    pub empty: bool,
}

impl TerrainCellData {
    pub fn average_distance(d: [&Self; 8]) -> f32 {
        let (count, sum) = d.iter()
            .map(|x| x.distance.to_f32())
            .fold((0f32, 0f32), |(count, sum), distance| (count + 1., sum + distance));
        sum / count
    }

    pub fn density_delta(d: [&Self; 8]) -> f32 {
        let average = Self::average_distance(d);
        let (count, sum) = d.iter().map(|x| x.distance.to_f32())
            .map(|distance| (distance - average))
            .fold((0f32, 0f32), |(count, sum), x| (
                count + 1.,
                sum + (x - average).powi(2)
            ));

        (sum / count).sqrt()
    }
}

impl Data for TerrainCellData {
    type Internal = Self;
}

impl InternalData for TerrainCellData {  }

impl SplittableData for TerrainCellData {
    fn split(self) -> (Self::Internal, [Self; 8]) {
        (self, [self; 8])
    }
}

impl AggregateData for TerrainCellData {
    fn aggregate<'a>(d: [EitherDataRef<Self>; 8]) -> Self {
        Self {
            empty: d.iter().all(|d| d.empty),
            ..*d[0].into_inner()   
        }
    }
}

impl MergeableData for TerrainCellData {
    fn should_auto_merge(
        _this: &TerrainCellData,
        d: [&Self; 8]
    ) -> bool {
        d.iter().map(|c| c.kind).all_equal() &&
        Self::average_distance(d) > 10.
    }

    fn merge(
        this: Self::Internal,
        _children: [Self; 8]
    ) -> Self {
        this
    }
}

pub type TerrainCell = Cell<TerrainCellData>;
pub type TerrainInternalCell = InternalCell<TerrainCellData>;
pub type TerrainLeafCell = LeafCell<TerrainCellData>;
pub type TerrainPackedCell = PackedCell<TerrainCellData>;
