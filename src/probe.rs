#![allow(unused_imports)]

use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::Command;

use crate::project::{find_benchmarks_for_project, get_workdir_for_project, BenchFile, Project};
use crate::collect::{compile_benchmark_file, create_command_for_bench, Benchmark, create_tmp_file};
use lazy_static::lazy_static;
use ra_ap_hir::known::{assert, str};
use regex::Regex;


pub(crate) fn find_mangled_functions(executable_path: &str) -> Vec<String> {
    lazy_static! {
        static ref RE_MANGLED_FUNCS: Regex = Regex::new(r"(?m)^[[:xdigit:]]*\st\s(.*Bencher.+iter.*)$").unwrap();
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
        Command::new("perf")
            .current_dir(get_workdir_for_project(&bench.project))
            .arg("probe")
            .arg("-f") // Force probes with the same name
            .arg("-x").arg(executable)
            .arg("--add").arg(format!("{}={} self->iters", bench.get_clean_name(), function))
            .status().unwrap()
    }).all(|status| status.success())
}

pub(crate) fn create_named_probe_for_adresses(
    name: &str,
    project: &str,
    executable: &str,
    addresses: Vec<String>,
) -> String {
    for address in addresses {
        let mut command = Command::new("perf");
        let output = command
            .current_dir(get_workdir_for_project(project))
            .arg("probe")
            .arg("-x")
            .arg(executable)
            .arg("--add")
            .arg(format!("{}=0x{}", name, address))
            .arg("-f")
            .output()
            .unwrap();
        println!("{:?}", command);
        // TODO process the output
        println!("{}", String::from_utf8(output.stdout).unwrap());
        println!("------------------");
        let logs = String::from_utf8(output.stderr).unwrap();
        let x = logs
            .split("\n")
            .nth(1)
            .unwrap()
            .trim()
            .split(":")
            .next()
            .unwrap();
        println!("{}", logs);
        println!("------------------");
    }

    String::from("sdass") // TODO output something sensible
}

fn delete_probes_for_group(groupname: &str) -> () {}

pub(crate) fn find_probe_addresses(project: &str, executable: &str) -> Vec<String> {
    let raw_out = Command::new("objdump")
        .current_dir(get_workdir_for_project(project))
        .arg("-S")
        .arg(executable)
        .arg("-C")
        .arg("-l")
        .output()
        .unwrap()
        .stdout;

    let output = String::from_utf8(raw_out).unwrap();
    // println!("{}", output);

    lazy_static! {
        static ref SECTION_RE: Regex = Regex::new(r"self\.measurement\.start\(\);((?:.|\n)*?)self\.measurement.end\(start\);").unwrap();
        static ref ITER_NEXT_TARGET: Regex = Regex::new(r"([a-f0-9]*?):\s*(?:[a-f0-9]{2}\s)*\s+?j[a-z]{1,2}\s*[a-z0-9]+? <criterion::bencher::Bencher<M>::iter\+0x[a-f0-9]+>").unwrap();
        static ref BATCHED_TARGET: Regex = Regex::new(r"[0-9a-f]+:\s+(?:[0-9a-f]{2}\s)+\s+[a-z]{2,4}\s+.+?(?:.+?)?\s+[0-9a-f]+:\s+(?:[0-9a-f]{2}\s)+\s+cmp\s+.+?,.+?\s+([[:xdigit:]]+):\s+(?:[0-9a-f]{2}\s)+\s+j[mpnegtlz]{1,2}\s+[0-9a-f]+\s+").unwrap();

    }

    let mut addresses: Vec<String> = vec![];
    for cap in SECTION_RE.captures_iter(&output) {
        let iter_section = &cap[1];
        // println!("{}", iter_section);
        // println!("------------------------------------------------------------------------------");
        // for closure in CLOSURE_TARGET.captures_iter(iter_section) {
        let option = ITER_NEXT_TARGET.captures_iter(iter_section).last();
        // println!("{:?}", option);
        if option.is_some() {
            addresses.push(option.unwrap()[1].to_string())
        }

        let option = BATCHED_TARGET.captures_iter(iter_section).last();
        if option.is_some() {
            addresses.push(option.unwrap()[1].to_string())
        }


        // else {
        // println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
        // println!("{}", iter_section);
        // println!("^^^^^^^^^^^^^^^^^^^^^^^^^^^^^")
        // }
        // }
    }

    addresses
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
fn test_find_addresses() {
    use crate::collect::compile_benchmark_file;

    // let string =
    //     compile_benchmark_file("chrono", "chrono", &vec![String::from("__internal_bench")]);
    // println!("{}", string);
    // let vec1 = find_probe_addresses("chrono", &string);
    // println!("length: {}, items: {:?}", vec1.len(), vec1);
}

#[test]
fn create_probe_for_executable() {
    use crate::collect::compile_benchmark_file;

    let project = "prost";
    let file = BenchFile {
        project: String::from("prost"),
        name: String::from("dataset"),
        source: String::from("benches/dataset.rs"),
        features: vec![],
        benches: vec![],
    };
    let string = compile_benchmark_file(&file);
    println!("{}", string);
    let vec1 = find_probe_addresses(project, &string);
    println!("{:?}", vec1);
    create_named_probe_for_adresses("test_probe", project, &string, vec1);
}

#[test]
fn run_test_mangled_function_probe(){
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
            let mut command = create_command_for_bench(&benchmark, &exe, 2, 1, create_tmp_file().as_raw_fd());
            println!("{:?}", command);
            let output = command.output().unwrap();
            println!("{}", std::str::from_utf8(&*output.stderr).unwrap());
        }
        delete_probe(&format!("probe_{}:*", &bench.name));
    }
}
