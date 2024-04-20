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
}

impl TryFrom<u8> for TerrainCellKind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Invalid),
            1 => Ok(Self::Air),
            2 => Ok(Self::StoneDarker),
            3 => Ok(Self::Stone),
            4 => Ok(Self::Stone),
            _ => Err(()),
        }
    }
}

impl TerrainCellKind {
    pub fn color(&self) -> Color {
        match self {
            TerrainCellKind::Invalid => Color::rgba(0.,0.,0.,0.),
            TerrainCellKind::Air => Color::rgba(1.,1.,1.,0.),
            TerrainCellKind::StoneDarker => Color::rgb(0.6, 0.6, 0.6),
            TerrainCellKind::Stone => Color::rgb(0.3, 0.3, 0.3),
            TerrainCellKind::Pink => Color::rgb(1., 0., 0.69),
        }
    }
}

impl Into<TerrainLeafCell> for TerrainCellKind {
    fn into(self) -> TerrainLeafCell {
        LeafCell::new(TerrainCellData { kind: self, distance: f16::ZERO })
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
    pub distance: f16,
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
        *d[0].into_inner()
    }
}

impl MergeableData for TerrainCellData {
    fn can_merge(
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
