use std::collections::HashMap;

use bevy_math::{bounding::Aabb3d, UVec3, Vec3, Vec4};
use bevy_render::{color::Color, mesh::{self, Mesh}, render_asset::RenderAssetUsages};
use ordered_float::OrderedFloat;
use utils::AabbExt as _;

use crate::{self as svo, CellPath, PackedIndexIterator, TerrainCellKind};

const EDGE_TABLE: [u16; 256] = [
0x0  , 0x109, 0x203, 0x30a, 0x406, 0x50f, 0x605, 0x70c,
0x80c, 0x905, 0xa0f, 0xb06, 0xc0a, 0xd03, 0xe09, 0xf00,
0x190, 0x99 , 0x393, 0x29a, 0x596, 0x49f, 0x795, 0x69c,
0x99c, 0x895, 0xb9f, 0xa96, 0xd9a, 0xc93, 0xf99, 0xe90,
0x230, 0x339, 0x33 , 0x13a, 0x636, 0x73f, 0x435, 0x53c,
0xa3c, 0xb35, 0x83f, 0x936, 0xe3a, 0xf33, 0xc39, 0xd30,
0x3a0, 0x2a9, 0x1a3, 0xaa , 0x7a6, 0x6af, 0x5a5, 0x4ac,
0xbac, 0xaa5, 0x9af, 0x8a6, 0xfaa, 0xea3, 0xda9, 0xca0,
0x460, 0x569, 0x663, 0x76a, 0x66 , 0x16f, 0x265, 0x36c,
0xc6c, 0xd65, 0xe6f, 0xf66, 0x86a, 0x963, 0xa69, 0xb60,
0x5f0, 0x4f9, 0x7f3, 0x6fa, 0x1f6, 0xff , 0x3f5, 0x2fc,
0xdfc, 0xcf5, 0xfff, 0xef6, 0x9fa, 0x8f3, 0xbf9, 0xaf0,
0x650, 0x759, 0x453, 0x55a, 0x256, 0x35f, 0x55 , 0x15c,
0xe5c, 0xf55, 0xc5f, 0xd56, 0xa5a, 0xb53, 0x859, 0x950,
0x7c0, 0x6c9, 0x5c3, 0x4ca, 0x3c6, 0x2cf, 0x1c5, 0xcc ,
0xfcc, 0xec5, 0xdcf, 0xcc6, 0xbca, 0xac3, 0x9c9, 0x8c0,
0x8c0, 0x9c9, 0xac3, 0xbca, 0xcc6, 0xdcf, 0xec5, 0xfcc,
0xcc , 0x1c5, 0x2cf, 0x3c6, 0x4ca, 0x5c3, 0x6c9, 0x7c0,
0x950, 0x859, 0xb53, 0xa5a, 0xd56, 0xc5f, 0xf55, 0xe5c,
0x15c, 0x55 , 0x35f, 0x256, 0x55a, 0x453, 0x759, 0x650,
0xaf0, 0xbf9, 0x8f3, 0x9fa, 0xef6, 0xfff, 0xcf5, 0xdfc,
0x2fc, 0x3f5, 0xff , 0x1f6, 0x6fa, 0x7f3, 0x4f9, 0x5f0,
0xb60, 0xa69, 0x963, 0x86a, 0xf66, 0xe6f, 0xd65, 0xc6c,
0x36c, 0x265, 0x16f, 0x66 , 0x76a, 0x663, 0x569, 0x460,
0xca0, 0xda9, 0xea3, 0xfaa, 0x8a6, 0x9af, 0xaa5, 0xbac,
0x4ac, 0x5a5, 0x6af, 0x7a6, 0xaa , 0x1a3, 0x2a9, 0x3a0,
0xd30, 0xc39, 0xf33, 0xe3a, 0x936, 0x83f, 0xb35, 0xa3c,
0x53c, 0x435, 0x73f, 0x636, 0x13a, 0x33 , 0x339, 0x230,
0xe90, 0xf99, 0xc93, 0xd9a, 0xa96, 0xb9f, 0x895, 0x99c,
0x69c, 0x795, 0x49f, 0x596, 0x29a, 0x393, 0x99 , 0x190,
0xf00, 0xe09, 0xd03, 0xc0a, 0xb06, 0xa0f, 0x905, 0x80c,
0x70c, 0x605, 0x50f, 0x406, 0x30a, 0x203, 0x109, 0x0
];

const EDGE_TO_VERTEX_TABLE: [[u8; 2]; 12] = [
    [0, 1],
    [1, 2],
    [2, 3],
    [3, 0],
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 4],

    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7],
];

const TRIANGULATIONS: [[i8; 15]; 256] = [ 
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  1,  9, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  8,  3,  9,  8,  1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  10,-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  3,  1,  2,  10,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 9,  2,  10, 0,  2,  9, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 2,  8,  3,  2,  10, 8,  10, 9,  8, -1, -1, -1, -1, -1, -1 ],
    [ 3,  11, 2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  11, 2,  8,  11, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  9,  0,  2,  3,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  11, 2,  1,  9,  11, 9,  8,  11,-1, -1, -1, -1, -1, -1 ],
    [ 3,  10, 1,  11, 10, 3, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  10, 1,  0,  8,  10, 8,  11, 10,-1, -1, -1, -1, -1, -1 ],
    [ 3,  9,  0,  3,  11, 9,  11, 10, 9, -1, -1, -1, -1, -1, -1 ],
    [ 9,  8,  10, 10, 8,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  7,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  3,  0,  7,  3,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  1,  9,  8,  4,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  1,  9,  4,  7,  1,  7,  3,  1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  10, 8,  4,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 3,  4,  7,  3,  0,  4,  1,  2,  10,-1, -1, -1, -1, -1, -1 ],
    [ 9,  2,  10, 9,  0,  2,  8,  4,  7, -1, -1, -1, -1, -1, -1 ],
    [ 2,  10, 9,  2,  9,  7,  2,  7,  3,  7,  9,  4, -1, -1, -1 ],
    [ 8,  4,  7,  3,  11, 2, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 11, 4,  7,  11, 2,  4,  2,  0,  4, -1, -1, -1, -1, -1, -1 ],
    [ 9,  0,  1,  8,  4,  7,  2,  3,  11,-1, -1, -1, -1, -1, -1 ],
    [ 4,  7,  11, 9,  4,  11, 9,  11, 2,  9,  2,  1, -1, -1, -1 ],
    [ 3,  10, 1,  3,  11, 10, 7,  8,  4, -1, -1, -1, -1, -1, -1 ],
    [ 1,  11, 10, 1,  4,  11, 1,  0,  4,  7,  11, 4, -1, -1, -1 ],
    [ 4,  7,  8,  9,  0,  11, 9,  11, 10, 11, 0,  3, -1, -1, -1 ],
    [ 4,  7,  11, 4,  11, 9,  9,  11, 10,-1, -1, -1, -1, -1, -1 ],
    [ 9,  5,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 9,  5,  4,  0,  8,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  5,  4,  1,  5,  0, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 8,  5,  4,  8,  3,  5,  3,  1,  5, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  10, 9,  5,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 3,  0,  8,  1,  2,  10, 4,  9,  5, -1, -1, -1, -1, -1, -1 ],
    [ 5,  2,  10, 5,  4,  2,  4,  0,  2, -1, -1, -1, -1, -1, -1 ],
    [ 2,  10, 5,  3,  2,  5,  3,  5,  4,  3,  4,  8, -1, -1, -1 ],
    [ 9,  5,  4,  2,  3,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  11, 2,  0,  8,  11, 4,  9,  5, -1, -1, -1, -1, -1, -1 ],
    [ 0,  5,  4,  0,  1,  5,  2,  3,  11,-1, -1, -1, -1, -1, -1 ],
    [ 2,  1,  5,  2,  5,  8,  2,  8,  11, 4,  8,  5, -1, -1, -1 ],
    [ 10, 3,  11, 10, 1,  3,  9,  5,  4, -1, -1, -1, -1, -1, -1 ],
    [ 4,  9,  5,  0,  8,  1,  8,  10, 1,  8,  11, 10,-1, -1, -1 ],
    [ 5,  4,  0,  5,  0,  11, 5,  11, 10, 11, 0,  3, -1, -1, -1 ],
    [ 5,  4,  8,  5,  8,  10, 10, 8,  11,-1, -1, -1, -1, -1, -1 ],
    [ 9,  7,  8,  5,  7,  9, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 9,  3,  0,  9,  5,  3,  5,  7,  3, -1, -1, -1, -1, -1, -1 ],
    [ 0,  7,  8,  0,  1,  7,  1,  5,  7, -1, -1, -1, -1, -1, -1 ],
    [ 1,  5,  3,  3,  5,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 9,  7,  8,  9,  5,  7,  10, 1,  2, -1, -1, -1, -1, -1, -1 ],
    [ 10, 1,  2,  9,  5,  0,  5,  3,  0,  5,  7,  3, -1, -1, -1 ],
    [ 8,  0,  2,  8,  2,  5,  8,  5,  7,  10, 5,  2, -1, -1, -1 ],
    [ 2,  10, 5,  2,  5,  3,  3,  5,  7, -1, -1, -1, -1, -1, -1 ],
    [ 7,  9,  5,  7,  8,  9,  3,  11, 2, -1, -1, -1, -1, -1, -1 ],
    [ 9,  5,  7,  9,  7,  2,  9,  2,  0,  2,  7,  11,-1, -1, -1 ],
    [ 2,  3,  11, 0,  1,  8,  1,  7,  8,  1,  5,  7, -1, -1, -1 ],
    [ 11, 2,  1,  11, 1,  7,  7,  1,  5, -1, -1, -1, -1, -1, -1 ],
    [ 9,  5,  8,  8,  5,  7,  10, 1,  3,  10, 3,  11,-1, -1, -1 ],
    [ 5,  7,  0,  5,  0,  9,  7,  11, 0,  1,  0,  10, 11, 10, 0 ],
    [ 11, 10, 0,  11, 0,  3,  10, 5,  0,  8,  0,  7,  5,  7,  0 ],
    [ 11, 10, 5,  7,  11, 5, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 10, 6,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  3,  5,  10, 6, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 9,  0,  1,  5,  10, 6, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  8,  3,  1,  9,  8,  5,  10, 6, -1, -1, -1, -1, -1, -1 ],
    [ 1,  6,  5,  2,  6,  1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  6,  5,  1,  2,  6,  3,  0,  8, -1, -1, -1, -1, -1, -1 ],
    [ 9,  6,  5,  9,  0,  6,  0,  2,  6, -1, -1, -1, -1, -1, -1 ],
    [ 5,  9,  8,  5,  8,  2,  5,  2,  6,  3,  2,  8, -1, -1, -1 ],
    [ 2,  3,  11, 10, 6,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 11, 0,  8,  11, 2,  0,  10, 6,  5, -1, -1, -1, -1, -1, -1 ],
    [ 0,  1,  9,  2,  3,  11, 5,  10, 6, -1, -1, -1, -1, -1, -1 ],
    [ 5,  10, 6,  1,  9,  2,  9,  11, 2,  9,  8,  11,-1, -1, -1 ],
    [ 6,  3,  11, 6,  5,  3,  5,  1,  3, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  11, 0,  11, 5,  0,  5,  1,  5,  11, 6, -1, -1, -1 ],
    [ 3,  11, 6,  0,  3,  6,  0,  6,  5,  0,  5,  9, -1, -1, -1 ],
    [ 6,  5,  9,  6,  9,  11, 11, 9,  8, -1, -1, -1, -1, -1, -1 ],
    [ 5,  10, 6,  4,  7,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  3,  0,  4,  7,  3,  6,  5,  10,-1, -1, -1, -1, -1, -1 ],
    [ 1,  9,  0,  5,  10, 6,  8,  4,  7, -1, -1, -1, -1, -1, -1 ],
    [ 10, 6,  5,  1,  9,  7,  1,  7,  3,  7,  9,  4, -1, -1, -1 ],
    [ 6,  1,  2,  6,  5,  1,  4,  7,  8, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  5,  5,  2,  6,  3,  0,  4,  3,  4,  7, -1, -1, -1 ],
    [ 8,  4,  7,  9,  0,  5,  0,  6,  5,  0,  2,  6, -1, -1, -1 ],
    [ 7,  3,  9,  7,  9,  4,  3,  2,  9,  5,  9,  6,  2,  6,  9 ],
    [ 3,  11, 2,  7,  8,  4,  10, 6,  5, -1, -1, -1, -1, -1, -1 ],
    [ 5,  10, 6,  4,  7,  2,  4,  2,  0,  2,  7,  11,-1, -1, -1 ],
    [ 0,  1,  9,  4,  7,  8,  2,  3,  11, 5,  10, 6, -1, -1, -1 ],
    [ 9,  2,  1,  9,  11, 2,  9,  4,  11, 7,  11, 4,  5,  10, 6 ],
    [ 8,  4,  7,  3,  11, 5,  3,  5,  1,  5,  11, 6, -1, -1, -1 ],
    [ 5,  1,  11, 5,  11, 6,  1,  0,  11, 7,  11, 4,  0,  4,  11],
    [ 0,  5,  9,  0,  6,  5,  0,  3,  6,  11, 6,  3,  8,  4,  7 ],
    [ 6,  5,  9,  6,  9,  11, 4,  7,  9,  7,  11, 9, -1, -1, -1 ],
    [ 10, 4,  9,  6,  4,  10,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  10, 6,  4,  9,  10, 0,  8,  3, -1, -1, -1, -1, -1, -1 ],
    [ 10, 0,  1,  10, 6,  0,  6,  4,  0, -1, -1, -1, -1, -1, -1 ],
    [ 8,  3,  1,  8,  1,  6,  8,  6,  4,  6,  1,  10,-1, -1, -1 ],
    [ 1,  4,  9,  1,  2,  4,  2,  6,  4, -1, -1, -1, -1, -1, -1 ],
    [ 3,  0,  8,  1,  2,  9,  2,  4,  9,  2,  6,  4, -1, -1, -1 ],
    [ 0,  2,  4,  4,  2,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 8,  3,  2,  8,  2,  4,  4,  2,  6, -1, -1, -1, -1, -1, -1 ],
    [ 10, 4,  9,  10, 6,  4,  11, 2,  3, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  2,  2,  8,  11, 4,  9,  10, 4,  10, 6, -1, -1, -1 ],
    [ 3,  11, 2,  0,  1,  6,  0,  6,  4,  6,  1,  10,-1, -1, -1 ],
    [ 6,  4,  1,  6,  1,  10, 4,  8,  1,  2,  1,  11, 8,  11, 1 ],
    [ 9,  6,  4,  9,  3,  6,  9,  1,  3,  11, 6,  3, -1, -1, -1 ],
    [ 8,  11, 1,  8,  1,  0,  11, 6,  1,  9,  1,  4,  6,  4,  1 ],
    [ 3,  11, 6,  3,  6,  0,  0,  6,  4, -1, -1, -1, -1, -1, -1 ],
    [ 6,  4,  8,  11, 6,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 7,  10, 6,  7,  8,  10, 8,  9,  10,-1, -1, -1, -1, -1, -1 ],
    [ 0,  7,  3,  0,  10, 7,  0,  9,  10, 6,  7,  10,-1, -1, -1 ],
    [ 10, 6,  7,  1,  10, 7,  1,  7,  8,  1,  8,  0, -1, -1, -1 ],
    [ 10, 6,  7,  10, 7,  1,  1,  7,  3, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  6,  1,  6,  8,  1,  8,  9,  8,  6,  7, -1, -1, -1 ],
    [ 2,  6,  9,  2,  9,  1,  6,  7,  9,  0,  9,  3,  7,  3,  9 ],
    [ 7,  8,  0,  7,  0,  6,  6,  0,  2, -1, -1, -1, -1, -1, -1 ],
    [ 7,  3,  2,  6,  7,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 2,  3,  11, 10, 6,  8,  10, 8,  9,  8,  6,  7, -1, -1, -1 ],
    [ 2,  0,  7,  2,  7,  11, 0,  9,  7,  6,  7,  10, 9,  10, 7 ],
    [ 1,  8,  0,  1,  7,  8,  1,  10, 7,  6,  7,  10, 2,  3,  11],
    [ 11, 2,  1,  11, 1,  7,  10, 6,  1,  6,  7,  1, -1, -1, -1 ],
    [ 8,  9,  6,  8,  6,  7,  9,  1,  6,  11, 6,  3,  1,  3,  6 ],
    [ 0,  9,  1,  11, 6,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 7,  8,  0,  7,  0,  6,  3,  11, 0,  11, 6,  0, -1, -1, -1 ],
    [ 7,  11, 6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 7,  6,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 3,  0,  8,  11, 7,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  1,  9,  11, 7,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 8,  1,  9,  8,  3,  1,  11, 7,  6, -1, -1, -1, -1, -1, -1 ],
    [ 10, 1,  2,  6,  11, 7, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  10, 3,  0,  8,  6,  11, 7, -1, -1, -1, -1, -1, -1 ],
    [ 2,  9,  0,  2,  10, 9,  6,  11, 7, -1, -1, -1, -1, -1, -1 ],
    [ 6,  11, 7,  2,  10, 3,  10, 8,  3,  10, 9,  8, -1, -1, -1 ],
    [ 7,  2,  3,  6,  2,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 7,  0,  8,  7,  6,  0,  6,  2,  0, -1, -1, -1, -1, -1, -1 ],
    [ 2,  7,  6,  2,  3,  7,  0,  1,  9, -1, -1, -1, -1, -1, -1 ],
    [ 1,  6,  2,  1,  8,  6,  1,  9,  8,  8,  7,  6, -1, -1, -1 ],
    [ 10, 7,  6,  10, 1,  7,  1,  3,  7, -1, -1, -1, -1, -1, -1 ],
    [ 10, 7,  6,  1,  7,  10, 1,  8,  7,  1,  0,  8, -1, -1, -1 ],
    [ 0,  3,  7,  0,  7,  10, 0,  10, 9,  6,  10, 7, -1, -1, -1 ],
    [ 7,  6,  10, 7,  10, 8,  8,  10, 9, -1, -1, -1, -1, -1, -1 ],
    [ 6,  8,  4,  11, 8,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 3,  6,  11, 3,  0,  6,  0,  4,  6, -1, -1, -1, -1, -1, -1 ],
    [ 8,  6,  11, 8,  4,  6,  9,  0,  1, -1, -1, -1, -1, -1, -1 ],
    [ 9,  4,  6,  9,  6,  3,  9,  3,  1,  11, 3,  6, -1, -1, -1 ],
    [ 6,  8,  4,  6,  11, 8,  2,  10, 1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  10, 3,  0,  11, 0,  6,  11, 0,  4,  6, -1, -1, -1 ],
    [ 4,  11, 8,  4,  6,  11, 0,  2,  9,  2,  10, 9, -1, -1, -1 ],
    [ 10, 9,  3,  10, 3,  2,  9,  4,  3,  11, 3,  6,  4,  6,  3 ],
    [ 8,  2,  3,  8,  4,  2,  4,  6,  2, -1, -1, -1, -1, -1, -1 ],
    [ 0,  4,  2,  4,  6,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  9,  0,  2,  3,  4,  2,  4,  6,  4,  3,  8, -1, -1, -1 ],
    [ 1,  9,  4,  1,  4,  2,  2,  4,  6, -1, -1, -1, -1, -1, -1 ],
    [ 8,  1,  3,  8,  6,  1,  8,  4,  6,  6,  10, 1, -1, -1, -1 ],
    [ 10, 1,  0,  10, 0,  6,  6,  0,  4, -1, -1, -1, -1, -1, -1 ],
    [ 4,  6,  3,  4,  3,  8,  6,  10, 3,  0,  3,  9,  10, 9,  3 ],
    [ 10, 9,  4,  6,  10, 4, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  9,  5,  7,  6,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  3,  4,  9,  5,  11, 7,  6, -1, -1, -1, -1, -1, -1 ],
    [ 5,  0,  1,  5,  4,  0,  7,  6,  11,-1, -1, -1, -1, -1, -1 ],
    [ 11, 7,  6,  8,  3,  4,  3,  5,  4,  3,  1,  5, -1, -1, -1 ],
    [ 9,  5,  4,  10, 1,  2,  7,  6,  11,-1, -1, -1, -1, -1, -1 ],
    [ 6,  11, 7,  1,  2,  10, 0,  8,  3,  4,  9,  5, -1, -1, -1 ],
    [ 7,  6,  11, 5,  4,  10, 4,  2,  10, 4,  0,  2, -1, -1, -1 ],
    [ 3,  4,  8,  3,  5,  4,  3,  2,  5,  10, 5,  2,  11, 7,  6 ],
    [ 7,  2,  3,  7,  6,  2,  5,  4,  9, -1, -1, -1, -1, -1, -1 ],
    [ 9,  5,  4,  0,  8,  6,  0,  6,  2,  6,  8,  7, -1, -1, -1 ],
    [ 3,  6,  2,  3,  7,  6,  1,  5,  0,  5,  4,  0, -1, -1, -1 ],
    [ 6,  2,  8,  6,  8,  7,  2,  1,  8,  4,  8,  5,  1,  5,  8 ],
    [ 9,  5,  4,  10, 1,  6,  1,  7,  6,  1,  3,  7, -1, -1, -1 ],
    [ 1,  6,  10, 1,  7,  6,  1,  0,  7,  8,  7,  0,  9,  5,  4 ],
    [ 4,  0,  10, 4,  10, 5,  0,  3,  10, 6,  10, 7,  3,  7,  10],
    [ 7,  6,  10, 7,  10, 8,  5,  4,  10, 4,  8,  10,-1, -1, -1 ],
    [ 6,  9,  5,  6,  11, 9,  11, 8,  9, -1, -1, -1, -1, -1, -1 ],
    [ 3,  6,  11, 0,  6,  3,  0,  5,  6,  0,  9,  5, -1, -1, -1 ],
    [ 0,  11, 8,  0,  5,  11, 0,  1,  5,  5,  6,  11,-1, -1, -1 ],
    [ 6,  11, 3,  6,  3,  5,  5,  3,  1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  10, 9,  5,  11, 9,  11, 8,  11, 5,  6, -1, -1, -1 ],
    [ 0,  11, 3,  0,  6,  11, 0,  9,  6,  5,  6,  9,  1,  2,  10],
    [ 11, 8,  5,  11, 5,  6,  8,  0,  5,  10, 5,  2,  0,  2,  5 ],
    [ 6,  11, 3,  6,  3,  5,  2,  10, 3,  10, 5,  3, -1, -1, -1 ],
    [ 5,  8,  9,  5,  2,  8,  5,  6,  2,  3,  8,  2, -1, -1, -1 ],
    [ 9,  5,  6,  9,  6,  0,  0,  6,  2, -1, -1, -1, -1, -1, -1 ],
    [ 1,  5,  8,  1,  8,  0,  5,  6,  8,  3,  8,  2,  6,  2,  8 ],
    [ 1,  5,  6,  2,  1,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  3,  6,  1,  6,  10, 3,  8,  6,  5,  6,  9,  8,  9,  6 ],
    [ 10, 1,  0,  10, 0,  6,  9,  5,  0,  5,  6,  0, -1, -1, -1 ],
    [ 0,  3,  8,  5,  6,  10,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 10, 5,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 11, 5,  10, 7,  5,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 11, 5,  10, 11, 7,  5,  8,  3,  0, -1, -1, -1, -1, -1, -1 ],
    [ 5,  11, 7,  5,  10, 11, 1,  9,  0, -1, -1, -1, -1, -1, -1 ],
    [ 10, 7,  5,  10, 11, 7,  9,  8,  1,  8,  3,  1, -1, -1, -1 ],
    [ 11, 1,  2,  11, 7,  1,  7,  5,  1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  3,  1,  2,  7,  1,  7,  5,  7,  2,  11,-1, -1, -1 ],
    [ 9,  7,  5,  9,  2,  7,  9,  0,  2,  2,  11, 7, -1, -1, -1 ],
    [ 7,  5,  2,  7,  2,  11, 5,  9,  2,  3,  2,  8,  9,  8,  2 ],
    [ 2,  5,  10, 2,  3,  5,  3,  7,  5, -1, -1, -1, -1, -1, -1 ],
    [ 8,  2,  0,  8,  5,  2,  8,  7,  5,  10, 2,  5, -1, -1, -1 ],
    [ 9,  0,  1,  5,  10, 3,  5,  3,  7,  3,  10, 2, -1, -1, -1 ],
    [ 9,  8,  2,  9,  2,  1,  8,  7,  2,  10, 2,  5,  7,  5,  2 ],
    [ 1,  3,  5,  3,  7,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  7,  0,  7,  1,  1,  7,  5, -1, -1, -1, -1, -1, -1 ],
    [ 9,  0,  3,  9,  3,  5,  5,  3,  7, -1, -1, -1, -1, -1, -1 ],
    [ 9,  8,  7,  5,  9,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 5,  8,  4,  5,  10, 8,  10, 11, 8, -1, -1, -1, -1, -1, -1 ],
    [ 5,  0,  4,  5,  11, 0,  5,  10, 11, 11, 3,  0, -1, -1, -1 ],
    [ 0,  1,  9,  8,  4,  10, 8,  10, 11, 10, 4,  5, -1, -1, -1 ],
    [ 10, 11, 4,  10, 4,  5,  11, 3,  4,  9,  4,  1,  3,  1,  4 ],
    [ 2,  5,  1,  2,  8,  5,  2,  11, 8,  4,  5,  8, -1, -1, -1 ],
    [ 0,  4,  11, 0,  11, 3,  4,  5,  11, 2,  11, 1,  5,  1,  11],
    [ 0,  2,  5,  0,  5,  9,  2,  11, 5,  4,  5,  8,  11, 8,  5 ],
    [ 9,  4,  5,  2,  11, 3, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 2,  5,  10, 3,  5,  2,  3,  4,  5,  3,  8,  4, -1, -1, -1 ],
    [ 5,  10, 2,  5,  2,  4,  4,  2,  0, -1, -1, -1, -1, -1, -1 ],
    [ 3,  10, 2,  3,  5,  10, 3,  8,  5,  4,  5,  8,  0,  1,  9 ],
    [ 5,  10, 2,  5,  2,  4,  1,  9,  2,  9,  4,  2, -1, -1, -1 ],
    [ 8,  4,  5,  8,  5,  3,  3,  5,  1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  4,  5,  1,  0,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 8,  4,  5,  8,  5,  3,  9,  0,  5,  0,  3,  5, -1, -1, -1 ],
    [ 9,  4,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  11, 7,  4,  9,  11, 9,  10, 11,-1, -1, -1, -1, -1, -1 ],
    [ 0,  8,  3,  4,  9,  7,  9,  11, 7,  9,  10, 11,-1, -1, -1 ],
    [ 1,  10, 11, 1,  11, 4,  1,  4,  0,  7,  4,  11,-1, -1, -1 ],
    [ 3,  1,  4,  3,  4,  8,  1,  10, 4,  7,  4,  11, 10, 11, 4 ],
    [ 4,  11, 7,  9,  11, 4,  9,  2,  11, 9,  1,  2, -1, -1, -1 ],
    [ 9,  7,  4,  9,  11, 7,  9,  1,  11, 2,  11, 1,  0,  8,  3 ],
    [ 11, 7,  4,  11, 4,  2,  2,  4,  0, -1, -1, -1, -1, -1, -1 ],
    [ 11, 7,  4,  11, 4,  2,  8,  3,  4,  3,  2,  4, -1, -1, -1 ],
    [ 2,  9,  10, 2,  7,  9,  2,  3,  7,  7,  4,  9, -1, -1, -1 ],
    [ 9,  10, 7,  9,  7,  4,  10, 2,  7,  8,  7,  0,  2,  0,  7 ],
    [ 3,  7,  10, 3,  10, 2,  7,  4,  10, 1,  10, 0,  4,  0,  10],
    [ 1,  10, 2,  8,  7,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  9,  1,  4,  1,  7,  7,  1,  3, -1, -1, -1, -1, -1, -1 ],
    [ 4,  9,  1,  4,  1,  7,  0,  8,  1,  8,  7,  1, -1, -1, -1 ],
    [ 4,  0,  3,  7,  4,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 4,  8,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 9,  10, 8,  10, 11, 8, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 3,  0,  9,  3,  9,  11, 11, 9,  10,-1, -1, -1, -1, -1, -1 ],
    [ 0,  1,  10, 0,  10, 8,  8,  10, 11,-1, -1, -1, -1, -1, -1 ],
    [ 3,  1,  10, 11, 3,  10,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  2,  11, 1,  11, 9,  9,  11, 8, -1, -1, -1, -1, -1, -1 ],
    [ 3,  0,  9,  3,  9,  11, 1,  2,  9,  2,  11, 9, -1, -1, -1 ],
    [ 0,  2,  11, 8,  0,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 3,  2,  11,-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 2,  3,  8,  2,  8,  10, 10, 8,  9, -1, -1, -1, -1, -1, -1 ],
    [ 9,  10, 2,  0,  9,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 2,  3,  8,  2,  8,  10, 0,  1,  8,  1,  10, 8, -1, -1, -1 ],
    [ 1,  10, 2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 1,  3,  8,  9,  1,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  9,  1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [ 0,  3,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1 ],
];

#[derive(Debug, Default)]
pub struct Out {
    pub indexed: bool,
    pub smooth: bool,

    pub indices: Vec<u32>,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub colors: Vec<Vec4>,
}

impl Out {
    pub fn new(indexed: bool, smooth: bool) -> Self {
        Self {
            indexed,
            smooth,
            ..Default::default()
        }
    }

    pub fn into_mesh(&mut self) -> Mesh {
        let vertices = std::mem::take(&mut self.vertices);
        let normals = std::mem::take(&mut self.normals);
        let colors = std::mem::take(&mut self.colors);
        let indices = std::mem::take(&mut self.indices);

        let mut m = Mesh::new(mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::all())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        if self.indexed {
            m = m.with_inserted_indices(mesh::Indices::U32(indices))
        }

        m
    }
}

struct Index {
    index: usize,
    count: f32,
    sum_normal: Vec3,
}

struct State<'a> {
    indices: HashMap<([OrderedFloat<f32>; 3], [OrderedFloat<f32>; 4]), Index>,
    color: Color,
    normal: Vec3,
    out: &'a mut Out,
}

impl<'a> State<'a> {
    pub fn new(out: &'a mut Out) -> Self {
        Self {
            indices: HashMap::new(),
            out,
            color: Color::rgba(1.,1.,1.,0.),
            normal: Vec3::ZERO,
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_normal(&mut self, normal: Vec3) {
        self.normal = normal;
    }

    pub fn add_vertex(&mut self, pos: Vec3) {
        if self.out.indexed && self.out.smooth {
            let key = (
                [pos.x, pos.y, pos.z].map(OrderedFloat),
                [self.color.r(), self.color.g(), self.color.b(), self.color.a()].map(OrderedFloat)
            );
            let entry = self.indices.entry(key).or_insert_with(|| {
                let idx = self.out.vertices.len();
                self.out.normals.push(self.normal);
                self.out.colors.push(self.color.rgba_to_vec4());
                self.out.vertices.push(pos);
                Index {
                    index: idx,
                    count: 0.,
                    sum_normal: Vec3::ZERO,
                }
            });
            entry.count += 1.;
            entry.sum_normal += self.normal;
            self.out.normals[entry.index] =
                entry.sum_normal / entry.count;
            self.out.indices.push(entry.index.try_into().unwrap());
        }
        else if self.out.indexed && !self.out.smooth {
            let index = self.out.indices.len();
            self.out.normals.push(self.normal);
            self.out.colors.push(self.color.rgba_to_vec4());
            self.out.vertices.push(pos);
            self.out.indices.push(index.try_into().unwrap());
        }
        else {
            self.out.normals.push(self.normal);
            self.out.colors.push(self.color.rgba_to_vec4());
            self.out.vertices.push(pos);
        }
    }
}

fn kernel(
    state: &mut State,
    samples: [(f32, TerrainCellKind); 8],
    vertices: [Vec3; 8]
) {
    let id = samples.iter().rev().fold(0u8, |id, (_, k)| {
        (id << 1) | if *k == TerrainCellKind::Air || *k == TerrainCellKind::Invalid { 0 } else { 1 }
    });

    let mut edges = [Vec3::ONE * -1.; 12];
    let mut edges_mats = [Color::WHITE; 12];
    let edges_to_take = EDGE_TABLE[id as usize];
    (0..u16::BITS).filter(|i| (edges_to_take & (1 << i)) != 0)
        .map(|i| i as usize)
        .for_each(|i| {
            let [ai, bi] = EDGE_TO_VERTEX_TABLE[i]
                .map(|i| i as usize);
            let [a, b] = [vertices[ai], vertices[bi]];
            let [sa, sb] = [samples[ai], samples[bi]];
            let [da, db] = [sa, sb]
                .map(|x| x.0);
            edges[i] = a + -da * (b - a) / (db - da);
            // edges[i] = (a + b) / 2.;
            let kind = if db > da { sa } else { sb }.1;
            edges_mats[i] = kind.color();
        });
    
    TRIANGULATIONS[id as usize].into_iter()
        .take_while(|&x| x != -1)
        .array_chunks::<3>()
        .for_each(|v| {
            let arr = v.map(|i| edges[i as usize]);
            let mat = v.map(|i| edges_mats[i as usize]);

            let a = arr[0] - arr[1];
            let b = arr[2] - arr[1];
            let normal = -a.cross(b).normalize();

            state.set_normal(normal);

            let color: Color = mat.iter().copied().reduce(|a, b| a + b).unwrap();

            state.set_color(color * (1./3.));

            state.add_vertex(arr[0]);
            state.add_vertex(arr[1]);
            state.add_vertex(arr[2]);
        });
}

const VERTICES: [UVec3; 8] = [
    UVec3::new(0, 0, 0), UVec3::new(1, 0, 0),
    UVec3::new(1, 0, 1), UVec3::new(0, 0, 1),
    UVec3::new(0, 1, 0), UVec3::new(1, 1, 0),
    UVec3::new(1, 1, 1), UVec3::new(0, 1, 1),
];

pub fn run(
    out: &mut Out,
    chunk: CellPath,
    root_cell: &svo::TerrainCell,
    root_aabb: Aabb3d,
    depth: u32,
) {
    let aabb: Aabb3d = chunk.get_aabb(root_aabb.into()).into();
    let cube_size = aabb.size() / 2f32.powi(depth as i32);

    let mut state = State::new(out);

    for (_, subpath) in PackedIndexIterator::new(depth) {
        let path = chunk.extended(subpath);
        let pos = subpath.get_pos();

        let samples = VERTICES
            .map(|v| {
                path.neighbor(v.x as _, v.y as _, v.z as _)
                    .map(|n| root_cell.get_path(n).into_inner())
                    .map(|cell| (cell.distance, cell.kind))
                    .unwrap_or_default()
            });

        kernel(
            &mut state, samples, VERTICES
                .map(|d| (pos + d).as_vec3() * cube_size + aabb.min)
        );
    }
}
