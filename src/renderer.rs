//
// renderer.rs: Display-independent rendering of the scene.
//

use std::path::Path;

use anyhow::Result;

use crate::vec4::*;

type Pixel = [u8; 4];

////////////////////////////////////////////////////////////////////////
// Environment map.
//

// Last bool is "is vertical?". Vertical pair flips in a different
// way.
type ImagePair = (image::RgbaImage, image::RgbaImage, bool);

pub struct EnvMap {
    xmap: ImagePair,
    ymap: ImagePair,
    zmap: ImagePair,
}

// Build and sample a cubic environment map. Has various axis tweaks
// to match the environment maps we use.
impl EnvMap {
    pub fn from(path: &Path) -> Result<EnvMap> {
        let open = |s: &str| image::open(path.join(s)).map(|img| img.into_rgba8());

        Ok(EnvMap {
            xmap: (open("negx.jpg")?, open("posx.jpg")?, false),
            ymap: (open("negy.jpg")?, open("posy.jpg")?, true),
            zmap: (open("negz.jpg")?, open("posz.jpg")?, false),
        })
    }

    // Coordinates should be normalised to have largest direction in z.
    fn colour_face(&self, x: f64, y: f64, z: f64, img_pair: &ImagePair) -> Pixel {
        // Get image for appropriate direction.
        let img = if z > 0.0 { &img_pair.0 } else { &img_pair.1 };
        // Normalise coordinates. Does some flipping as needed to make
        // the faces' edges match up.
        let (x, y) = if img_pair.2 {
            (x / z.abs(), y / z)
        } else {
            (x / z, y / z.abs())
        };
        // Convert face coordinates -1..1 to texture coordinates 0..1.
        let x = 0.5 * (x + 1.0);
        let y = 0.5 * (y + 1.0);
        // Then scale to pixel coordinates.
        let (w, h) = img.dimensions();
        // Mapping semi-open interval [0..1) to [0..size).
        let ix = ((x * w as f64) as u32).min(w - 1);
        let iy = ((y * h as f64) as u32).min(h - 1);
        img.get_pixel(ix, iy).0
    }

    // Ignores the w component.
    fn colour(&self, dir: Dir4) -> Pixel {
        let (ax, ay, az) = (dir.x.abs(), dir.y.abs(), dir.z.abs());
        // We do some coordinate flipping to make sure the faces'
        // edges match up.
        if az > ax && az > ay {
            self.colour_face(dir.x, dir.y, dir.z, &self.xmap)
        } else if ax > ay {
            self.colour_face(dir.z, dir.y, -dir.x, &self.zmap)
        } else {
            self.colour_face(-dir.z, -dir.x, dir.y, &self.ymap)
        }
    }
}

////////////////////////////////////////////////////////////////////////
// Tracer/renderer
//

// Radius beyond which we assume that space is effectively flat,
// so that the direction will not change further, and we can look
// it up in the environment map.
const RADIUS: f64 = 4.0;

// Ray stepping size.
const RAY_STEP: f64 = 0.01;

pub struct Tracer {
    pub env_map: EnvMap,
}

// Configuration for the screen we expect. `render` then returns an
// array of pixels that would fill in that canvas.
pub struct CanvasConfig {
    // Width and height in pixels.
    pub width: usize,
    pub height: usize,
    // Aspect ratio in the form of height of a pixel / width of a pixel.
    pub aspect: f64,
    // Field of view, in degrees.
    pub fov_degrees: f64,
}

impl Tracer {
    //`Render a whole scene by tracing all the rays in the canvas.
    pub fn render(&self, conf: &CanvasConfig, tilt: f64, turn: f64) -> Vec<u8> {
        let tilt_rad = -tilt * std::f64::consts::PI / 180.0;
        let tilt_cos = tilt_rad.cos();
        let tilt_sin = tilt_rad.sin();

        let turn_rad = -turn * std::f64::consts::PI / 180.0;
        let turn_cos = turn_rad.cos();
        let turn_sin = turn_rad.sin();

        let fov_rad = conf.fov_degrees * std::f64::consts::PI / 180.0;
        let fov = (fov_rad * 0.5).tan();

        // Invariants: start + step * (size - 1)/2 = 0.
        let x_range = fov * 2.0;
        let x_step = -x_range / conf.width as f64;
        let x_start = -0.5 * x_step * (conf.width - 1) as f64;

        let y_range = x_range * conf.aspect * conf.height as f64 / conf.width as f64;
        let y_step = -y_range / conf.height as f64;
        let y_start = -0.5 * y_step * (conf.height - 1) as f64;

        let mut v = Vec::new();
        let mut y = y_start;
        for _ in 0..conf.height {
            let mut x = x_start;
            for _ in 0..conf.width {
                let z = 1.0;

                let tx = x;
                let ty = y * tilt_cos + z * tilt_sin;
                let tz = -y * tilt_sin + z * tilt_cos;

                let t2x = tx * turn_cos + tz * turn_sin;
                let t2y = ty;
                let t2z = -tx * turn_sin + tz * turn_cos;

                v.extend(self.trace(
                    Point4 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 0.0,
                    },
                    Dir4 {
                        x: t2x,
                        y: t2y,
                        z: t2z,
                        w: 0.0,
                    },
                ));
                x += x_step;
            }
            y += y_step;
        }
        v
    }

    // Trace a single ray.
    fn trace(&self, p: Point4, dir: Dir4) -> Pixel {
        let mut p = p;
        let delta = dir.norm().scale(RAY_STEP);

        while p.len() < RADIUS {
            p = p.add(delta);
        }

        // TODO: Should be dir, but using p ensures the previous loop
        // is evaluated.
        self.env_map.colour(p)
    }
}
