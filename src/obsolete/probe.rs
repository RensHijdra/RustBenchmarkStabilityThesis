use std::process::Command;

use lazy_static::lazy_static;
use ra_ap_hir::known::{ str};
use regex::Regex;
use crate::collect::compile_benchmark_file;

use crate::data::project::{
    get_workdir_for_project, BenchFile, Project,
};

pub(crate) fn find_mangled_functions(executable_path: &str) -> Vec<String> {
    lazy_static! {
        static ref RE_MANGLED_FUNCS: Regex =
            Regex::new(r"(?m)^[[:xdigit:]]*\st\s(.*Bencher.{1,4}iter.*)$").unwrap();
    }

    let vec = Command::new("nm")
        .arg("-a")
        .arg(executable_path)
        .output()
        .unwrap()
        .stdout;
    let output = String::from_utf8(vec).unwrap();

    RE_MANGLED_FUNCS
        .captures_iter(&output)
        .map(|capture| String::from(&capture[1]))
        .collect()
}

#[test]
fn test_find_mangled_functions() {
    let project = Project::load("ahash").unwrap();
    let bench = project
        .bench_files
        .iter()
        .filter(|p| p.name == "ahash")
        .next()
        .expect("Are the projects loaded?");
    let exe = compile_benchmark_file(&bench, None, None, None, None).unwrap();
    let mangled_functions = find_mangled_functions(&exe);
    println!("{:?}", mangled_functions);
    println!("{}", mangled_functions.len());
    assert!(mangled_functions.len() > 0);
}

pub(crate) fn create_probe_for_mangled_functions(
    function_names: &Vec<String>,
    executable: &str,
    bench: &BenchFile,
) -> bool {
    function_names
        .iter()
        .map(|function| {
            let result = Command::new("perf")
                .arg("probe")
                .arg("-f") // Force probes with the same name
                .arg("-x")
                .arg(executable)
                .arg("--add")
                .arg(format!(
                    "{}={} self->iters",
                    bench.get_clean_name(),
                    function
                ))
                .status();
            if result.is_err() {
                // Some versions require iters to be accessed by . instead of ->
                Command::new("perf")
                    .arg("probe")
                    .arg("-f") // Force probes with the same name
                    .arg("-x")
                    .arg(executable)
                    .arg("--add")
                    .arg(format!(
                        "{}={} self.iters",
                        bench.get_clean_name(),
                        function
                    ))
                    .status()
                    .unwrap()
            } else {
                result.unwrap()
            }
        })
        .all(|status| status.success())
}

pub(crate) fn delete_probe(probe: &str) -> bool {
    Command::new("perf")
        .arg("probe")
        .arg("-d")
        .arg(probe)
        .output()
        .is_ok()
}
