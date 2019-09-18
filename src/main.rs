use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct RunContents {
    Ok: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RunKey {
    benchmark_key: String,
    toolchain: Toolchain,
}

#[derive(Debug, Deserialize)]
struct Toolchain {
    spec: String,
}

#[derive(Debug, Deserialize)]
struct RunPlan {
    generated_at: String,
    key: RunKey,
    contents: RunContents,
}

#[derive(Debug, Deserialize)]
struct MeasurementDetails {
    point_estimate: f64,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct MeasurementValues {
    Median: MeasurementDetails,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct MeasurementContents {
    Ok: HashMap<String, MeasurementValues>,
}

#[derive(Debug, Deserialize)]
struct MeasurementKey {
    binary_hash: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct Measurement {
    generated_at: String,
    key: MeasurementKey,
    contents: MeasurementContents,
}

/// Running: lolbench_parser <data_dir> <base_toolchain> <new_toolchain> <event>
///
/// data_dir - location of data crated by lolbench; it must contain results from both toolchains
/// base_toolchain - name of base toolchain
/// new_toolchain - name of toolchain which results will be compared
/// event - one of [instructions, nanoseconds]
fn main() {
    let mut run_plans: Vec<RunPlan> = Vec::new();
    let mut measurements: Vec<Measurement> = Vec::new();
    let mut benchmarks: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut results: Vec<(String, f64, f64, f64)> = Vec::new();
    let data_dir = env::args().nth(1).unwrap();
    let base_toolchain = env::args().nth(2).unwrap();
    let new_toolchain = env::args().nth(3).unwrap();
    let event = env::args().nth(4).unwrap();

    // Populate list of benchmarks
    for entry in fs::read_dir(format!("{}/run-plans", data_dir))
        .unwrap_or_else(|_| panic!("Could not read {}", data_dir))
    {
        let mut file = fs::File::open(entry.unwrap().path()).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        run_plans.push(serde_json::from_str(&contents).unwrap());
    }

    // Populate list of results
    for entry in fs::read_dir(format!("{}/measurements", data_dir))
        .unwrap_or_else(|_| panic!("Could not read {}", data_dir))
    {
        let mut file = fs::File::open(entry.unwrap().path()).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        measurements.push(serde_json::from_str(&contents).unwrap());
    }

    // Map benchmarks with their results
    for run_plan in run_plans {
        let name = run_plan.key.benchmark_key;
        let mut median = 0.0;

        for measurement in &measurements {
            if run_plan.contents.Ok == measurement.key.binary_hash {
                median = measurement.contents.Ok[&event].Median.point_estimate;
            }
        }

        if let Some(toolchains) = benchmarks.get_mut(&name) {
            toolchains.insert(run_plan.key.toolchain.spec.to_string(), median);
        } else {
            let mut toolchain = HashMap::new();
            toolchain.insert(run_plan.key.toolchain.spec.to_string(), median);
            benchmarks.insert(name, toolchain);
        }
    }

    // Sort by difference
    for benchmark in benchmarks {
        let difference = (benchmark.1[&new_toolchain] - benchmark.1[&base_toolchain])
            / benchmark.1[&base_toolchain]
            * 100.0;
        results.push((
            benchmark.0,
            benchmark.1[&base_toolchain],
            benchmark.1[&new_toolchain],
            difference,
        ));
    }
    results.sort_by(|a, b| b.3.abs().partial_cmp(&a.3.abs()).unwrap());

    // Print everything
    println!(
        "Benchmark name | {} | {} | % diff",
        base_toolchain, new_toolchain
    );
    for result in results {
        println!("{} | {} | {} | {}", result.0, result.1, result.2, result.3);
    }
}
