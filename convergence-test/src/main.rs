//
// Tool to trace paths using various step sizes in order to
// practically understand the convergence properties.
//

use std::path::Path;

// TODO use anyhow::*;

use tray_racer_lib::{CanvasConfig, EnvMap, Tracer};

const RESOLUTION: usize = 64;

const ENV_MAP_PATH_POS: &str = "skyboxes/beach-skyboxes/HeartInTheSand";
const ENV_MAP_PATH_NEG: &str = "skyboxes/night-skyboxes/PondNight";

fn main() {
    let env_map_pos = EnvMap::from(Path::new(ENV_MAP_PATH_POS)).unwrap();
    let env_map_neg = EnvMap::from(Path::new(ENV_MAP_PATH_NEG)).unwrap();

    let tracer = Tracer {
        env_map_pos,
        env_map_neg,
        w_scale: 0.25,
        radius: 0.25,
        infinity: 4.0,
    };

    let conf = CanvasConfig {
        width: RESOLUTION,
        height: RESOLUTION,
        aspect: 1.0,
        fov_degrees: 90.0,
    };

    let tex_data = tracer.render(&conf, 0.0, 0.0, 0.0);

    // Avoid too much optimisation.
    println!("{}", tex_data.len());
}
