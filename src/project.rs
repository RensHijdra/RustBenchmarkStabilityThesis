use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub(crate) name: String,
    pub(crate) bench_files: Vec<BenchFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BenchFile {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) benches: Vec<String>,
    pub(crate) sources: Vec<String>,
}

#[allow(unused)]
fn find_benchmarks_for_project(project_name: &str, project_additional_args: &Vec<String>) {
    println!("Running `cargo bench -- --list` for '{}'", project_name);

    let work_dir = get_workdir_for_project(project_name);

    let output = Command::new("cargo").current_dir(&work_dir)
        .arg("bench")
        .args(project_additional_args)
        .arg("--")
        .arg("--list")
        .output().expect("Failed to run cargo bench");

    let benches_list = std::str::from_utf8(&*output.stderr).expect("Could not parse stderr as UTF-8");

    let re = Regex::new(r"Running (benches/([\w/_-]+)\.rs)\s(\([\w/_\-]+\))").unwrap();
    let re_bench_func = Regex::new(r"([\w_/:.\- ]+): bench").unwrap();

    let mut bench_files: Vec<BenchFile> = vec![];
    for cap in re.captures_iter(benches_list) {
        let source = &cap[1];
        let name = &cap[2];
        print!("Checking benches for file {}... \t", source);


        // TODO get the specific benches from the source file

        let benches = Command::new("cargo").current_dir(&work_dir)
            .arg("bench")
            .args(project_additional_args)
            .arg("--bench")
            .arg(name.to_string())
            .arg("--").arg("--list")
            .output().expect("could not run --bench").stdout;

        let benchmark_ids = re_bench_func.captures_iter(std::str::from_utf8(&*benches).expect("Could not parse UTF-8")).map(|c| String::from(&c[1])).collect::<Vec<String>>();
        let bf = BenchFile { name: name.to_string(), source: source.to_string(), benches: benchmark_ids, sources: vec![] };
        println!("found {} benchmark(s) for {}", bf.benches.len(), name);

        // TODO find the source of the benched method
        bench_files.push(bf)
    }

    let proj = Project { name: project_name.to_string(), bench_files };

    let string = serde_json::to_string(&proj).unwrap();

    std::fs::write(format!("{}.json", proj.name), string).unwrap();
    println!("Wrote output to {}.json", proj.name);

    // TODO append to targets.csv

    // perf stat -e probe_chrono:iter /bin/su -x, -c "cd `pwd` && CARGO_PROFILE_BENCH_DEBUG=true cargo bench --features __internal_bench --bench chrono -- --profile-time 1 bench_datetime_from_str > /dev/null" - ${USER}

    // println!("{:?}", output);
}

pub fn get_workdir_for_project(project: &str) -> PathBuf {
    Path::new(&std::env::current_dir().unwrap()).join("..").join("projects").join(project)
}

#[test]
fn run_find_benchmarks_for_project() {
    find_benchmarks_for_project("rust-prometheus", &vec![])
}