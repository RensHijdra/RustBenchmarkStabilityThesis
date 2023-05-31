use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use crate::collect::compile_benchmark_file;
use crate::llvmcovdata::{Filter, Function, LlvmCovData};
use crate::project::{read_target_projects, BenchFile, Project};

fn generate_coverage_data() -> Vec<BenchFile> {
    let coverage = build_with_coverage();
    for (exe, benchfile) in coverage {
        let mut dir = env::current_dir().unwrap();
        dir.push("coverage");
        dir.push(benchfile.project);

        fs::create_dir_all(&dir).unwrap();

        for bench in benchfile.benches {
            run_with_coverage(&exe, &bench, &dir);
            merge_profdata(&dir, &bench);
            export_profdata(&exe, &dir, &bench);
            // break;
        }
        // break;
    }
    coverage.iter().map(|(e, b)| b).collect::<Vec<BenchFile>>()
}

fn build_with_coverage() -> Vec<(String, BenchFile)> {
    let mut vec: Vec<(String, BenchFile)> = Default::default();
    for record in read_target_projects() {
        let project = Project::load(&record.name).expect("Could not load project {");
        for benchmark_file in project.bench_files {
            let (exe, _) = compile_benchmark_file(
                &benchmark_file,
                HashMap::from([("RUSTFLAGS", "-C instrument-coverage")]),
            );
            vec.push((exe, benchmark_file));
        }
        break;
    }
    vec
}

fn run_with_coverage(executable: &str, name: &str, proj: &PathBuf) {
    let _ = Command::new(executable)
        .current_dir(proj)
        .env("LLVM_PROFILE_FILE", format!("{}.profraw", name))
        .arg(format!("^{name}$")) // Filter to run only this benchmark
        .output()
        .unwrap();
    // let vec = result.stdout;
    // let string = String::from_utf8(vec).unwrap();
    // println!("{:?}", string);
}

fn merge_profdata(dir: &PathBuf, benchmark_id: &str) {
    let mut input = dir.clone();
    let mut output = dir.clone();

    input.push(benchmark_id);
    output.push(benchmark_id);
    let _ = Command::new("llvm-profdata")
        .arg("merge")
        .arg(format!("{}.profraw", input.as_path().to_str().unwrap()))
        .arg(format!(
            "--output={}.profdata",
            output.as_path().to_str().unwrap()
        ))
        .output()
        .unwrap();
}

fn export_profdata(executable: &str, dir: &PathBuf, benchmark_id: &str) {
    let mut input = dir.clone();
    let mut output = dir.clone();

    input.push(benchmark_id);
    output.push(benchmark_id);
    let _from = "todo!()";
    let _to = "todo!()";
    let result = Command::new("llvm-cov")
        .arg("export")
        .arg(format!(
            "-instr-profile={}.profdata",
            input.as_path().to_str().unwrap()
        ))
        .arg("-format=text")
        .arg(executable)
        .output()
        .unwrap()
        .stdout;

    let string = String::from_utf8(result).unwrap();
    fs::write(
        PathBuf::from(format!("{}.json", output.to_str().unwrap())),
        &string,
    )
    .unwrap();
    let cov_data = serde_json::from_str::<LlvmCovData>(&string).unwrap();
    // cov_data.filter_non_zero();
    // let usage = cov_data.data.iter().map(|entry| entry.functions.iter().map(|func| (func.get_demangled(), func.count.clone()))).flatten().collect::<Vec<(String, i64)>>();
    // println!("{:?}", usage);
    let files = cov_data
        .data
        .iter()
        .map(|entry| {
            entry.files.iter().map(|file| {
                (
                    file.filename.to_string(),
                    file.summary.functions.covered.clone(),
                )
            })
        })
        .flatten()
        .collect::<Vec<(String, i64)>>();
    println!("{:?}", files);
}

fn parse_source_with_coverage(benchfile: BenchFile) {
    let mut dir = env::current_dir().unwrap();
    dir.push("coverage");
    dir.push(benchfile.project);

    for bench in benchfile.benches {
        let data = load_coverage(&dir, &bench);
        for mut entry in data.data {
            entry.filter_non_zero();
            for func in entry.functions {
                get_statistics_from_function(func)
            }
        }
    }
}

fn get_statistics_from_function(function: Function) {

}

fn load_coverage(dir: &PathBuf, id: &str) -> LlvmCovData {
    let mut input = dir.clone();
    input.push(id);

    let string = fs::read_to_string(input).expect("File not found!");
    serde_json::from_str::<LlvmCovData>(&string).expect("Could not read file as valid json")
}

#[test]
fn test_coverage() {
    let covered_benchmarks = generate_coverage_data();
    for bench in covered_benchmarks {
        parse_source_with_coverage(bench);
    }
}
