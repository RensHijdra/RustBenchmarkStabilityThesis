#![allow(unused)]

use std::{env, fs, ptr};
use std::ffi::{CString, OsString};
use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::ops::Add;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{AsFd, AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use caps::{Capability, CapSet, CapsHashSet};
use caps::errors::CapsError;
use indicatif::{MultiProgress, ProgressBar, ProgressIterator, ProgressStyle};
use itertools::Itertools;
use lazy_static::lazy_static;
use nix::{libc, unistd};
use nix::libc::execv;
use nix::sys::stat;
use nix::sys::stat::Mode;
use ra_ap_hir::known::assert;
use rand::{Rng, thread_rng};
use rand::seq::SliceRandom;
use regex::{Captures, Regex};
use rstats::Printing;
use serde::{Deserialize, Serialize};
use syscalls::{Errno, syscall, Sysno};
use tempfile::tempdir;

use crate::probe::{create_probe_for_mangled_functions, delete_probe, find_mangled_functions};
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

macro_rules! debugln {
    () => {
        if env::var("ENERGY_DEBUG").is_ok() {
            println();
        }
    };
    ($($arg:tt)*) => {
        if env::var("ENERGY_DEBUG").is_ok() {
            println!($($arg)*);
        }
    };
}

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
    // Set debug mode
    env::set_var("ENERGY_DEBUG", "");
    run(1, 5, 1, 5);
}


pub fn run(iterations: usize, measurement_time: u64, warmup_time: u64, sample_size: u64) {
    for _ in 0..iterations {
        iteration(measurement_time, warmup_time, sample_size);
    }
}

fn iteration(measurement_time: u64, warmup_time: u64, sample_size: u64) {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
        .unwrap()
        .progress_chars("##-");


    // Clear artifacts
    let target_projects = read_target_projects();

    let cargo_clear_bar = m.add(ProgressBar::new(target_projects.len() as u64));
    cargo_clear_bar.set_style(sty.clone());
    // m.println("Clearing existing projects");


    if !env::var("ENERGY_DEBUG").is_ok() {
        // Clear
        for record in &target_projects {
            let project = Project::load(&record.name).expect("Could not load project {");
            cargo_clear_bar.set_message(format!("Clearing project: {}", &project.name));
            cargo_clean_project(&project.name);
            cargo_clear_bar.inc(1);
        }
    }

    cargo_clear_bar.finish();
    m.remove(&cargo_clear_bar);

    let mut commands: Vec<Command> = Default::default();

    let compile_project_bar = m.add(ProgressBar::new(target_projects.len() as u64));
    compile_project_bar.set_style(sty.clone());
    compile_project_bar.enable_steady_tick(Duration::from_secs(1));
    // Compile all files and give permissions to all executables
    // Create command per benchmark
    for record in &target_projects {
        compile_project_bar.set_message(format!("Compiling project: {}", record.name));

        let target_project = record;
        let project = Project::load(&target_project.name).unwrap();

        let bench_group_bar = m.insert_after(&compile_project_bar, ProgressBar::new(project.bench_files.len() as u64));
        bench_group_bar.set_style(sty.clone());
        bench_group_bar.enable_steady_tick(Duration::from_secs(1));
        for group in &project.bench_files {
            bench_group_bar.set_message(format!("Compiling benchmark: {}", group.source));

            // Compile and save the executable
            let executable = compile_benchmark_file(&group);

            for bench_id in group.benches.iter() {
                commands.push(criterion_bench_command(&executable, &bench_id, measurement_time, warmup_time, sample_size));
            }
            bench_group_bar.inc(1);
        }
        bench_group_bar.finish();
        compile_project_bar.inc(1);
        m.remove(&bench_group_bar);
    }
    compile_project_bar.finish();
    m.remove(&compile_project_bar);

    // Shuffle commands
    commands.shuffle(&mut thread_rng());

    let sty = ProgressStyle::with_template(
        "[{elapsed_precise} | {eta_precise}] {wide_bar:40.cyan/blue} {pos:>7}/{len:7}",
    ).unwrap();

    // Set up progress bar for commands
    let mut benchmark_command_iterator = commands.iter_mut().progress();
    benchmark_command_iterator.progress.clone().with_style(sty.clone());
    benchmark_command_iterator.progress.clone().enable_steady_tick(Duration::from_secs(1));

    m.add(benchmark_command_iterator.progress.clone());

    // Run commands
    let success = benchmark_command_iterator.all(run_command);

    // Check if all commands were succesful
    if !success {
        panic!("One of the benchmarks exited with a non-zero exit code.")
    }

    // Save all data
    todo!() // Copy target/criterion/... to a safe location
}

fn run_command(command: &mut Command) -> bool {
    debugln!("{command:?}");
    command.status().unwrap().success()
}


fn criterion_bench_command(executable: &str, benchmark_id: &str, measurement_time: u64, warmup_time: u64, sample_size: u64) -> Command {
    let mut bench_binary = Command::new(executable);

    // Configure the benchmark settings
    bench_binary.arg("--bench")
        .args(["--measurement-time", &measurement_time.to_str()])
        .args(["--warm-up-time", &warmup_time.to_str()])
        .args(["--sample-size", &sample_size.to_str()])

        .arg(format!("^{}$", benchmark_id));
    // Criterion uses a regex to select benchmarks,
    // so we do this to prevent selecting multiple benchmarks to run

    bench_binary
}

fn cargo_clean_project(project: &str) -> () {
    Command::new("cargo")
        .arg("clean")
        .current_dir(get_workdir_for_project(project))
        .output()
        .unwrap();
}

pub fn compile_benchmark_file(benchmark: &BenchFile) -> String {
    debugln!("Compiling {} in {}", benchmark.name, benchmark.get_workdir());
    let mut cargo = Command::new("cargo");

    cargo
        .arg("bench") // cargo bench
        .current_dir(get_workdir_for_project(&benchmark.project).join(benchmark.get_workdir()))
        .arg("--bench")
        .arg(&benchmark.name)
        .arg("--no-run");

    if benchmark.features.len() > 0 {
        cargo.arg("--features").arg(benchmark.features.join(","));
    }

    let output = cargo.output().unwrap();
    let stderr = std::str::from_utf8(&*output.stderr).unwrap().to_string();

    lazy_static! {
        static ref EXEC_REG: Regex = Regex::new(r"Executable .*? \((.*?target/release/deps/[\w_-]+)\)").unwrap();
    }

    match EXEC_REG.captures_iter(&stderr).next() {
        None => String::new(),
        Some(found_match) => found_match[1].to_string()
    }
}

