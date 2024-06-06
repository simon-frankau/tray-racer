//
// vec4.rs: A 4D vector class, allowing us to represent the space in
// which our 3D space is embedded.
//

#[derive(Clone, Copy, Debug)]
pub struct Vec4 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    // This dimension is *not* the homogeneous coordinate
    // representation for perspective etc. It's a genuine 4th
    // dimension in which our 3D space is embedded.
    pub w: f64,
}

// Provide a couple of type synonyms to distinguish between the usage
// as a point as and as a direction.
pub type Point4 = Vec4;
pub type Dir4 = Vec4;

impl Vec4 {
    pub fn scale(&self, m: f64) -> Vec4 {
        Vec4 {
            x: self.x * m,
            y: self.y * m,
            z: self.z * m,
            w: self.w * m,
        }
    }

    pub fn add(&self, rhs: &Vec4) -> Vec4 {
        Vec4 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
        }
    }

    pub fn sub(&self, rhs: &Vec4) -> Vec4 {
        Vec4 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w,
        }
    }

    pub fn dot(&self, rhs: &Vec4) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }

    pub fn len(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2) + self.z.powi(2) + self.w.powi(2)).sqrt()
    }

    pub fn norm(&self) -> Vec4 {
        self.scale(self.len().recip())
    }
}
