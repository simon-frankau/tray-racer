//
// renderer.rs: Display-independent rendering of the scene.
//

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;

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
    // Stub envmap for tests etc.
    pub fn new() -> EnvMap {
        let img = image::RgbaImage::new(1, 1);
        let img_pair = (img.clone(), img.clone(), false);
        EnvMap {
            xmap: img_pair.clone(),
            ymap: img_pair.clone(),
            zmap: img_pair.clone(),
        }
    }

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

// Ray stepping size.
pub const RAY_STEP: f64 = 0.01;

pub struct Tracer {
    pub env_map_pos: EnvMap,
    pub env_map_neg: EnvMap,
    // How we scale w in the equation. effectively controls the depth
    // of the wormhole.
    pub w_scale: f64,
    // Radius of the wormhole.
    pub radius: f64,
    // Radius beyond which we assume that space is effectively flat,
    // so that the direction will not change further, and we can look
    // it up in the environment map.
    pub infinity: f64,
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

////////////////////////////////////////////////////////////////////////
// Fixed-step renderer
//

impl Tracer {
    // Render a whole scene by tracing all the rays in the canvas.
    pub fn render(
        &self,
        conf: &CanvasConfig,
        tilt: f64,
        turn: f64,
        pan: f64,
        step_size: Option<f64>,
    ) -> Vec<u8> {
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

        // Set the camera position.
        let pan_rad = pan * std::f64::consts::PI / 180.0;
        let pan_sin = pan_rad.sin();
        let pan_cos = pan_rad.cos();
        let origin = Point4 {
            x: pan_sin,
            y: 0.0,
            z: -pan_cos,
            w: 1.0,
        };

        let render_row = |y: f64| {
            let mut v = Vec::new();
            let mut x = x_start;
            for _ in 0..conf.width {
                let z = 1.0;

                let tx = x;
                let ty = y * tilt_cos + z * tilt_sin;
                let tz = -y * tilt_sin + z * tilt_cos;

                let t2x = tx * turn_cos + tz * turn_sin;
                let t2y = ty;
                let t2z = -tx * turn_sin + tz * turn_cos;

                // And rotate the looking direction to be centered around (0, 0, 0)
                let dir = Dir4 {
                    x: t2x * pan_cos - t2z * pan_sin,
                    y: t2y,
                    z: t2x * pan_sin + t2z * pan_cos,
                    w: 0.0,
                };

                v.extend(if let Some(step_size) = step_size {
                    self.trace(origin, dir, step_size)
                } else {
                    self.trace_adaptive(origin, dir)
                });
                x += x_step;
            }
            v
        };

        (0..conf.height)
            .into_par_iter()
            .map(|y| render_row(y_start + y as f64 * y_step))
            .flatten()
            .collect::<Vec<u8>>()
    }

    // Trace a single ray.
    fn trace(&self, p: Point4, dir: Dir4, step_size: f64) -> Pixel {
        let delta = dir.norm().scale(step_size);
        let mut p = self.project_vertical(p).unwrap();
        let mut old_p = self.project_vertical(p.sub(delta)).unwrap();

        while p.len() < self.infinity {
            let delta = p.sub(old_p).norm().scale(step_size);
            let norm = self.normal_at(p).norm();

            if let Some(new_p) = self.step(p, delta, norm) {
                (p, old_p) = (new_p, p);
            } else {
                panic!("trace_aux could not extend path");
            }
        }

        let final_dir = p.sub(old_p);
        if final_dir.w > 0.0 {
            self.env_map_pos.colour(final_dir)
        } else {
            self.env_map_neg.colour(final_dir)
        }
    }

    // Take a step from p in direction delta, constrained to the
    // surface in direction norm.
    fn step(&self, p: Point4, delta: Dir4, norm: Dir4) -> Option<Point4> {
        let mut delta = delta;
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
            // If it takes too many iterations, we're probably best
            // off taking a smaller step, so set the Newton-Raphson
            // convergence iterations low.
            new_p = self.intersect_line(p.add(delta), norm, 3);
            delta = delta.scale(0.5);
            iter += 1;
        }
        new_p
    }

    // Not a true distance, but the implicit surface function, where
    // the surface is all points where dist == 0.
    fn dist(&self, point: Point4) -> f64 {
        // If w_scale is zero, the implicit surface needs to be
        // special-cased to work.
        if self.w_scale.abs() <= EPSILON {
            return point.w;
        }

        let w_scale = self.w_scale.signum() * self.w_scale.abs().max(0.02);
        let (x, y, z, w) = (point.x, point.y, point.z, point.w / w_scale);
        x * x + y * y + z * z - w * w - self.radius
    }

    fn intersect_line(&self, point: Point4, direction: Dir4, max_iters: usize) -> Option<Point4> {
        // Newton-Raphson solver on dist(point + lambda direction)
        let mut lambda = 0.0;
        for _ in 0..max_iters {
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
        // Plenty of iterations to converge, since the starting point
        // may be far from the intersection.
        self.intersect_line(point, VERTICAL, 10)
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
}

////////////////////////////////////////////////////////////////////////
// Adaptive tracer
//

const TARGET_NORM_DIFF: f64 = 1.0e-4;
const BASE_ADAPTIVE_STEP: f64 = 0.01;
const MAX_ADAPTIVE_STEP: f64 = 0.1;

impl Tracer {
    // Trace a single ray.
    fn trace_adaptive(&self, p: Point4, dir: Dir4) -> Pixel {
        // We'll adapt the step size, so that the optimal size from
        // the previous step is used for the next one.
        let mut step_size = BASE_ADAPTIVE_STEP;
        let delta = dir.norm().scale(step_size);
        let mut p = self.project_vertical(p).unwrap();
        let mut norm = self.normal_at(p).norm();
        let mut old_p = self.project_vertical(p.sub(delta)).unwrap();

        while p.len() < self.infinity {
            let delta = p.sub(old_p).norm();
            ((p, norm), old_p) = (self.step_adaptive(p, delta, norm, &mut step_size), p);
        }

        let final_dir = p.sub(old_p);
        if final_dir.w > 0.0 {
            self.env_map_pos.colour(final_dir)
        } else {
            self.env_map_neg.colour(final_dir)
        }
    }

    // Take a step from p in direction delta, constrained to the
    // surface in direction norm.
    fn step_adaptive(
        &self,
        p: Point4,
        delta: Dir4,
        norm: Dir4,
        step_size: &mut f64,
    ) -> (Point4, Dir4) {
        let delta = delta.scale(*step_size);
        let base = p.add(delta);
        let (projection, new_p) = self.intersect_line_adaptive(base, norm);

        // Now, calculate the next step size.
        let new_norm = self.normal_at(new_p).norm();
        let other_p = base.add(new_norm.scale(projection));
        let actual_norm_diff = new_p.sub(other_p).len() / *step_size;
        *step_size = (*step_size * TARGET_NORM_DIFF / actual_norm_diff).min(MAX_ADAPTIVE_STEP);

        (new_p, new_norm)
    }

    fn intersect_line_adaptive(&self, point: Point4, direction: Dir4) -> (f64, Point4) {
        // Newton-Raphson solver on dist(point + lambda direction)
        const MAX_ITERS: usize = 3;
        let mut lambda = 0.0;
        for _ in 0..MAX_ITERS {
            let guess = point.add(direction.scale(lambda));
            let guess_val = self.dist(guess);
            if guess_val.abs() < EPSILON {
                return (lambda, guess);
            }

            let guess2 = point.add(direction.scale(lambda + EPSILON));
            let guess2_val = self.dist(guess2);

            let dguess_val = (guess2_val - guess_val) / EPSILON;

            lambda -= guess_val / dguess_val;
        }

        panic!("step_adaptive could not extend path");
    }
}

////////////////////////////////////////////////////////////////////////
// Renderer that returns ray-level stats, for understanding
// convergence behaviour.
//

#[derive(Debug)]
pub struct RayStats {
    pub step_dir: Dir4,
    pub deriv_dir: Dir4,
    pub point: Dir4,
    pub len: f64,
}

impl Tracer {
    // Render a whole scene by tracing all the rays in the canvas.
    pub fn render_ray_stats(&self, conf: &CanvasConfig, step_size: f64) -> Vec<RayStats> {
        let fov_rad = conf.fov_degrees * std::f64::consts::PI / 180.0;
        let fov = (fov_rad * 0.5).tan();

        // Invariants: start + step * (size - 1)/2 = 0.
        let x_range = fov * 2.0;
        let x_step = -x_range / conf.width as f64;
        let x_start = -0.5 * x_step * (conf.width - 1) as f64;

        let y_range = x_range * conf.aspect * conf.height as f64 / conf.width as f64;
        let y_step = -y_range / conf.height as f64;
        let y_start = -0.5 * y_step * (conf.height - 1) as f64;

        // Set the camera position.
        let origin = Point4 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
            w: 1.0,
        };

        let mut v = Vec::new();
        let mut y = y_start;
        for _ in 0..conf.height {
            let mut x = x_start;
            for _ in 0..conf.width {
                let dir = Dir4 {
                    x,
                    y,
                    z: 1.0,
                    w: 0.0,
                };
                v.push(self.trace_ray_stats(origin, dir, step_size));
                x += x_step;
            }
            y += y_step;
        }
        v
    }

    // Trace a single ray, collecting stats.
    fn trace_ray_stats(&self, p: Point4, dir: Dir4, step_size: f64) -> RayStats {
        let delta = dir.norm().scale(step_size);
        let mut p = self.project_vertical(p).unwrap();
        let mut old_p = self.project_vertical(p.sub(delta)).unwrap();

        let mut len = 0.0;
        while p.len() < self.infinity {
            let delta = p.sub(old_p).norm().scale(step_size);
            let norm = self.normal_at(p).norm();

            if let Some(new_p) = self.step(p, delta, norm) {
                (p, old_p) = (new_p, p);
            } else {
                panic!("trace_aux could not extend path");
            }

            len += p.sub(old_p).len();
        }

        let step_dir = p.sub(old_p);
        let norm = self.normal_at(p).norm();
        let deriv_dir = step_dir.sub(norm.scale(step_dir.dot(norm)));
        let point = self.clip_to_radius(p, old_p);

        RayStats {
            step_dir,
            deriv_dir,
            point,
            len,
        }
    }

    // Excessively precise way to clip the line to end on the given
    // radius, so that the clipping doesn't distort the error
    // calculation.
    fn clip_to_radius(&self, p: Point4, prev_p: Point4) -> Point4 {
        // I <heart/> basic Newton-Raphson.
        let delta = p.sub(prev_p);
        let mut lambda = 0.0;
        loop {
            let guess = prev_p.add(delta.scale(lambda));
            let radius_diff = guess.dot(guess) - self.infinity.powi(2);
            if radius_diff.abs() < EPSILON {
                return guess;
            }
            // d radius / d lambda = d radius / d guess * d guess/ lambda
            let deriv = 2.0 * guess.dot(delta);

            lambda -= radius_diff / deriv;
        }
    }
}

////////////////////////////////////////////////////////////////////////
// Renderer that returns step-level stats, for further understanding
// convergence behaviour.
//

#[derive(Debug)]
pub struct StepStats {
    pub step_num: usize,
    pub len: f64,
    pub error: f64,
    pub curvature: f64,
    pub dcurve: f64,
    pub norm_diff: f64,
}

// Number of sub-steps per step to get the 'accurate' alternative to
// calculate error with.
const TRACE_STEP_MULT: usize = 10;

impl Tracer {
    // Render a whole scene by tracing all the rays in the canvas.
    pub fn render_step_stats(&self, conf: &CanvasConfig, step_size: f64) -> Vec<StepStats> {
        let fov_rad = conf.fov_degrees * std::f64::consts::PI / 180.0;
        let fov = (fov_rad * 0.5).tan();

        // Invariants: start + step * (size - 1)/2 = 0.
        let x_range = fov * 2.0;
        let x_step = -x_range / conf.width as f64;
        let x_start = -0.5 * x_step * (conf.width - 1) as f64;

        let y_range = x_range * conf.aspect * conf.height as f64 / conf.width as f64;
        let y_step = -y_range / conf.height as f64;
        let y_start = -0.5 * y_step * (conf.height - 1) as f64;

        // Set the camera position.
        let origin = Point4 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
            w: 1.0,
        };

        let mut v = Vec::new();
        let mut y = y_start;
        for _ in 0..conf.height {
            let mut x = x_start;
            for _ in 0..conf.width {
                let dir = Dir4 {
                    x,
                    y,
                    z: 1.0,
                    w: 0.0,
                };
                v.append(&mut self.trace_step_stats(origin, dir, step_size));
                x += x_step;
            }
            y += y_step;
        }
        v
    }

    // Trace a single ray, collecting stats.
    fn trace_step_stats(&self, p: Point4, dir: Dir4, step_size: f64) -> Vec<StepStats> {
        let mut stats = Vec::new();

        let delta = dir.norm().scale(step_size);
        let mut p = self.project_vertical(p).unwrap();
        let mut old_p = self.project_vertical(p.sub(delta)).unwrap();
        let mut old_norm = self.normal_at(old_p).scale(EPSILON.recip());

        let mut step_num = 0;
        while p.len() < self.infinity {
            let delta = p.sub(old_p).norm().scale(step_size);
            let norm = self.normal_at(p).scale(EPSILON.recip());
            let nnorm = norm.norm();

            let (saved_p, saved_old_p) = (p, old_p);

            if let Some(new_p) = self.step(p, delta, nnorm) {
                (p, old_p) = (new_p, p);
            } else {
                panic!("trace_aux could not extend path");
            }

            let len = p.sub(old_p).len();

            // And do a more accurate step to compare with.
            let alt_p = {
                let (mut p, mut old_p) = (saved_p, saved_old_p);
                let step_size = step_size / TRACE_STEP_MULT as f64;
                for _ in 0..TRACE_STEP_MULT {
                    let delta = p.sub(old_p).norm().scale(step_size);
                    let nnorm = self.normal_at(p).norm();

                    if let Some(new_p) = self.step(p, delta, nnorm) {
                        (p, old_p) = (new_p, p);
                    } else {
                        panic!("trace_step_stats could not extend path");
                    }
                }
                p
            };
            let error = p.sub(alt_p).len() / len;

            // And find the curvatature, as how far the new point is
            // away from the old one (normalised), in the direction of
            // the normal.
            let curvature = p.sub(old_p).norm().dot(norm) / len;

            // Find the change in curvature over the step.
            let dcurve = old_norm.sub(norm).len() / len;
            old_norm = norm;

            // Find difference in projection based on normal.
            let new_norm = self.normal_at(p).norm();
            let other_p_base = old_p.add(delta);
            let projection = nnorm.dot(p.sub(other_p_base));
            let other_p = other_p_base.add(new_norm.scale(projection));
            let norm_diff = p.sub(other_p).len() / len;

            stats.push(StepStats {
                step_num,
                len,
                error,
                curvature,
                dcurve,
                norm_diff,
            });
            step_num += 1;
        }

        stats
    }
}
