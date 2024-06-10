//
// Command-line based ray-tracing curved space renderer. Renders to a
// file.
//

use std::path::Path;

use anyhow::*;
use clap::Parser;
use image::imageops::flip_vertical_in_place;
use image::RgbaImage;

use tray_racer_lib::{CanvasConfig, EnvMap, Tracer, RAY_STEP};

////////////////////////////////////////////////////////////////////////
// Command-line args

// TODO: Still need to finalise and source-control these.
const DEFAULT_ENV_MAP_POS: &str = "skyboxes/beach-skyboxes/HeartInTheSand";
const DEFAULT_ENV_MAP_NEG: &str = "skyboxes/night-skyboxes/PondNight";

/// Program to allow you to view distorted space
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory containing positive-w env maps
    #[arg(long, default_value_t = DEFAULT_ENV_MAP_POS.to_string())]
    env_map_pos: String,
    /// Directory containing negative-w env maps
    #[arg(long, default_value_t = DEFAULT_ENV_MAP_NEG.to_string())]
    env_map_neg: String,
    /// File to write the output to
    #[arg(short, long)]
    output: String,
    /// Output image width
    #[arg(short, long, default_value_t = 1024)]
    width: usize,
    /// Output image height
    #[arg(short, long, default_value_t = 768)]
    height: usize,
    /// Camera 'pitch', in degrees
    #[arg(long, default_value_t = 0.0)]
    tilt: f64,
    /// Camera 'yaw', in degrees
    #[arg(long, default_value_t = 0.0)]
    turn: f64,
    /// Angle turned around the wormhole, in degrees
    #[arg(long, default_value_t = 0.0)]
    pan: f64,
    /// Horizontal camera field of view, in degrees
    #[arg(long, default_value_t = 90.0)]
    fov: f64,
    /// Wormhole radius
    #[arg(long, default_value_t = 0.1)]
    radius: f64,
    /// How smooth the curve between sides of the wormhole are - width
    /// of the wormhole in the fourth dimension
    #[arg(long, default_value_t = 0.25)]
    smoothness: f64,
    /// The 4-distance at which we assume no further curvature occurs
    #[arg(long, default_value_t = 4.0)]
    infinity: f64,
    /// Path-tracing step size
    #[arg(short, long, default_value_t = RAY_STEP)]
    step_size: f64,
}

////////////////////////////////////////////////////////////////////////
// Main code.
//

/* TODO: Other configurable values:
      ray step size
*/

fn main() -> Result<()> {
    let args = Args::parse();

    let env_map_pos = EnvMap::from(Path::new(&args.env_map_pos))?;
    let env_map_neg = EnvMap::from(Path::new(&args.env_map_neg))?;
    let w_scale = args.smoothness;
    assert!(0.1 <= w_scale && w_scale <= 1.0);
    let radius = args.radius;
    assert!(-1.0 <= radius && radius <= 1.0);
    let infinity = args.infinity;
    assert!(1.0 <= infinity && infinity <= 10.0);

    let tracer = Tracer {
        env_map_pos,
        env_map_neg,
        w_scale,
        radius,
        infinity,
    };

    let width = args.width;
    assert!(16 <= width && width <= 16384);
    let height = args.height;
    assert!(16 <= height && height <= 16384);

    let fov_degrees = args.fov;
    assert!(20.0 <= fov_degrees && fov_degrees <= 160.0);
    let tilt = args.tilt;
    assert!(-90.0 <= tilt && tilt <= 90.0);
    let turn = args.turn;
    assert!(-180.0 <= turn && turn <= 180.0);
    let pan = args.pan;
    assert!(-180.0 <= pan && pan <= 180.0);
    let step_size = args.step_size;
    assert!(0.001 <= step_size && step_size <= 0.1);

    let raw_image = tracer.render(
        &CanvasConfig {
            width,
            height,
            // When writing out an image, we'll always assume square pixels.
            aspect: 1.0,
            fov_degrees,
        },
        tilt,
        turn,
        pan,
        step_size,
    );

    let mut image = RgbaImage::from_raw(width as u32, height as u32, raw_image)
        .ok_or(anyhow!("Couldn't create image"))?;
    // OpenGL uses inverted vertical axis.
    flip_vertical_in_place(&mut image);
    image.save(args.output)?;

    Ok(())
}
