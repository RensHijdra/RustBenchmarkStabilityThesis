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
use std::time::{SystemTime, UNIX_EPOCH};

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
    do_one_iteration(1, 1, 1);
    // do_one_iteration();
    // do_one_iteration();
    // do_one_iteration();
    // do_one_iteration();
}


pub fn run(repetitions: usize, iterations: usize, profile_time: u64, cpu: usize) {
    for _ in 0..iterations {
        iteration(30, 5, 30);
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
    m.println("Clearing existing projects");

    // for record in &target_projects {
    //     let project = Project::load(&record.name).expect("Could not load project {");
    //     cargo_clean_project(&project.name);
    //     cargo_clear_bar.inc(1);
    // }

    cargo_clear_bar.finish();
    m.remove(&cargo_clear_bar);

    m.println("Compiling projects");
    let mut commands: Vec<Command> = Default::default();

    let compile_project_bar = m.add(ProgressBar::new(target_projects.len() as u64));
    compile_project_bar.set_style(sty.clone());
    // Compile all files and give permissions to all executables
    // Create command per benchmark
    for record in &target_projects {
        m.println(format!("Compiling project: {:?}", record));

        let target_project = record;
        let project = Project::load(&target_project.name).unwrap();

        let bench_group_bar = m.insert_after(&compile_project_bar, ProgressBar::new(project.bench_files.len() as u64));
        bench_group_bar.set_style(sty.clone());

        for group in &project.bench_files {
            m.println(format!("Compiling project: {}; benchmark: {}", project.name, group.name));

            // Compile and save the executable
            let executable = compile_benchmark_file(&group);
            let success = elevate_executable(&executable);

            if !success {
                panic!("Failed to set capabilities for benchmark {}, have you run `sudo *this program* elevate`", &executable)
            }

            for bench_id in group.benches.iter() {
                commands.push(criterion_bench_command(&executable, &bench_id, measurement_time, warmup_time, sample_size));
            }
            bench_group_bar.inc(1);
        }
        bench_group_bar.finish();
        compile_project_bar.inc(1);
    }
    compile_project_bar.finish();

    // Shuffle commands
    commands.shuffle(&mut thread_rng());

    println!("Running commands");
    // Run commands
    let success = commands.iter_mut().progress().all(run_command);

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

fn do_one_iteration(repetitions: usize, profile_time: u64, cpu: usize) {

    // let tmp_dir = tempdir().unwrap();
    // let fifo_path = tmp_dir.path().join("control.pipe");

    // create new fifo and give read, write and execute rights to others
    // match unistd::mkfifo(&fifo_path, stat::Mode::S_IRWXU) {
    //     Ok(_) => println!("Created {:?}", fifo_path),
    //     Err(err) => println!("Error creating fifo: {}", err),
    // }

    // Remove all artifacts
    println!("Clearing existing builds");
    for record in read_target_projects().iter().progress() {
        let project = Project::load(&record.name).expect("Could not load project {");
        cargo_clean_project(&project.name)
    }
    println!("Compiling projects");

    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
        .unwrap()
        .progress_chars("##-");

    let pb = m.add(ProgressBar::new(128));
    pb.set_style(sty.clone());


    let mut run_requests: Vec<(Benchmark, Command)> = Default::default();
    let mut existing_probes: Vec<String> = vec![];

    for record in read_target_projects().iter().progress() {
        debugln!("{:?}", record);

        let project = Project::load(&record.name).unwrap();


        for group in project.bench_files.iter().progress() {
            // Compile and save the executable
            let executable = compile_benchmark_file(&group);


            let functions = find_mangled_functions(&executable);
            let status = create_probe_for_mangled_functions(&functions, &executable, &group);

            existing_probes.push(group.get_clean_name());

            for bench_id in group.benches.iter().progress() {
                let bench = Benchmark {
                    project: project.name.clone(),
                    benchmark: group.name.clone(),
                    path: group.source.clone(),
                    id: bench_id.clone(),
                    features: group.features.clone(),
                };

                let command = create_command_for_bench(&bench, &executable, profile_time, cpu);
                run_requests.push((bench, command));
            }
        }
    }

    run_requests.shuffle(&mut thread_rng());
    println!("Running experiments now...");
    run_requests
        .iter_mut().progress()
        .for_each(|(b, c)| run_benchmark(b, c, repetitions));

    for probe in existing_probes {
        delete_probe(&format!("probe_{}:*", probe));
    }
}

fn elevate_executable(executable: &str) -> bool {
// Command::new("getpcaps").arg("0").spawn().unwrap().wait_with_output().unwrap();
    // unsafe {
    //     println!("{}"&executable);
    //     let string = CString::new("setcap".as_bytes()).unwrap();
    //     let prog = string.as_ptr();
    //     let args = vec![CString::new("CAP_SYS_RAWIO=ep".as_bytes()).unwrap(), CString::new(executable.as_bytes()).unwrap()];
    //     let mut args_raw: Vec<*const libc::c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    //     args_raw.push(ptr::null());
    //     let argv: *const *const libc::c_char = args_raw.as_ptr();
    //     execv(prog, argv);
    //     let errno: i32 = Error::last_os_error().raw_os_error().unwrap();
    //     if errno != 0 {
    //         panic!("execv exitted with non-zero code {}", errno);
    //     }
    // }
    // match Command::new("setcap").arg("CAP_SYS_RAWIO=epi").arg(&executable).spawn() {
    //     Ok(child) => {
    //
    //         match child.wait_with_output() {
    //             Ok(out) => {
    //                 let success = out.status.success();
    //                 if !success {
    //
    //                     println!("out: {}", String::from_utf8(out.stdout).unwrap());
    //                     println!("err: {}", String::from_utf8(out.stderr).unwrap());
    //                 }
    //                 return success;
    //             }
    //             Err(err) => panic!("setcap failed to exit succesfully: {}", err)
    //         }
    //     }
    //     Err(err) => panic!("Error setting capabilities. Consider running `sudo *this program* elevate` first.")
    // }
    true
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

    pub fn create_command_for_bench(benchmark: &Benchmark, executable: &str, profile_time: u64, cpu: usize) -> Command {

        // let events: String = vec![
        //     format!("probe_{}:{}*", benchmark.get_clean_benchmark(), benchmark.get_clean_benchmark()).as_str(),
        //     "duration_time",
        //     "cycles",
        //     "instructions",
        //     "branches",
        //     "branch-misses",
        //     "cache-misses",
        //     "context-switches"
        //     // "r119", // Energy per core
        //     // "r19c", // Temperature IA32_THERMAL_STATUS register, bits 22:16 are temp in C
        //     // "power/energy-pkg/",
        //     // "power/energy-ram/",
        //     // "mem-loads", // Always 0
        // ]
        //     .join(",");
        // let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
        // let perf_output_file = format!("{}.profraw", benchmark.get_clean_id());
        // let msr_output_file = format!("{}_{}.txt", benchmark.get_clean_id(), ts);
        // let perf_output_file_path = Path::new(&std::env::current_dir().unwrap())
        //     .join("data")
        //     .join(&benchmark.get_clean_project())
        //     .join(benchmark.get_clean_benchmark())
        //     .join(perf_output_file)
        //     .to_str()
        //     .unwrap()
        //     .to_string();
        // let msr_output_file_path = Path::new(&std::env::current_dir().unwrap())
        //     .join("data")
        //     .join(&benchmark.get_clean_project())
        //     .join(benchmark.get_clean_benchmark())
        //     .join(msr_output_file)
        //     .to_str()
        //     .unwrap()
        //     .to_string();
        //
        // fs::create_dir_all(Path::new(&perf_output_file_path).parent().unwrap());
        //
        // let workdir_path = get_workdir_for_project(&benchmark.project);
        // let benchmark_id = &benchmark.id;
        //
        // let target_executable = format!("\"{executable}\" \"--bench\" \"--profile-time\" \"{profile_time}\" \"^{benchmark_id}\\$\"");
        // let rdmsr = format!("\"rdmsr\" \"-d\" \"0xc001029a\" \"-p\" \"{cpu}\" | \"tee\" \"-a\" \"{msr_output_file_path}\"");
        // let date = format!("\"date\" \"+%s%3N\" | \"tee\" \"-a\" \"{msr_output_file_path}\"");
        // let enable = format!("\"echo\" \"enable\" | \"tee\" \"{tmpfile}\"");
        // let disable =format!("\"echo\" \"disable\" | \"tee\" \"{tmpfile}\"");
        //
        //
        let mut command = Command::new("perf");
        //
        // // Configure perf
        // command
        //     .arg("record")
        //     .arg("-o").arg(perf_output_file_path)// Append to the file for this benchmark
        //     .arg("-e").arg(format!("{{{events}}}:S"))// The list of events we want to collect
        //     .arg("-D").arg("-1") // Start with events disabled
        //     .arg("-C").arg(cpu.to_string()) // measure core [cpu]
        //     .arg("--timestamp-boundary") // Add timestamp to first and last sample for matching with rdmsr
        //     .arg("--timestamp-filename")// Append timestamp to output file name
        //     .arg("--control").arg(format!("fifo:{tmpfile}"))// Set the control file for enable/disable commands
        //     .arg("--") // Command for perf to execute come after this
        //
        //     // Taskset - run on the specified core
        //     .arg("taskset").arg("--cpu-list").arg(cpu.to_string())
        //
        //     // Nice process affinity
        //     // Run the process with the highest priority
        //     .arg("nice").arg("-n").arg("-19")
        //
        //     // Execute the following multiple commands in a shell
        //     .arg("bash").arg("-c")
        //     .arg(format!("{date};{rdmsr};{enable};{target_executable};{disable};{rdmsr}"));
        //
        command // Return the command
    }

    pub(crate) fn run_benchmark(benchmark: &Benchmark, cmd: &mut Command, repetitions: usize) {
        println!(
            "Running project: {}, benchmark: {}, id: {} at {:?}",
            &benchmark.project,
            &benchmark.benchmark,
            &benchmark.id,
            cmd.get_current_dir()
        );
        debugln!("{:?}", cmd);
        let output = cmd.output().unwrap().status.success();
    }

    fn cargo_clean_project(project: &str) -> () {
        Command::new("cargo")
            .arg("clean")
            .current_dir(get_workdir_for_project(project))
            .output()
            .unwrap();
    }

    pub fn compile_benchmark_file(benchmark: &BenchFile) -> String {
        debugln!(
        "Compiling {} in {}",
        benchmark.name,
        benchmark.get_workdir()
    );
        let mut cargo = Command::new("cargo");

        cargo
            // .arg("+nightly")
            .arg("bench") // cargo bench
            .current_dir(get_workdir_for_project(&benchmark.project).join(benchmark.get_workdir()))
            // .env("CARGO_PROFILE_BENCH_DEBUG", "true") // We need debug info to find probepoints
            // .env("CARGO_PROFILE_BENCH_LTO", "no") // Debug info is stripped if LTO is on
            // .env("RUSTFLAGS","-Csymbol-mangling-version=v0")
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

