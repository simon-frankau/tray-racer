//
// Tool to trace paths using various step sizes in order to
// practically understand the convergence properties.
//

use std::path::Path;

use tray_racer_lib::{CanvasConfig, EnvMap, TraceStats, Tracer};

const RESOLUTION: usize = 64;
const MIN_SIZE: f64 = 0.001;
const SCALE: f64 = 2.0;
const STEPS: usize = 7;

// TODO: These maps are unnecessary and loading them slows down the code...
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

    // Find the result direction vectors for various step sizes (grouped by path).
    let mut results = (0..RESOLUTION.pow(2))
        .map(|_| Vec::new())
        .collect::<Vec<_>>();
    let mut size = MIN_SIZE;
    for _ in 0..STEPS {
        eprintln!("Step size: {}", size);
        let step_result = tracer.render_stats(&conf, size);
        for (path_results, path_result) in results.iter_mut().zip(step_result.into_iter()) {
            path_results.push(path_result);
        }
        size *= SCALE;
    }

    // For each path and step size, work out the "distance" between
    // the shortest step result and that step's result.
    fn get_errors(path_results: &Vec<TraceStats>) -> Vec<f64> {
        // First entry should be most precise.
        let base = path_results[0].dir.norm();
        // Find difference against subsequent entries.
        path_results[1..]
            .iter()
            .map(|x| (x.dir.norm().sub(base)).len())
            .collect::<Vec<_>>()
    }

    let errors = results.iter().map(get_errors).collect::<Vec<_>>();

    // Then, calcuate the ratio between successive terms, which should
    // roughly represent the scaling of the error term as we scale the
    // step size.
    fn get_ratios(errors: &Vec<f64>) -> Vec<f64> {
        errors
            .iter()
            .zip(errors.iter().skip(1))
            .map(|(small, big)| big / small)
            .collect::<Vec<_>>()
    }

    let ratios = errors.iter().map(get_ratios).collect::<Vec<_>>();

    // Finally, display our results.
    for ratio in ratios.iter() {
        println!(
            "{}",
            ratio
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
}
