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

    // TODO: Work on -1..1 interval, etc.
    fn colour(&self, x: f64, y: f64) -> [u8; 4] {
        let img = &self.xmap.0;
        let (w, h) = img.dimensions();
        // Mapping semi-open interval [0..1) to [0..size).
        let ix = ((x * w as f64) as u32).min(w - 1);
        let iy = ((y * h as f64) as u32).min(h - 1);
        img.get_pixel(ix, iy).0
    }
}

pub struct CanvasConfig {
    pub width: usize,
    pub height: usize,
    pub aspect: f64,
    pub fov: f64,
}

pub fn render(conf: &CanvasConfig) -> Vec<u8> {
    // TODO: Still need to finalise and source-control these.
    let env_map = EnvMap::from(&Path::new("skyboxes/night-skyboxes/NightPath")).unwrap();

    // TODO: Dumb and wrong version to start with.
    let mut v = Vec::new();
    for y in 0..conf.height {
        for x in 0..conf.width {
            // Invert Y axis.
            v.extend(env_map.colour(
                x as f64 / conf.width as f64,
                1.0 - y as f64 / conf.height as f64,
            ));
        }
    }
    v
}
