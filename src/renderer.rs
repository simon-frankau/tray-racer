//
// renderer.rs: Display-independent rendering of the scene.
//

use std::path::Path;

use anyhow::Result;

use crate::vec4::*;

type Pixel = [u8; 4];

// Step size when doing finite-difference calculations.
const EPSILON: f64 = 1.0e-7;

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

////////////////////////////////////////////////////////////////////////
// TODO
//

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Function {
    Plane,
    PosCurve,
    NegCurve,
    Hole,
}

impl Function {
    fn label(&self) -> &'static str {
        match self {
            Function::Plane => "Plane",
            Function::PosCurve => "Positive curvature",
            Function::NegCurve => "Negative curvature",
            Function::Hole => "Wormhole",
        }
    }
}

pub struct Tracer2 {
    w_scale: f64,
    func: Function,
}

impl Tracer2 {
    pub fn new() -> Tracer2 {
        Tracer2 {
            w_scale: 0.25,
            func: Function::Plane,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        use egui::Color32;
        let mut needs_retex = false;
        needs_retex |= ui
            .add(egui::Slider::new(&mut self.w_scale, -1.0..=1.0).text("W scale"))
            .changed();
        needs_retex |= egui::ComboBox::from_label("Function")
            .selected_text(self.func.label())
            .show_ui(ui, |ui| {
                [
                    Function::Plane,
                    Function::PosCurve,
                    Function::NegCurve,
                    Function::Hole,
                ]
                .iter()
                .map(|x| ui.selectable_value(&mut self.func, *x, x.label()).changed())
                // Force evaluation of whole list.
                .fold(false, |a, b| a || b)
            })
            .inner
            .unwrap_or(false);
        if needs_retex {
            // TODO
        }
    }

    // Not a true distance, but the implicit surface function, where
    // the surface is all points where dist == 0.
    fn dist(&self, point: Point4) -> f64 {
        // If w_scale is zero, the implicit surface needs to be
        // special-cased to work.
        if self.w_scale.abs() <= EPSILON {
            return point.w;
        }

        // If the surface folds back, put a floor on the absolute
        // w_scale, otherwise multiple solutions get too close
        // together and the solver has a bad time.
        let mut w_scale = self.w_scale;
        if self.func == Function::Hole {
            w_scale = w_scale.signum() * w_scale.abs().max(0.02);
        }

        let (x, y, z, w) = (point.x, point.y, point.z, point.w / w_scale);
        match self.func {
            Function::Plane => (x + y + z) * 0.5 - w,
            Function::PosCurve => -(x * x + y * y + z * z) * 0.5 - w,
            Function::NegCurve => (x * x + y * y - z * z) * 0.5 - w,
            Function::Hole => x * x + y * y + z * z - w * w - 0.1,
        }
    }

    fn intersect_line(&self, point: Point4, direction: Dir4) -> Option<Point4> {
        // Newton-Raphson solver on dist(point + lambda direction)
        //
        // In practice, it's locally flat enough that a a single
        // iteration seems to suffice.
        const MAX_ITER: usize = 10;

        let mut lambda = 0.0;
        for _ in 0..MAX_ITER {
            let guess = point.add(direction.scale(lambda));
            let guess_val = self.dist(guess);
            if guess_val.abs() < EPSILON {
                return Some(guess);
            }

            let guess2 = point.add(direction.scale(lambda + EPSILON));
            let guess2_val = self.dist(guess2);

            let dguess_val = (guess2_val - guess_val) / EPSILON;

            lambda -= guess_val / dguess_val;
        }

        // Could fall back to binary chop, but as it generally seems
        // to converge in <= 2 iterations if there is a solution, this
        // seems excessive.
        None
    }

    // Intersect the surface with a line in the w-axis from the
    // point.
    fn project_vertical(&self, point: Point4) -> Option<Point4> {
        const VERTICAL: Dir4 = Dir4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        };
        self.intersect_line(point, VERTICAL)
    }

    // Take a step from p in direction delta, constrained to the
    // surface in direction norm.
    fn step(&self, p: Point4, delta: Dir4, norm: Dir4) -> Option<Point4> {
        let mut delta = delta.clone();
        // If curvature is extreme, there may be no intersection,
        // because the normal at p and the normal at the intersection
        // point are sufficiently different. We try again with a
        // smaller step.
        //
        // An example of extreme curvature is the "wormhole" surface
        // with w_scale around e.g. 0.01.
        const MAX_ITER: usize = 8;
        let mut new_p = None;
        let mut iter = 0;
        while new_p.is_none() && iter < MAX_ITER {
            new_p = self.intersect_line(p.add(delta), norm);
            delta = delta.scale(0.5);
            iter += 1;
        }
        new_p
    }

    // Calculate a normal vector using finite differences.
    fn normal_at(&self, p: Point4) -> Dir4 {
        let base_dist = self.dist(p);
        Dir4 {
            x: self.dist(Point4 {
                x: p.x + EPSILON,
                ..p
            }) - base_dist,
            y: self.dist(Point4 {
                y: p.y + EPSILON,
                ..p
            }) - base_dist,
            z: self.dist(Point4 {
                z: p.z + EPSILON,
                ..p
            }) - base_dist,
            w: self.dist(Point4 {
                w: p.w + EPSILON,
                ..p
            }) - base_dist,
        }
    }

    fn plot_path(&self, point: Point4, prev: Point4) -> Option<Point4> {
        let mut p = point.clone();
        let mut old_p = prev.clone();

        while p.x.abs() <= 1.0 && p.y.abs() <= 1.0 && p.z.abs() <= 1.0 {
            let delta = p.sub(old_p).norm().scale(RAY_STEP);
            let norm = self.normal_at(p).norm();

            if let Some(new_p) = self.step(p, delta, norm) {
                (p, old_p) = (new_p, p);
            } else {
                log::error!("plot_path could not extend path");
                return None;
            }
        }

        Some(p)
    }

    fn repath(&mut self, x0: f64, y0: f64, z0: f64, ray_dir: f64) -> Option<Point4> {
        let p = if let Some(p) = self.project_vertical(Point4 {
            x: x0,
            y: y0,
            z: z0,
            w: 1.0,
        }) {
            p
        } else {
            // No intersection point at ray_start. Give up.
            return None;
        };

        let ray_dir_rad = ray_dir * std::f64::consts::PI / 180.0;
        let delta = Dir4 {
            x: ray_dir_rad.sin() * RAY_STEP,
            y: ray_dir_rad.cos() * RAY_STEP,
            z: 0.0,
            w: 0.0,
        };

        // Take a step back, roughly, for initial previous point.
        let old_p = if let Some(p) = self.project_vertical(p.sub(delta)) {
            p
        } else {
            // No intersection point near ray_start. Give up.
            return None;
        };

        self.plot_path(p, old_p)
    }
}
