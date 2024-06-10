//
// Command-line based ray-tracing curved space renderer. Renders to a
// file.
//

use std::path::Path;

use anyhow::*;
use clap::Parser;
use image::imageops::flip_vertical_in_place;
use image::RgbaImage;

use tray_racer_lib::{CanvasConfig, EnvMap, Tracer};

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
}

////////////////////////////////////////////////////////////////////////
// Main code.
//

const WIDTH: usize = 1024;
const HEIGHT: usize = 768;

/* TODO: Check ranges on inputs
           .add(egui::Slider::new(&mut self.tracer.radius, -1.0..=1.0).text("Wormhole radius"))
           .add(egui::Slider::new(&mut self.tracer.w_scale, 0.1..=1.0).text("Smoothness"))
           .add(egui::Slider::new(&mut self.tracer.infinity, 1.0..=10.0).text("Infinity"))
   Other configurable values:
      image width
      image height
      ray step size
*/

fn main() -> Result<()> {
    let args = Args::parse();
    
    let env_map_pos = EnvMap::from(Path::new(&args.env_map_pos))?;
    let env_map_neg = EnvMap::from(Path::new(&args.env_map_neg))?;
    let tracer = Tracer {
        env_map_pos,
        env_map_neg,
        w_scale: 0.25,
        radius: 0.1,
        infinity: 4.0,
    };

    let fov = args.fov;
    assert!(20.0 <= fov && fov <= 160.0);
    let tilt = args.tilt;
    assert!(-90.0 <= tilt && tilt <= 90.0);
    let turn = args.turn;
    assert!(-180.0 <= turn && turn <= 180.0);
    let pan = args.pan;
    assert!(-180.0 <= pan && pan <= 180.0);

    let raw_image = tracer.render(
        &CanvasConfig {
            width: WIDTH,
            height: HEIGHT,
	    // When writing out an image, we'll always assume square pixels.
            aspect: 1.0,
            fov_degrees: fov,
        },
        tilt,
        turn,
        pan,
    );

    let mut image = RgbaImage::from_raw(WIDTH as u32, HEIGHT as u32, raw_image)
        .ok_or(anyhow!("Couldn't create image"))?;
    // OpenGL uses inverted vertical axis.
    flip_vertical_in_place(&mut image);
    image.save(args.output)?;

    Ok(())
}
