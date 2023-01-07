use std::{fs, io};
use std::ops::{Add, DerefMut};
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;
use regex::Regex;
use serde::{Deserialize, Serialize};

use project::{get_workdir_for_project, Project};

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

// Replace Criterion in benchmark with Criterion::default().with_profiler(EnergyProfiler::new())
// Import in benchmark file
// use energy_profiler::profiler::EnergyProfiler;
// Add crate to Cargo.toml
//
/*
criterion_group!(name = benches;
    config = Criterion::default().with_profiler(EnergyProfiler::new());
    targets =
 */

// Do a logout

const ITERATIONS: usize = 100;
const CPU: u8 = 1;
const PROFILE_TIME: u8 = 1;

#[derive(Debug, Serialize)]
struct IterationStat {
    project: String,
    group: String,
    id: String,
    bench_iters: String,
    power_usage: f64,
}

struct NewIterationRequest {
    command: Command,

}

#[derive(Clone, Debug)]
struct IterationRequest {
    project: String,
    group: String,
    id: String,
    additional_args: Vec<String>,
    probe: Probe,
}

#[derive(Debug, Clone)]
struct Probe {
    name: String,
    location: String,
    binary: String,
    project: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TargetProject {
    name: String,
    custom_args: Option<Vec<String>>,
}


#[test]
fn write_vec() {
    let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(io::stdout());
    let project = TargetProject { name: "Hello".to_string(), custom_args: None };
    wtr.serialize(project).expect("Coulndnt write");
}

fn main() {
    let mut reader = csv::ReaderBuilder::new().has_headers(false).from_path(Path::new("targets.csv")).expect("Could not find file targets.cvs, consider adding targets");
    // const ITERATIONS: usize = 1;
    // let project_name = String::from("pyo3");
    // let project_additional_args = vec!["--features".to_string(), "__internal_bench".to_string()];
    // let project_additional_args: Vec<String> = vec![];
    // find_benchmarks_for_project(&project_name, &project_additional_args);
    // return;

    //Attempt to delete any existing probes:


    // println!("{:?}", Command::new("pwd").current_dir("../pyo3/").output());

    // find_benchmarks_for_project(project_name, &project_additional_args);
    // println!("{:?}", output);

    let mut requests: Vec<NewIterationRequest> = vec![];

    for record in reader.deserialize::<TargetProject>() {
        println!("{:?}", record);
        let target_project = record.unwrap();
        let custom_args = target_project.custom_args.unwrap_or(vec![]);


        let project = serde_json::from_str::<Project>(&std::fs::read_to_string(format!("{}.json", target_project.name)).unwrap()).unwrap();
        for group in &project.bench_files {
            let bin = compile_benchmark_file(&project.name, &group.name, &custom_args);
            for (idx, benchmark) in group.benches.iter().enumerate() {
                // println!("Attempt to delete existing probes");
                // delete_probe(&safe_name(&benchmark));
                clear_power_results(&target_project.name, benchmark);


                let mut command = Command::new("perf");
                command.arg("stat")
                    // .arg("-e").arg(probe)           // Probe the specified event
                    .arg("-e").arg("cycles,instructions,branches,branch-misses,cache-misses,context-switches")
                    .arg("-x,")                     // Output all on one line separated by comma
                    .arg("-C").arg(CPU.to_string()) // measure core CPU

                    // Common
                    .env("CARGO_PROFILE_BENCH_DEBUG", "true")   // Probe requires debuginfo
                    .current_dir(get_workdir_for_project(&project.name))         // Work in the project directory

                    // Command to execute
                    .arg("--")

                    // Taskset - run on certain core
                    .arg("taskset").arg("-c").arg(CPU.to_string())      // Run the following on core CPU

                    //Nice process affinity
                    .arg("nice").arg("-n").arg("-19")                   // Run the process with the highest priority

                    // Cargo bench command
                    .arg("cargo").arg("bench")
                    .args(custom_args.clone())
                    .arg("--bench").arg(benchmark)
                    .arg("--")
                    .arg("--profile-time").arg(PROFILE_TIME.to_string())
                    .arg(benchmark);

                let cmd = NewIterationRequest {command };


                let probe = Probe {
                    name: benchmark.replace("/", "_").replace(" ", "_"),
                    location: String::new(),
                    binary: bin.clone(),
                    project: target_project.name.clone(),
                };

                let request = IterationRequest {
                    project: project.name.clone(),
                    group: group.name.clone(),
                    id: benchmark.to_string(),
                    additional_args: custom_args.clone(),
                    probe,
                };

                // for _ in 0..ITERATIONS {
                    requests.push(cmd);
                // }
                // Benchmark
                // let iterations = (0..ITERATIONS).map(|_| run_benchmark(&project_name, &group.name, &benchmark, &probe_name, &project_additional_args)).collect::<Vec<u32>>();
                // let vec1 = read_power_from_file(&project_name, &benchmark, 1);
                // let vec2 = zip(iterations, vec1).collect::<Vec<(u32, f64)>>();
                //
                // Conclusion
                // store_csv(&project_name, &group.name, benchmark, vec2);
            }
        }
    }
    println!("{}", requests.len());
    // requests.shuffle(&mut thread_rng());
    // // println!("{:?}", requests.iter().map(|r| r.id.clone()).collect::<Vec<String>>());
    // let mut stats = requests.iter().map(|r| do_benchmark_iteration(r)).collect::<Vec<IterationStat>>();
    // // println!("{:?}", stats);
    //
    // let mut map = stats.iter().map(|stat| (&stat.id, stat)).into_group_map();
    // // println!("{:?}", map);
    //
    // for key in map.keys() {
    //     let mut stats_for_key = map.get(key).unwrap();
    //     let first = stats_for_key.get(0).unwrap();
    //
    //     let powers: Vec<f64> = read_power_from_file(&first.project, &first.id, 0);
    //     let iters: Vec<String> = stats_for_key.iter().map(|s| s.bench_iters.clone()).collect();
    //     let data: Vec<(String, f64)> = iters.into_iter().zip(powers).collect::<Vec<(String, f64)>>();
    //
    //     store_csv(&first.project, &first.group, &first.id, data);
    // }
    // run_benchmark(&project.name, &project.bench_files[0].name, &project.bench_files[0].benches[0], &project_additional_args);
    // println!("{:?}", vec2);
}

fn do_new_iteration(mut request: &mut NewIterationRequest) {
    request.command.output().unwrap().status;
}

// fn do_benchmark_iteration(request: &IterationRequest) -> IterationStat {
    // let probe = String::from("cycles");
    // let iterations = run_benchmark(&request.project, &request.group, &request.id, &probe, &request.additional_args);
    // if !delete_probe(&probe) {
    //     println!("Failed to delete probe {}", &probe);
    // };
    // IterationStat { project: request.project.clone(), group: request.group.clone(), id: request.id.clone(), bench_iters: iterations, power_usage: 0.0 }
// }

fn store_csv(project: &str, bench: &str, id: &str, data: Vec<(String, f64)>) {
    let map = data.iter().map(|(x, y)| format!("{x}, {y}")).reduce(|acc, c| acc.add("\n").add(&c)).unwrap();
    fs::create_dir_all(format!("data/{}/{}/", project, bench)).unwrap();
    fs::write(format!("data/{}/{}/{}.csv", project, bench, id.replace("/", "_")), map).unwrap();
}

/*
 * https://man7.org/linux/man-pages/man1/perf-probe.1.html
 */
fn create_probe(probe: &Probe) -> String {
    let mut perf = Command::new("perf");
    let command = perf.arg("probe")
        .current_dir(get_workdir_for_project(&probe.project))
        .arg("-vvv")
        .arg("-x").arg(&probe.binary)
        .arg("--add").arg(format!("{}={}", probe.name, probe.location));

    // println!("{:?}", command);

    let output = command.output().unwrap();


    let result = unsafe { std::str::from_utf8_unchecked(&output.stderr) };
    // println!("{}", result);
    let regex = Regex::new(r"Writing event: \w:(probe_\w+)/(\w+)").unwrap();
    let mut _match = regex.captures_iter(result).next().unwrap();
    let string = String::from(&_match[1]).add(":").add(&_match[2]);

    // println!("Created probe {string}");
    string
    // result.to_string()
}

fn delete_probe(probe: &str) -> bool {
    Command::new("perf").arg("probe")
        .arg("-d").arg(probe).status().is_ok()
}

fn find_iter(project: &String, a_random_bench_id: &String, binary: &String, source: &String) -> String {
    // TODO read source file and find a iter within a bench method
    let file = fs::read_to_string(Path::new("..").join(project).join(source)).unwrap();
    let idx = Regex::new(a_random_bench_id).unwrap().find(&file).unwrap().end();
    let iter_loc = Regex::new(r"iter").unwrap().find_at(&file, idx).unwrap().start();
    let mut offset = file[0..=iter_loc].matches("\n").count();

    let re = Regex::new(r"(?m)^Reversed line: (/rustc/[0-9a-f]+/library/core/src/ptr/mod.rs:\d+$)").unwrap();

    let probe_point: String;

    loop {
        let x = Command::new("perf").arg("probe")
            .current_dir(get_workdir_for_project(project))
            .arg("-n").arg("-vvv")
            .arg("-x").arg(binary).arg("--add").arg(format!("{}:{}", source, offset))
            .output().unwrap();

        let result = std::str::from_utf8(&x.stderr).unwrap();
        match re.captures_iter(result).next() {
            None => {}
            Some(cap) => {
                probe_point = String::from(&cap[1]);
                break;
            }
        }
        offset += 1;
    };
    probe_point
}

fn clear_power_results(project: &str, benchmark_id: &str) {
    match fs::remove_file(Path::new("..").join(project).join("target/criterion").join(benchmark_id).join("profile").join(benchmark_id.replace("/", "_"))) {
        Ok(_) => {}
        Err(_) => {} // This is fine, the file does probably not exist
    }
}

fn read_power_from_file(project: &str, benchmark_id: &str, _amount: usize) -> Vec<f64> {
    let string = fs::read_to_string(
        get_workdir_for_project(project).join("target/criterion")
            .join(benchmark_id).join("profile").join(benchmark_id)
    ).unwrap();
    string.split("\n").into_iter().filter_map(|s| f64::from_str(s).ok()).collect()
}

// fn run_benchmark(cmd: NewIterationRequest, project: &str, benchmark: &str, benchmark_id: &str, probe: &str, project_args: &Vec<String>) -> String {
//     println!("Benchmarking {}/{}/{}", project, benchmark, benchmark_id);
//     Perf stat command
    //
    //
    // let output = cmd.command.output().unwrap().stderr;
    // let str = std::str::from_utf8(&output).unwrap().trim().split("\n").last().unwrap();
    // String::from(str)
// }

fn compile_benchmark_file(project: &str, benchmark: &str, project_args: &Vec<String>) -> String {
    println!("Compiling {}", project);
    let mut cargo = Command::new("cargo");
    let cmd = cargo.arg("bench")// cargo bench
        .current_dir(get_workdir_for_project(project))
        .env("CARGO_PROFILE_BENCH_DEBUG", "true")
        .arg("--bench").arg(benchmark)
        .arg("--no-run")
        .args(project_args);

    println!("{:?}", cmd);


    let raw = cmd.output().unwrap();

    let output = std::str::from_utf8(&*raw.stderr).unwrap().to_string();
    // println!("output {}", output);

    let regex = Regex::new(r"\((target/release/deps/[\w_-]+)\)").unwrap();
    // println!("{:?}", regex.captures_iter(&output));
    let mut string = String::new();
    let mut matches = regex.captures_iter(&output);
    string.push_str(&matches.next().unwrap()[1]);
    string
}


fn safe_name(name: &str) -> String {
    name.replace("/", "_").replace(" ", "_")
}



