#![allow(unused)]

use std::fs;
use std::fs::File;
use std::io::Write;
use std::ops::Add;
use std::os::unix::io::{AsFd, AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use itertools::Itertools;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use rand::{Rng, thread_rng};
use regex::Regex;
use serde::{Deserialize, Serialize};
use crate::probe::{create_named_probe_for_adresses, create_probe_for_mangled_functions, delete_probe, find_mangled_functions, find_probe_addresses};
use crate::project::{BenchFile, get_workdir_for_project, Project, read_target_projects, TargetProject};


// use crate::probe::{create_named_probe_for_adresses, delete_probe, find_probe_addresses};
// use crate::project::BenchFile;
// use crate::project::{get_workdir_for_project, read_target_projects, Project, TargetProject};

// mod ::probe;
// mod project;

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

static mut ITERATIONS: usize = 1;
static mut CPU: usize = 1;
static mut PROFILE_TIME: u64 = 30;

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
pub struct Benchmark {
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
        self.benchmark
            .replace(" ", "_")
            .replace("/", "_")
            .replace("-", "_")
    }

    fn get_clean_id(&self) -> String {
        self.id
            .replace(" ", "_")
            .replace("/", "_")
            .replace("-", "_")
    }


    pub fn new(project: String, benchmark: String, path: String, id: String, features: Vec<String>) -> Self {
        Self { project, benchmark, path, id, features }
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
    do_one_iteration(1, 1, 1);
    // do_one_iteration();
    // do_one_iteration();
    // do_one_iteration();
    // do_one_iteration();
}


pub fn run(repetitions: usize, iterations: usize, profile_time: u64, cpu: usize) {

    for _ in 0..iterations {
        do_one_iteration(repetitions,profile_time, cpu);
    }
}


fn do_one_iteration(repetitions: usize, profile_time: u64, cpu: usize) {

    // Remove all artifacts
    for record in read_target_projects() {
        let project = Project::load(&record.name).expect("Could not load project {");
        cargo_clean_project(&project.name)
    }

    let mut run_requests: Vec<(Benchmark, Command)> = Default::default();
    let mut existing_probes: Vec<String> = vec![];

    let file = create_tmp_file();
    let fd = file.as_raw_fd();

    for record in read_target_projects() {
        println!("{:?}", record);

        let target_project = record;
        let project = Project::load(&target_project.name).unwrap();


        for group in &project.bench_files {
            // Compile and save the executable
            let executable = compile_benchmark_file(&group);


            let functions = find_mangled_functions(&executable);
            let status = create_probe_for_mangled_functions(&functions, &executable, &group);

            existing_probes.push(group.get_clean_name());

            for bench_id in group.benches.iter() {
                let bench = Benchmark {
                    project: project.name.clone(),
                    benchmark: group.name.clone(),
                    path: group.source.clone(),
                    id: bench_id.clone(),
                    features: group.features.clone(),
                };

                let command = create_command_for_bench(&bench, &executable, profile_time, cpu, fd);
                run_requests.push((bench, command));
            }
        }
    }

    run_requests.shuffle(&mut thread_rng());

    run_requests
        .iter_mut()
        .for_each(|(b, c)| run_benchmark(b, c));

    for probe in existing_probes {
        delete_probe(&format!("probe_{}:*", probe));
    }
}

pub fn create_tmp_file() -> File {
    // Generate a random id for this temporary file
    let mut uuid: u128 = 0;
    thread_rng().fill(&mut [uuid]);
    let file = File::create(format!("/tmp/perf-control-{:x}", uuid)).unwrap();
    file
}

pub fn create_command_for_bench(benchmark: &Benchmark, executable: &str, profile_time: u64, cpu: usize, fd: RawFd) -> Command {

    let events: String = vec![
        format!("probe_{}:{}*", benchmark.get_clean_benchmark(), benchmark.get_clean_benchmark()).as_str(),
        "duration_time",
        "cycles",
        "instructions",
        "branches",
        "branch-misses",
        "cache-misses",
        "context-switches"
        // "r119", // Energy per core
        // "r19c", // Temperature IA32_THERMAL_STATUS register, bits 22:16 are temp in C
        // "power/energy-pkg/",
        // "power/energy-ram/",
        // "mem-loads", // Always 0
    ]
        .join(",");
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
    let perf_output_file = format!("{}_{}.profraw", benchmark.get_clean_id(), ts);
    let msr_output_file = format!("{}_{}.txt", benchmark.get_clean_id(), ts);
    let perf_output_file_path = Path::new(&std::env::current_dir().unwrap())
        .join("data")
        .join(&benchmark.get_clean_project())
        .join(benchmark.get_clean_benchmark())
        .join(perf_output_file)
        .to_str()
        .unwrap()
        .to_string();
    let msr_output_file_path = Path::new(&std::env::current_dir().unwrap())
        .join("data")
        .join(&benchmark.get_clean_project())
        .join(benchmark.get_clean_benchmark())
        .join(msr_output_file)
        .to_str()
        .unwrap()
        .to_string();
    fs::create_dir_all(Path::new(&perf_output_file_path).parent().unwrap());

    let file = create_tmp_file();

    let fd = file.as_raw_fd();

    let workdir_path = get_workdir_for_project(&benchmark.project);
    let benchmark_id = &benchmark.id;
    let target_executable = format!("\"{executable}\" \"--bench\" \"--profile-time\" \"{profile_time}\" \"^{benchmark_id}\\$\"");
    // let create_fd = String::from("\"exec 89< /tmp/perf.fifo\"");
    let rdmsr = format!("\"rdmsr\" \"-d\" \"0xc001029a\" | \"tee\" \"-a\" \"{msr_output_file_path}\"");
    let enable = "\"echo\" \"enable\" | \"tee\" \"/tmp/perf.fifo\"";
    let disable ="\"echo\" \"disable\" | \"tee\" \"/tmp/perf.fifo\"";

    // Command::new("exec").arg("{ctl_fd}<>/tmp/perf-control.pipe").arg(";").

    let mut command = Command::new("perf");

    // Configure perf
    command
        .arg("record")
        // TODO add output location
        // TODO add quiet perf to read
        // TODO! add collecting rdmsr data
        .arg("-o").arg(perf_output_file_path)// Append to the file for this benchmark
        .arg("-e").arg(format!("{{{events}}}:S"))// The list of events we want to collect
        .arg("-D").arg("-1") // Start with events disabled
        .arg("-C").arg(cpu.to_string()) // measure core CPU
        // .arg("--control").arg("fd:`exec {fd}<>/tmp/perf.fifo; echo ${fd}`")
        .arg("--control").arg("fifo:/tmp/perf.fifo")
        .arg("--") // Command for perf to execute come after this

        // Taskset - run on the specified core
        .arg("taskset").arg("--cpu-list").arg(cpu.to_string())

        // Nice process affinity
        // Run the process with the highest priority
        .arg("nice").arg("-n").arg("-19")

        // Execute the following multiple commands in a shell
        .arg("bash").arg("-c")
        .arg(format!("{rdmsr};{enable};{target_executable};{disable};{rdmsr}"));

    // command // Return the command

    let mut bash = Command::new("bash");
    bash.arg("-c").arg(format!("{command:?}"));
    bash
}

fn run_benchmark(benchmark: &Benchmark, cmd: &mut Command) {
    println!(
        "Running project: {}, benchmark: {}, id: {} at {:?}",
        &benchmark.project,
        &benchmark.benchmark,
        &benchmark.id,
        cmd.get_current_dir()
    );
    println!("{:?}", cmd);
    let output = cmd.output().unwrap();
    let status = output.status;
    println!("{}", status);
}

fn cargo_clean_project(project: &str) -> () {
    Command::new("cargo")
        .arg("clean")
        .current_dir(get_workdir_for_project(project))
        .output()
        .unwrap();
}

pub fn compile_benchmark_file(benchmark: &BenchFile) -> String {
    println!(
        "Compiling {} in {}",
        benchmark.name,
        benchmark.get_workdir()
    );
    let mut cargo = Command::new("cargo");

    cargo
        .arg("+nightly")
        .arg("bench") // cargo bench
        .current_dir(get_workdir_for_project(&benchmark.project).join(benchmark.get_workdir())) // TODO get this from benchmark
        .env("CARGO_PROFILE_BENCH_DEBUG", "true") // We need debug info to find probepoints
        .env("CARGO_PROFILE_BENCH_LTO", "no") // Debug info is stripped if LTO is on
        .env("RUSTFLAGS","-Csymbol-mangling-version=v0")
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
    let next = matches.next();
    if next.is_some() {
        string.push_str(&next.unwrap()[1]);
    } else {
        println!("{}", output);
    }

    string
}
