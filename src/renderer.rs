//
// Display-independent rendering of the scene.
//

use std::path::Path;

use anyhow::Result;

type ImagePair = (image::RgbaImage, image::RgbaImage);

struct EnvMap {
    xmap: ImagePair,
    ymap: ImagePair,
    zmap: ImagePair,
}

impl EnvMap {
    fn from(path: &Path) -> Result<EnvMap> {
        let open = |s: &str| image::open(path.join(s)).map(|img| img.into_rgba8());

        Ok(EnvMap {
            xmap: (open("negx.jpg")?, open("posx.jpg")?),
            ymap: (open("negy.jpg")?, open("posy.jpg")?),
            zmap: (open("negz.jpg")?, open("posz.jpg")?),
        })
    }

    fn colour_face(&self, x: f64, y: f64, img: &image::RgbaImage) -> [u8; 4] {
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

    fn colour(&self, x: f64, y: f64, z: f64) -> [u8; 4] {
        if z.abs() > x.abs() && z.abs() > y.abs() {
            self.colour_face(x, y, &self.xmap.0)
        } else {
            [0, 0, 0, 255]
        }
    }
}

pub struct CanvasConfig {
    pub width: usize,
    pub height: usize,
    pub aspect: f64,
    pub fov: f64,
}

pub fn render(conf: &CanvasConfig, tilt: f64, turn: f64) -> Vec<u8> {
    // TODO: Still need to finalise and source-control these.
    let env_map = EnvMap::from(Path::new("skyboxes/night-skyboxes/NightPath")).unwrap();

    let tilt_rad = -tilt * std::f64::consts::PI / 180.0;
    let tilt_cos = tilt_rad.cos();
    let tilt_sin = tilt_rad.sin();

    let turn_rad = -turn * std::f64::consts::PI / 180.0;
    let turn_cos = turn_rad.cos();
    let turn_sin = turn_rad.sin();

    // Invariants: start + step * (size - 1)/2 = 0.
    let x_range = conf.fov * 2.0;
    let x_step = -x_range / conf.width as f64;
    let x_start = -0.5 * x_step * (conf.width - 1) as f64;

    let y_range = x_range * conf.aspect;
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

            v.extend(env_map.colour(t2x, t2y, t2z));
            x += x_step;
        }
        y += y_step;
    }
    v
}
