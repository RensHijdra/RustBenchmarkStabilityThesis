#![allow(unused_imports)]

use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::Command;

use lazy_static::lazy_static;
use nix::sys::stat;
use nix::unistd;
use ra_ap_hir::known::{assert, str};
use regex::Regex;
use tempfile::tempdir;

use crate::collect::{Benchmark, compile_benchmark_file, create_command_for_bench, run_benchmark};
use crate::project::{BenchFile, find_benchmarks_for_project, get_workdir_for_project, Project};

pub(crate) fn find_mangled_functions(executable_path: &str) -> Vec<String> {
    lazy_static! {
        static ref RE_MANGLED_FUNCS: Regex = Regex::new(r"(?m)^[[:xdigit:]]*\st\s(.*Bencher.{1,4}iter.*)$").unwrap();
    }

    let vec = Command::new("nm").arg("-a").arg(executable_path).output().unwrap().stdout;
    let output = String::from_utf8(vec).unwrap();

    RE_MANGLED_FUNCS.captures_iter(&output).map(|capture| String::from(&capture[1])).collect()
}

#[test]
fn test_find_mangled_functions() {
    let project = Project::load("ahash").unwrap();
    let bench = project.bench_files.iter().filter(|p| p.name == "ahash").next().expect("Are the projects loaded?");
    let exe = compile_benchmark_file(&bench);
    let mangled_functions = find_mangled_functions(&exe);
    println!("{:?}", mangled_functions);
    println!("{}", mangled_functions.len());
    assert!(mangled_functions.len() > 0);
}

pub(crate) fn create_probe_for_mangled_functions(function_names: &Vec<String>, executable: &str, bench: &BenchFile) -> bool {
    function_names.iter().map(|function| {
        let result = Command::new("perf")
            // .current_dir(get_workdir_for_project(&bench.project))
            .arg("probe")
            .arg("-f") // Force probes with the same name
            .arg("-x").arg(executable)
            .arg("--add").arg(format!("{}={} self->iters", bench.get_clean_name(), function))
            .status();
        // function return probe
        // Command::new("perf")
        //     // .current_dir(get_workdir_for_project(&bench.project))
        //     .arg("probe")
        //     .arg("-f") // Force probes with the same name
        //     .arg("-x").arg(executable)
        //     .arg("--add").arg(format!("{}={}%return", bench.get_clean_name(), function))
        //     .status().unwrap();

        if result.is_err() {
            // Some versions require iters to be accessed by . instead of ->
            Command::new("perf")
                // .current_dir(get_workdir_for_project(&bench.project))
                .arg("probe")
                .arg("-f") // Force probes with the same name
                .arg("-x").arg(executable)
                .arg("--add").arg(format!("{}={} self.iters", bench.get_clean_name(), function))
                .status().unwrap()
        } else {
            result.unwrap()
        }
    }).all(|status| status.success())
}

pub(crate) fn delete_probe(probe: &str) -> bool {
    Command::new("perf")
        .arg("probe")
        .arg("-d")
        .arg(probe)
        .output()
        .is_ok()
}


#[test]
fn run_test_mangled_function_probe() {
    let project = Project::load("chrono").unwrap();
    for bench in project.bench_files {
        let exe = compile_benchmark_file(&bench);
        let functions = find_mangled_functions(&exe);
        println!("{:?}", functions);
        let status = create_probe_for_mangled_functions(&functions, &exe, &bench);
        assert!(status);
    }
}

#[test]
fn run_test_with_probes() {
    let project = Project::load("chrono").unwrap();
    let tmp_dir = tempdir().unwrap();
    let fifo_path = tmp_dir.path().join("control.pipe");

    // create new fifo and give read, write and execute rights to others
    match unistd::mkfifo(&fifo_path, stat::Mode::S_IRWXU) {
        Ok(_) => println!("Created {:?}", fifo_path),
        Err(err) => println!("Error creating fifo: {}", err),
    }


    for bench in project.bench_files {
        let exe = compile_benchmark_file(&bench);
        println!("{}", exe);
        let functions = find_mangled_functions(&exe);
        println!("{:?}", functions);
        let status = create_probe_for_mangled_functions(&functions, &exe, &bench);
        for bench_method in bench.benches {
            let benchmark = Benchmark::new(
                project.name.clone(),
                bench.name.clone(),
                bench.source.clone().rsplit_once("/").unwrap().0.to_string(),
                bench_method.clone(),
                bench.features.clone(),
            );
            println!("{}", bench_method);
            let mut command = create_command_for_bench(&benchmark, &exe, 2, 1, fifo_path.to_str().unwrap());
            println!("{:?}", command);
            run_benchmark(&benchmark, &mut command, 1);
            // let output = command.output().unwrap();
            // println!("{}", std::str::from_utf8(&*output.stderr).unwrap());
        }
        delete_probe(&format!("probe_{}:*", &bench.name));
    }
}
