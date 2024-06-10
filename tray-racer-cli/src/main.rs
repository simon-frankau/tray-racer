//
// Command-line based ray-tracing curved space renderer. Renders to a
// file.
//

use std::path::Path;

use anyhow::*;
use clap::Parser;
use image::RgbaImage;
use image::imageops::flip_vertical_in_place;

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

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;

struct Drawable {
    tracer: Tracer,
    tilt: f64,
    turn: f64,
    pan: f64,
    fov: f64,
}

impl Drawable {
    fn new(env_map_path_pos: &Path, env_map_path_neg: &Path) -> Drawable {
        let env_map_pos = EnvMap::from(env_map_path_pos).unwrap();
        let env_map_neg = EnvMap::from(env_map_path_neg).unwrap();

        Drawable {
            tracer: Tracer {
                env_map_pos,
                env_map_neg,
                w_scale: 0.25,
                radius: 0.1,
                infinity: 4.0,
            },
            tilt: 0.0,
            turn: 0.0,
            pan: 0.0,
            fov: 90.0,
        }
    }

    /* TODO: Check ranges on inputs
               .add(egui::Slider::new(&mut self.fov, 20.0..=160.0).text("Field of view"))
               .add(egui::Slider::new(&mut self.tilt, -90.0..=90.0).text("Tilt"))
               .add(egui::Slider::new(&mut self.turn, -180.0..=180.0).text("Turn"))
               .add(egui::Slider::new(&mut self.pan, -180.0..=180.0).text("Pan"))
               .add(egui::Slider::new(&mut self.tracer.radius, -1.0..=1.0).text("Wormhole radius"))
               .add(egui::Slider::new(&mut self.tracer.w_scale, 0.1..=1.0).text("Smoothness"))
               .add(egui::Slider::new(&mut self.tracer.infinity, 1.0..=10.0).text("Infinity"))
    */

    fn render(&self) -> Vec<u8> {
        // TODO
        let (w, h) = (1024, 768);

        self.tracer.render(
            &CanvasConfig {
                width: w,
                height: h,
                aspect: 1.0,
                fov_degrees: self.fov,
            },
            self.tilt,
            self.turn,
            self.pan,
        )
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let drawable = Drawable::new(Path::new(&args.env_map_pos), Path::new(&args.env_map_neg));

    let raw_image = drawable.render();
    let mut image =
        RgbaImage::from_raw(WIDTH, HEIGHT, raw_image).ok_or(anyhow!("Couldn't create image"))?;
    // OpenGL uses inverted vertical axis.
    flip_vertical_in_place(&mut image);
    image.save(args.output)?;

    Ok(())
}
