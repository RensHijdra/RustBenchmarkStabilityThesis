#![allow(unused)]
use std::fs;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::process::Command;

use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;
use regex::Regex;
use serde::{Deserialize, Serialize};

use project::{get_workdir_for_project, read_target_projects, Project, TargetProject};
use crate::probe::{create_named_probe_for_adresses, delete_probe, find_probe_addresses};
use crate::project::BenchFile;

mod probe;
mod project;

// Enable setting probes and traces
// sudo sysctl kernel.perf_event_paranoid=-1 -w
// sudo mount -o remount,groups /sys/kernel/tracing/
// sudo mount -o remount,mode=755 /sys/kernel/tracing
// sudo chgrp -R tracing /sys/kernel/tracing/
// sudo chmod -R g+rw /sys/kernel/tracing/

// sudo groupadd tracing
// sudo usermod -a -G tracing $USER

// Enable setting the process to a certain core
// sudo groupadd nice
// sudo usermod -a -G nice $USER

// echo "@nice - nice -19" | sudo tee -a /etc/security/limits.conf
// echo "@nice hard nice -19" | sudo tee -a /etc/security/limits.conf
// echo "@nice soft nice -19" | sudo tee -a /etc/security/limits.conf

// Do a logout

const ITERATIONS: usize = 1;
const CPU: u8 = 1;
const PROFILE_TIME: u8 = 1;

#[derive(Debug, Serialize)]
struct IterationStat {
    benchmark: Benchmark,
    instructions: u64,
    iterations: u64,
    branches: u64,
    branch_misses: u64,
    cache_misses: u64,
    cycles: u64,
    context_switches: u64,
    power_usage: f64,
}

#[derive(Debug, Clone)]
struct Probe {
    name: String,
    location: String,
    binary: String,
    project: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialOrd, PartialEq, Eq)]
struct Benchmark {
    project: String,
    benchmark: String,
    path: String,
    id: String,
    features: Vec<String>,
}

impl Benchmark {
    fn to_path_buf(&self) -> PathBuf {
        Path::new(&self.project.replace(" ", "_").replace("/", "_"))
            .join(&self.benchmark.replace(r" ", "_").replace("/", "_"))
            .join(self.id.replace(" ", "_").replace("/", "_"))
    }

    fn get_clean_project(&self) -> String {
        self.project.replace(" ", "_").replace("-", "_")
    }

    fn get_clean_benchmark(&self) -> String {
        self.benchmark.replace(" ", "_").replace("/", "_").replace("-", "_")
    }

    fn get_clean_id(&self) -> String {
        self.id.replace(" ", "_").replace("/", "_").replace("-", "_")
    }
}

impl ToString for Benchmark {
    fn to_string(&self) -> String {
        self.to_path_buf().to_str().unwrap().to_string()
    }
}

#[test]
fn write_vec() {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(std::io::stdout());
    let project = TargetProject {
        name: "Hello".to_string(),
        repo_url: "".to_string(),
        repo_tag: "".to_string(),
    };
    wtr.serialize(project).expect("Couldn't write");
}

fn main() {
    do_one_iteration();
}

fn do_one_iteration() {
    let mut run_requests: Vec<(Benchmark, Command)> = Default::default();
    let mut existing_probes: Vec<String> = vec![];

    for record in read_target_projects() {
        println!("{:?}", record);

        let target_project = record;
        let project = Project::load(&target_project.name).unwrap();
        // let project = Project::load("rustls").unwrap();

        for group in &project.bench_files {

            // Compile and save the executable
            let executable = compile_benchmark_file(&group);

            // Create probes
            let probe_addresses = find_probe_addresses(&project.name, &executable);
            create_named_probe_for_adresses(&group.get_clean_name(), &project.name, &executable, probe_addresses);
            existing_probes.push(group.get_clean_name());

            for bench_id in group.benches.iter() {
                let bench = Benchmark {
                    project: project.name.clone(),
                    benchmark: group.name.clone(),
                    path: group.source.clone(),
                    id: bench_id.clone(),
                    features: group.features.clone(),
                };

                let command = create_command_for_bench(&bench);
                run_requests.push((bench, command));
            }
        }
    }

    run_requests.shuffle(&mut thread_rng());

    run_requests.iter_mut()
        .for_each(|(b, c)| do_new_iteration(b, c));

    for probe in existing_probes {
        delete_probe(&format!("probe_{}:*", probe));
    }
}

fn create_command_for_bench(benchmark: &Benchmark) -> Command {
    let measures: String = vec![
        "duration_time",
        "cycles",
        "instructions",
        "branches",
        "branch-misses",
        "cache-misses",
        "context-switches",
        "r119", // Energy per core
        "r19c", // Temperature IA32_THERMAL_STATUS register, bits 22:16 are temp in C
        "power/energy-pkg/",
        "power/energy-ram/",
        "mem-loads",
        &format!("probe_{}:*", benchmark.get_clean_benchmark())
    ]
    .join(",");

    let perf_output_file = format!("perf_bench_{}_{}.csv", benchmark.get_clean_benchmark(), benchmark.get_clean_id());
    let perf_output_file_path = Path::new(&std::env::current_dir().unwrap()).join("data").join(&benchmark.get_clean_project()).join(perf_output_file).to_str().unwrap().to_string();
    fs::create_dir_all(Path::new(&perf_output_file_path).parent().unwrap());

    let mut command = Command::new("perf");
    command
        .arg("stat")
        .arg("--append")
        .arg("-o").arg(perf_output_file_path)
        .arg("-e")
        .arg(measures)
        .arg("-x,") // Output all on one line separated by comma
        .arg("-C")
        .arg(CPU.to_string()) // measure core CPU
        .arg("-I1000")
        // Common values for each specific project
        .env("CARGO_PROFILE_BENCH_LTO", "no") // Debug info is stripped if LTO is on
        .env("CARGO_PROFILE_BENCH_DEBUG", "true") // Probe requires debuginfo
        .current_dir(get_workdir_for_project(&benchmark.project)) // Work in the project directory
        // Command for perf to execute come after this
        .arg("--")
        // Taskset - run on certain core
        // Run the following on core CPU
        .arg("taskset")
        .arg("-c")
        .arg(CPU.to_string())
        // Nice process affinity
        // Run the process with the highest priority
        .arg("nice")
        .arg("-n")
        .arg("-19")
        // Cargo bench command
        .arg("cargo")
        .arg("bench")
        .arg("--bench")
        .arg(&benchmark.benchmark);

    // Add cargo features if applicable
    if benchmark.features.len() > 0 {
        command.arg("--features").arg(&benchmark.features.join(","));
    }

    // Commands to criterion
    command
        .arg("--")
        .arg("--profile-time")
        .arg(PROFILE_TIME.to_string())
        // Last argument of criterion is <filter>, which acts as a regex.
        // This way we match and only match the benchmark we want
        .arg(format!("^{}$", &benchmark.id));


    command
}

fn do_new_iteration(benchmark: &Benchmark, cmd: &mut Command) {
    println!("Running project: {}, benchmark: {}, id: {} at {}", &benchmark.project, &benchmark.benchmark, &benchmark.id, cmd.get_current_dir().unwrap().to_str().unwrap());
    let output = cmd.output().unwrap();
    // writeln!("{:?}", output.stderr);
    // println!("{:?}", output.stdout);
    // println!("{}", std::str::from_utf8(&*output.stderr).unwrap());

    let status = output.status;
    println!("{}", status);

}

// fn store_csv(benchmark: &Benchmark, data: Vec<f64>) {
//     let map = data
//         .iter()
//         .map(|x| format!("{x}"))
//         .reduce(|acc, c| acc.add("\n").add(&c))
//         .unwrap();
//     fs::create_dir_all(format!(
//         "data/{}/{}/",
//         benchmark.get_clean_project(),
//         benchmark.get_clean_benchmark()
//     ))
//     .unwrap();
//     fs::write(
//         format!(
//             "data/{}/{}/{}.csv",
//             benchmark.get_clean_project(),
//             benchmark.get_clean_benchmark(),
//             benchmark.get_clean_id()
//         ),
//         map,
//     )
//     .unwrap();
// }

fn compile_benchmark_file(benchmark: &BenchFile) -> String {
    println!("Compiling {} in {}", benchmark.name, benchmark.get_workdir());
    let mut cargo = Command::new("cargo");

    cargo
        .arg("bench") // cargo bench
        .current_dir(get_workdir_for_project(&benchmark.project).join(benchmark.get_workdir())) // TODO get this from benchmark
        .env("CARGO_PROFILE_BENCH_DEBUG", "true") // We need debug info to find probepoints
        .env("CARGO_PROFILE_BENCH_LTO", "no") // Debug info is stripped if LTO is on
        // .env("CARGO_PROFILE_BENCH_CODEGEN_UNITS", "16") // Debug info is stripped if LTO is on
        .arg("--bench")
        .arg(&benchmark.name)
        .arg("--no-run");

    if benchmark.features.len() > 0 {
        cargo.arg("--features").arg(benchmark.features.join(","));
    }

    println!("{:?}", cargo);

    let raw = cargo.output().unwrap();

    let output = std::str::from_utf8(&*raw.stderr).unwrap().to_string();
    // println!("{}", output);
    let regex = Regex::new(r"Executable .*? \((.*?target/release/deps/[\w_-]+)\)").unwrap();
    let mut string = String::new();
    let mut matches = regex.captures_iter(&output);
    string.push_str(&matches.next().unwrap()[1]);

    string
}
