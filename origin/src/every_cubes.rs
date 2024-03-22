use godot::builtin::{Aabb, Vector3};

pub struct EveryCubes {
    aabb: Aabb,
    cube_size: Vector3,
    current: Vector3,
}

impl Iterator for EveryCubes {
    type Item = Vector3;

    fn next(&mut self) -> Option<Self::Item> {
        // current.x = aabb_start.x;
        // while current.x < aabb_end.x {
        //     current.y = aabb_start.y;
        //     while current.y < aabb_end.y {
        //         current.z = aabb_start.z;
        //         while current.z < aabb_end.z {
        if self.current.z > self.aabb.end().z {
            return None;
        }

        let p = self.current;

        self.current.x += self.cube_size.x;
        if self.current.x > self.aabb.end().x {
            self.current.x = self.aabb.position.x;
            self.current.y += self.cube_size.y;
        }
        if self.current.y > self.aabb.end().y {
            self.current.y = self.aabb.position.y;
            self.current.z += self.cube_size.z;
        }

        Some(p)
    }
}

pub fn every_cubes(aabb: Aabb, cube_size: Vector3) -> EveryCubes {
    EveryCubes {
        aabb, cube_size,
        current: aabb.position
    }
}
