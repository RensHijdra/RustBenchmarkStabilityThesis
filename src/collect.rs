#![allow(unused)]

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
use std::{env, fs, ptr};

use caps::errors::CapsError;
use caps::{CapSet, Capability, CapsHashSet};
use indicatif::{MultiProgress, ProgressBar, ProgressIterator, ProgressStyle};
use itertools::Itertools;
use lazy_static::lazy_static;
use nix::libc::execv;
use nix::sys::stat;
use nix::sys::stat::Mode;
use nix::{libc, unistd};
use ra_ap_hir::known::assert;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use regex::{Captures, Regex};
use rstats::Printing;
use serde::{Deserialize, Serialize};
use syscalls::{syscall, Errno, Sysno};
use tempfile::tempdir;

use crate::probe::{create_probe_for_mangled_functions, delete_probe, find_mangled_functions};
use crate::project::{
    get_workdir_for_project, read_target_projects, BenchFile, Project, TargetProject,
};

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

    pub fn new(
        project: String,
        benchmark: String,
        path: String,
        id: String,
        features: Vec<String>,
    ) -> Self {
        Self {
            project,
            benchmark,
            path,
            id,
            features,
        }
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
    for i in 0..iterations {
        println!("Running iteration #{}", i + 1);
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

    if !env::var("KEEP_PROJECTS").is_ok() {
        let cargo_clear_bar = m.add(ProgressBar::new(target_projects.len() as u64));
        cargo_clear_bar.set_style(sty.clone());

        // Clear
        for record in &target_projects {
            let project = Project::load(&record.name).expect("Could not load project {");
            cargo_clear_bar.set_message(format!("Clearing project: {}", &project.name));
            cargo_clean_project(&project.name);
            cargo_clear_bar.inc(1);
        }

        cargo_clear_bar.finish();
        m.remove(&cargo_clear_bar);
    }

    let mut commands: Vec<(Command, String)> = Default::default();

    let compile_project_bar = m.add(ProgressBar::new(target_projects.len() as u64));
    compile_project_bar.set_style(sty.clone());
    compile_project_bar.tick();
    compile_project_bar.enable_steady_tick(Duration::from_secs(1));

    // Compile all files and give permissions to all executables
    // Create command per benchmark
    for record in &target_projects {
        compile_project_bar.set_message(format!("Compiling project: {}", record.name.trim()));

        let target_project = record;
        let project = Project::load(&target_project.name).unwrap();

        let bench_group_bar = m.insert_after(
            &compile_project_bar,
            ProgressBar::new(project.bench_files.len() as u64),
        );
        bench_group_bar.set_style(sty.clone());
        bench_group_bar.tick();
        bench_group_bar.enable_steady_tick(Duration::from_secs(1));
        for group in &project.bench_files {
            bench_group_bar.set_message(format!("Compiling benchmark: {}", group.name.trim()));

            // Compile and save the executable
            let (executable, workdir) = compile_benchmark_file(&group);
            debugln!("Executable {} and workdir {:?}", &executable, &workdir);

            for benchmark_id in group.benches.iter() {
                commands.push((
                    criterion_bench_command(
                        &executable,
                        &benchmark_id,
                        &workdir,
                        measurement_time,
                        warmup_time,
                        sample_size,
                    ),
                    format!("{}/{}/{}", project.name, group.name, benchmark_id),
                ));
            }
            bench_group_bar.inc(1);
        }
        compile_project_bar.inc(1);
        m.remove(&bench_group_bar);
    }
    m.remove(&compile_project_bar);

    // Shuffle commands
    commands.shuffle(&mut thread_rng());

    let sty = ProgressStyle::with_template(
        "[{elapsed_precise} | {eta_precise}] {wide_bar:.cyan/blue} {pos:>4}/{len:4}",
    )
    .unwrap();

    // Set up progress bar for commands
    let mut progress_bar = ProgressBar::new(commands.len() as u64);
    progress_bar.clone().with_style(sty.clone());
    progress_bar
        .clone()
        .enable_steady_tick(Duration::from_secs(1));

    m.add(progress_bar.clone());
    println!("Number of commands: {}", commands.len());

    let mut failures: Vec<String> = Default::default();

    // Run commands
    while let Some((mut cmd, path)) = commands.pop() {
        let success = run_command(&mut cmd);
        progress_bar.inc(1);
        if !success {
            failures.push(format!("{cmd:?}"));
        }
    }

    // Check if all commands were succesful
    if failures.len() > 0 {
        println!("The following benchmarks failed:");
        println!("{}", failures.join("\n"));
    }

    // Save all data
    let timestamp = chrono::offset::Local::now().timestamp_millis().to_string();
    for record in &target_projects {
        let project = Project::load(&record.name).expect("Could not load project");
        move_data_for_project(project, &timestamp);
    }
}

#[test]
fn test_move() {
    let timestamp = chrono::offset::Local::now()
        .format("%Y%m%d%H%M%S")
        .to_string();
    for record in read_target_projects() {
        let project = Project::load(&record.name).expect("Could not load project");
        move_data_for_project(project, &timestamp);
    }
}

fn move_data_for_project(project: Project, timestamp: &str) {
    let from = get_workdir_for_project(&project.name)
        .join("target")
        .join("criterion");
    let to = env::current_dir()
        .unwrap()
        .join("data")
        .join(timestamp)
        .join(&project.name);

    Command::new("mkdir")
        .args(["-p", &to.to_string_lossy()])
        .output()
        .unwrap();

    Command::new("mv")
        .args([
            &from.to_string_lossy().to_str(),
            &to.to_string_lossy().to_string(),
        ])
        .status()
        .unwrap();
}

fn run_command(command: &mut Command) -> bool {
    debugln!("{command:?}");
    debugln!("workdir: {:?}", command.get_current_dir());
    let output = command.output().unwrap();
    debugln!("{}", String::from_utf8(output.stdout).unwrap());
    output.status.success()
}

fn criterion_bench_command(
    executable: &str,
    benchmark_id: &str,
    workdir: &PathBuf,
    measurement_time: u64,
    warmup_time: u64,
    sample_size: u64,
) -> Command {
    let mut bench_binary = Command::new(executable);

    // Configure the benchmark settings
    bench_binary
        .current_dir(workdir.as_path())
        .arg("--bench")
        .args(["--measurement-time", &measurement_time.to_str()])
        .args(["--warm-up-time", &warmup_time.to_str()])
        .args(["--sample-size", &sample_size.to_str()])
        // Criterion uses a regex to select benchmarks,
        // so we do this to prevent selecting multiple benchmarks to run
        .arg(format!("^{}$", benchmark_id));
    bench_binary
}

fn cargo_clean_project(project: &str) -> () {
    Command::new("cargo")
        .arg("clean")
        .current_dir(get_workdir_for_project(project).as_path())
        .output()
        .unwrap();
}

pub fn compile_benchmark_file(benchmark: &BenchFile) -> (String, PathBuf) {
    debugln!(
        "Compiling {} in {}",
        benchmark.name,
        benchmark.get_workdir()
    );
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

    debugln!("Compile command: {:?}", cargo);

    let output = cargo.output().unwrap();
    let stderr = std::str::from_utf8(&*output.stderr).unwrap().to_string();
    lazy_static! {
        static ref EXEC_REG: Regex =
            Regex::new(r"Executable .*? \((.*?target/release/deps/[\w_-]+)\)").unwrap();
    }
    // println!("{:?}", &stderr);
    match EXEC_REG.captures_iter(&stderr).next() {
        None => {
            panic!(
                "Did not find an executable while compiling {}: {:?}",
                benchmark.project, cargo
            );
        }
        Some(found_match) => {
            debugln!(
                "Found {} and {:?}",
                found_match[1].to_string(),
                &get_workdir_for_project(&benchmark.project)
            );
            (
                found_match[1].to_string(),
                get_workdir_for_project(&benchmark.project),
            )
        }
    }
}

#[test]
fn test_current_dir() {
    let mut command = Command::new("pwd");
    command.current_dir("/home/rens/thesis/scrape-crates/projects/chrono");
    println!("{command:?}");
    assert_eq!(
        String::from_utf8(command.output().unwrap().stdout)
            .unwrap()
            .trim(),
        "/home/rens/thesis/scrape-crates/projects/chrono"
    )
}
