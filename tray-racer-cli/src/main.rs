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
}

////////////////////////////////////////////////////////////////////////
// Main code.
//

const WIDTH: usize = 1024;
const HEIGHT: usize = 768;

/* TODO: Check ranges on inputs
           .add(egui::Slider::new(&mut self.fov, 20.0..=160.0).text("Field of view"))
           .add(egui::Slider::new(&mut self.tilt, -90.0..=90.0).text("Tilt"))
           .add(egui::Slider::new(&mut self.turn, -180.0..=180.0).text("Turn"))
           .add(egui::Slider::new(&mut self.pan, -180.0..=180.0).text("Pan"))
           .add(egui::Slider::new(&mut self.tracer.radius, -1.0..=1.0).text("Wormhole radius"))
           .add(egui::Slider::new(&mut self.tracer.w_scale, 0.1..=1.0).text("Smoothness"))
           .add(egui::Slider::new(&mut self.tracer.infinity, 1.0..=10.0).text("Infinity"))
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

    let tilt = 0.0;
    let turn = 0.0;
    let pan = 0.0;
    let fov = 90.0;

    let raw_image = tracer.render(
        &CanvasConfig {
            width: WIDTH,
            height: HEIGHT,
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
