use super::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TerrainCellKind {
    #[default]
    Invalid,
    Air,
    StoneDarker,
    Stone,
}

impl TryFrom<u8> for TerrainCellKind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Invalid),
            1 => Ok(Self::Air),
            2 => Ok(Self::StoneDarker),
            3 => Ok(Self::Stone),
            _ => Err(()),
        }
    }
}

impl Into<TerrainLeafCell> for TerrainCellKind {
    fn into(self) -> TerrainLeafCell {
        LeafCell::new(TerrainCellData { kind: self, distance: 0. })
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
    pub distance: f32,
}

impl TerrainCellData {
    pub fn average_distance(d: [&Self; 8]) -> f32 {
        let (count, sum) = d.iter()
            .fold((0f32, 0f32), |(count, sum), x| (count + 1., sum + x.distance));
        sum / count
    }

    pub fn density_delta(d: [&Self; 8]) -> f32 {
        let average = Self::average_distance(d);
        let (count, sum) = d.iter()
            .map(|x| (x.distance - average))
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

impl AggregateData for TerrainCellData {
    fn aggregate<'a>(d: [EitherDataRef<Self>; 8]) -> Self {
        let d = d.map(|x| x.into_inner());

        let mut most = [0u8; 256];

        d.iter().for_each(|k| {
            most[k.kind as u8 as usize] += 1;
        });

        let most_res = (most.iter().position_max().unwrap() as u8)
            .try_into().unwrap();

        Self {
            kind: most_res,
            distance: Self::average_distance(d),
        }
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
