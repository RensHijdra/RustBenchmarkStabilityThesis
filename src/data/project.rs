use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use cargo_toml::{Manifest, Product};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub name: String,
    pub bench_files: Vec<BenchFile>,
}

impl Project {
    pub(crate) fn store(&self) -> std::io::Result<()> {
        let serialized = serde_json::to_string(self).unwrap();

        std::fs::write(format!("{}.json", self.name), serialized)
    }

    pub fn load(project: &str) -> serde_json::Result<Project> {
        serde_json::from_str::<Project>(
            &std::fs::read_to_string(format!("{}.json", project))
                .expect(&format!("Could not read project {}.json", project)),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetProject {
    pub name: String,
    pub repo_url: String,
    pub repo_tag: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BenchFile {
    pub project: String,
    pub name: String,
    pub source: String,
    pub features: Vec<String>,
    pub benches: Vec<String>,
}

impl BenchFile {
    pub fn get_workdir(&self) -> String {
        let mut buf = Path::new(&self.source).to_path_buf();
        buf.pop();
        buf.to_str().unwrap().to_string()
    }
}

fn get_manifest(path: &PathBuf) -> Manifest {
    Manifest::from_path(path.join("Cargo.toml").as_path()).expect(
        format!(
            "Could not find Cargo.toml for {}",
            path.as_path().to_str().to_owned().unwrap()
        )
        .as_str(),
    )
}

pub fn find_benchmarks_for_project(project_name: &str) -> Project {
    let work_dir = get_workdir_for_project(project_name);
    println!(
        "Reading Cargo.toml for {} in {}",
        project_name,
        work_dir.to_str().unwrap()
    );

    let manifest = get_manifest(&work_dir);
    let mut project_bench_products = manifest.bench;

    project_bench_products.iter_mut().for_each(|product| {
        if !product.path.is_some() {
            product.path.replace(
                Path::new("benches")
                    .join(product.name.clone().unwrap())
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        };
    });

    // If there are multiple workspaces, also use all benches within those
    if manifest.workspace.is_some() {
        project_bench_products.extend(
            manifest
                .workspace
                .unwrap()
                .members
                .iter()
                .flat_map(|member| {
                    let mut bench_products = get_manifest(&work_dir.join(member)).bench;
                    bench_products.iter_mut().for_each(|prod| {
                        // Prepend the workspace name to the path
                        if prod.path.is_some() {
                            prod.path.replace(
                                Path::new(member)
                                    .join(prod.path.as_ref().unwrap())
                                    .to_str()
                                    .unwrap()
                                    .to_string(),
                            );
                        } else {
                            prod.path.replace(
                                Path::new(member)
                                    .join("benches")
                                    .join(prod.name.as_ref().unwrap())
                                    .to_str()
                                    .unwrap()
                                    .to_string(),
                            );
                        }
                    });
                    bench_products
                })
                .collect::<Vec<Product>>(),
        );
    }

    lazy_static! {
        static ref RE_BENCH_NAME: Regex = Regex::new(r"(?m)^(.+): bench$").unwrap();
    }

    let work_path = Path::new(&work_dir);

    let mut bench_files: Vec<BenchFile> = vec![];
    for product in project_bench_products {
        let product_name = product.name.unwrap();
        print!(
            "Checking benches for {:?} in file {:?}... \t",
            &product_name, &product.path
        );

        let mut command = Command::new("cargo");

        let product_path = product.path.unwrap();
        let mut abs_path = work_path.join(&product_path);
        abs_path.pop(); // remove filename from path
        println!("{:?}", abs_path);
        command
            .current_dir(abs_path.to_str().unwrap().to_string())
            .arg("bench");
        // .env("CARGO_PROFILE_BENCH_DEBUG", "true") // We need debug info to find probepoints
        // .env("CARGO_PROFILE_BENCH_LTO", "no"); // Debug info is stripped if LTO is on

        if product.required_features.len() > 0 {
            command
                .arg("--features")
                .arg(product.required_features.join(","));
        }

        command
            .arg("--bench")
            .arg(&product_name)
            .arg("--")
            .arg("--list");
        println!("{:?}", command);
        let benches = command.output().expect("could not run --bench").stdout;

        let parsed_output = std::str::from_utf8(&*benches).expect("Could not parse UTF-8");

        let benchmark_ids = RE_BENCH_NAME
            .captures_iter(parsed_output)
            .map(|c| String::from(&c[1]))
            .collect::<Vec<String>>();

        let bf = BenchFile {
            project: project_name.to_string(),
            name: product_name.to_string(),
            source: product_path.clone(),
            features: product.required_features.clone(),
            benches: benchmark_ids,
        };

        println!(
            "found {} benchmark(s) for {}",
            bf.benches.len(),
            product_name
        );
        // Only push if there are actually benchmarks found
        if bf.benches.len() != 0 {
            bench_files.push(bf)
        }
    }

    let proj = Project {
        name: project_name.to_string(),
        bench_files,
    };

    return proj;
}

pub fn get_workdir_for_project(project: &str) -> PathBuf {
    Path::new(&std::env::current_dir().unwrap())
        .join("projects")
        .join(project)
}

pub fn read_target_projects() -> Vec<TargetProject> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(Path::new("targets.csv"))
        .expect("Could not find file targets.cvs, consider adding targets");
    reader
        .deserialize::<TargetProject>()
        .filter_map(|record| record.ok())
        .collect()
}

pub(crate) fn find_all_benchmarks() -> Vec<Project> {
    let target_projects = read_target_projects();
    target_projects
        .iter()
        .map(|target| find_benchmarks_for_project(&target.name))
        .collect::<Vec<Project>>()
}

fn get_git_project(project: TargetProject) -> ExitStatus {
    Command::new("git")
        .current_dir(std::env::current_dir().unwrap().join("projects"))
        .arg("clone")
        .arg(project.repo_url)
        .arg("--depth")
        .arg("1")
        .arg("--branch")
        .arg(project.repo_tag)
        .status()
        .expect("Could not clone project.")
}

#[test]
fn run_find_benchmarks_for_project() {
    let project = find_benchmarks_for_project("prost");
    project.store().unwrap()
}

#[test]
fn parse_all_from_targets() {
    find_all_benchmarks()
        .iter()
        .for_each(|project| project.store().expect("Could not store project"));
}

#[test]
fn count_benches() {
    let num_projects = read_target_projects().len();
    let sum: usize = read_target_projects()
        .iter()
        .map(|target| Project::load(&target.name).unwrap())
        .map(|project| {
            project
                .bench_files
                .iter()
                .map(|b| b.benches.len())
                .sum::<usize>()
        })
        .sum();
    println!("{} benchmarks across {} projects", sum, num_projects);
}

#[test]
fn benches_length() {
    read_target_projects()
        .iter()
        .map(|target| Project::load(&target.name).unwrap())
        .for_each(|project| {
            println!(
                "{}: {}",
                project
                    .bench_files
                    .iter()
                    .map(|b| b.benches.len())
                    .sum::<usize>(),
                project.name
            )
        });
    println!("Done")
}

pub(crate) fn clone_projects_from_targets() {
    std::fs::create_dir_all(std::env::current_dir().unwrap().join("projects")).unwrap();

    for project in read_target_projects() {
        get_git_project(project);
    }
}

pub(crate) fn cargo_check_all_projects() {
    for project in read_target_projects() {
        let success = Command::new("cargo")
            .current_dir(get_workdir_for_project(&project.name))
            .args(&["check", "--benches", "--quiet"])
            .output()
            .unwrap()
            .status
            .success();
        if !success {
            println!("Check failed for {}", &project.name);
            println!(
                "{:?}",
                Command::new("cargo")
                    .current_dir(get_workdir_for_project(&project.name))
                    .args(&["check", "--benches"])
            );
        }
    }
    println!("Done");
}
