use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::ToString;
use std::time::{Duration};
use std::{env};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rand::{thread_rng};
use rstats::Printing;
use serde::{Deserialize, Serialize};

use crate::data::compileroutput::CompilerOutputElement;
use crate::data::project::{
    get_workdir_for_project, read_target_projects, BenchFile, Project,
};

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
// newgrp nice
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

// #[derive(Debug, Serialize)]
// struct IterationStat {
//     benchmark: Benchmark,
//     instructions: u64,
//     iterations: u64,
//     branches: u64,
//     branch_misses: u64,
//     cache_misses: u64,
//     cycles: u64,
//     context_switches: u64,
//     power_usage: f64,
// }

// #[derive(Debug, Clone)]
// struct Probe {
//     name: String,
//     location: String,
//     binary: String,
//     project: String,
// }

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
    let project = crate::data::project::TargetProject {
        name: "Hello".to_string(),
        repo_url: "".to_string(),
        repo_tag: "".to_string(),
    };
    wtr.serialize(project).expect("Couldn't write");
}

#[test]
fn main() {
    // Set debug mode
    env::set_var("ENERGY_DEBUG", "");
    env::set_var("KEEP_PROJECTS", "");
    run(1, 5, 1, 5);
}

pub fn run(iterations: usize, measurement_time: u64, warmup_time: u64, sample_size: u64) {
    for i in 0..iterations {
        println!("Running iteration #{}", i + 1);
        iteration(&measurement_time, &warmup_time, &sample_size);
    }
}

fn iteration(measurement_time: &u64, warmup_time: &u64, sample_size: &u64) {
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
            let executable = compile_benchmark_file(&group, None, None, None, None);
            let executable = if executable.is_some() {executable.unwrap()} else {continue};

            let workdir = get_workdir_for_project(&group.project);
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
    let progress_bar = ProgressBar::new(commands.len() as u64);
    progress_bar.clone().with_style(sty.clone());
    progress_bar
        .clone()
        .enable_steady_tick(Duration::from_secs(1));

    m.add(progress_bar.clone());
    println!("Number of commands: {}", commands.len());

    let mut failures: Vec<String> = Default::default();

    // Run commands
    while let Some((mut cmd, _)) = commands.pop() {
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
    measurement_time: &u64,
    warmup_time: &u64,
    sample_size: &u64,
) -> Command {
    let mut bench_binary = Command::new("cset");

    // Setup `cpuset`
    bench_binary.args(["proc", "--exec", "BENCH", "--"]);

    // Configure the benchmark settings
    bench_binary
        .current_dir(workdir.as_path())
        // The Benchmark
        .args(&[executable, "--bench"])
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

pub fn compile_benchmark_file(
    benchmark: &BenchFile,
    toolchain: Option<String>,
    args: Option<Vec<&str>>,
    features: Option<Vec<&str>>,
    envs: Option<std::collections::HashMap<&str, &str>>,
) -> Option<String> {
    debugln!(
        "Compiling {} in {}",
        benchmark.name,
        benchmark.get_workdir()
    );

    let mut cargo = Command::new("cargo");

    if let Some(toolchain) = toolchain {
        cargo.arg(toolchain);
    }

    if let Some(env_map) = envs {
        cargo.envs(env_map);
    }

    cargo
        .arg("bench") // cargo bench
        .current_dir(get_workdir_for_project(&benchmark.project))
        .arg("--bench")
        .arg(&benchmark.name)
        .arg("--no-run")
        .arg("--message-format=json");

    if let Some(args) = args {
        cargo.args(args);
    }

    let mut features = features.unwrap_or(Vec::new());
    features.extend(benchmark.features.iter().map(String::as_str));

    if features.len() > 0 {
        cargo.arg("--features").arg(features.join(","));
    }

    debugln!("Compile command: {:?}", cargo);

    let output = cargo.output().unwrap();
    let stdout = std::str::from_utf8(&*output.stdout).unwrap().to_string();
    // println!("{}", stdout);
    let compiler_emits: Option<String> = stdout
        .split("\n")
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str::<CompilerOutputElement>(&line).unwrap())
        .filter_map(|elem| elem.executable)
        .next();

    compiler_emits
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
