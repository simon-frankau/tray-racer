//
// Tool to trace paths using various step sizes in order to
// practically understand the convergence properties.
//

use clap::{Parser, ValueEnum};

use tray_racer_lib::vec4::*;
use tray_racer_lib::{CanvasConfig, EnvMap, Tracer};

const RESOLUTION: usize = 64;
const MIN_SIZE: f64 = 0.001;
const SCALE: f64 = 2.0;
const STEPS: usize = 7;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ResultFormat {
    /// Provide error compared to our best guess.
    Errors,
    /// Provide ratio of error to previous step size error.
    Ratios,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Value {
    /// Observe the direction from previous to last point.
    StepDir,
    /// Observe the direction, based on normal at last point.
    DerivDir,
    /// Observe the end point.
    Point,
}

/// Program to generate data to understand how the finite-difference
/// generated paths converge on the true paths as we adjust step
/// sizes.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// What analysis to do on the calculated values.
    #[arg(short, long, value_enum, default_value_t = ResultFormat::Errors)]
    output_mode: ResultFormat,
    /// The value to collect.
    #[arg(short, long, value_enum, default_value_t = Value::StepDir)]
    value: Value,
}

fn main() {
    let args = Args::parse();

    let tracer = Tracer {
        env_map_pos: EnvMap::new(),
        env_map_neg: EnvMap::new(),
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

    // Extract the values we care about from the result.
    let results = results
        .iter()
        .map(|v| {
            v.iter()
                .map(|result| match args.value {
                    Value::StepDir => result.step_dir,
                    Value::DerivDir => result.deriv_dir,
                    Value::Point => result.point,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // For each path and step size, work out the "distance" between
    // the shortest step result and that step's result.
    fn get_errors(path_results: &Vec<Vec4>) -> Vec<f64> {
        // First entry should be most precise.
        let base = path_results[0].norm();
        // Find difference against subsequent entries.
        path_results[1..]
            .iter()
            .map(|x| (x.norm().sub(base)).len())
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

    match args.output_mode {
        ResultFormat::Errors => display(&errors),
        ResultFormat::Ratios => display(&ratios),
    }
}

fn display(results: &[Vec<f64>]) {
    for result in results.iter() {
        println!(
            "{}",
            result
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
}
