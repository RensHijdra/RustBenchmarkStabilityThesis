use std::env;
use std::process::Command;

use caps::{Capability, CapSet, CapsHashSet};
use caps::errors::CapsError;
// #![allow(dead_code, unused)]
//
//
// use std::{thread, time};
//
// use crates_io_api::SyncClient;
// use lazy_static::lazy_static;
// use octocrab::models::Repository;
// use perf_event_open_sys::bindings::{__u32, __u64};
// use regex::Regex;
// use tokio::runtime::Runtime;
//
// fn main() {
//     list_rev_deps(1);
// }
//
// fn list_rev_deps(page: u64) {
//     let timeout = time::Duration::from_secs(30);
//
//     thread::sleep(timeout);
//     // Instantiate the client.
//     let client = SyncClient::new(
//         "Mozilla/5.0 (Linux; Android 9; SAMSUNG SM-A730F) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/15.0 Chrome/94.0.4603.0 Mobile Safari/537.36",
//         std::time::Duration::from_millis(1000),
//     ).unwrap();
//     let rt = Runtime::new().unwrap();
//     // Retrieve summary data. // TODO add more pages?
//     let rev_deps = client
//         .crate_reverse_dependencies_page("criterion", page)
//         .unwrap();
//     for c in &rev_deps.dependencies {
//         let total_crate_downloads: u64;
//         let reverse_dep_count: usize;
//         let repository: String;
//         let crate_name: &String = &c.crate_version.crate_name;
//
//         // Crate data
//         match client.full_crate(&c.crate_version.crate_name, false) {
//             Ok(full_crate) => {
//                 total_crate_downloads = full_crate.total_downloads;
//                 reverse_dep_count = full_crate.reverse_dependencies.dependencies.len();
//                 repository = match full_crate.repository {
//                     None => {
//                         continue;
//                     }
//                     Some(rep) => rep,
//                 };
//             }
//             Err(err) => {
//                 println!("{}", err);
//                 continue;
//             }
//         };
//
//         // Repo data
//         lazy_static! {
//             static ref RE: Regex =
//                 Regex::new(r"https://github.com/([\w-]*)/([\w-]*)(?:(?:/.*)|(?:\.git))?").unwrap();
//         }
//         let caps = match RE.captures_iter(&repository).next() {
//             None => {
//                 continue;
//             }
//             Some(cap) => cap,
//         };
//
//         let repo: Repository =
//             match rt.block_on(octocrab::instance().repos(&caps[1], &caps[2]).get()) {
//                 Ok(repo) => repo,
//                 Err(_) => {
//                     continue;
//                 }
//             };
//
//         let stars = repo.stargazers_count.unwrap();
//         let forks = repo.forks_count.unwrap();
//         let watchers = repo.watchers_count.unwrap();
//         let subscribers = repo.subscribers_count.unwrap();
//         let archived = repo.archived.unwrap();
//         let last_edit = repo.pushed_at.unwrap();
//
//         // Assume the following to be true since it is published as a crate.
//         let has_toml = true;
//
//         // Output as CSV
//         println!(
//             "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
//             crate_name,
//             repository,
//             forks,
//             stars,
//             watchers,
//             subscribers,
//             archived,
//             last_edit,
//             total_crate_downloads,
//             reverse_dep_count,
//             has_toml
//         );
//     }
// }
use clap::Parser;
use nix::libc;
use rstats::Printing;

use crate::project::clone_projects_from_targets;

mod collect;
mod project;
mod stats;
mod parse;
mod probe;

#[derive(Parser, Debug)] // requires `derive` feature
#[command(name = "power")]
#[command(bin_name = "power")]
enum Cli {
    #[command(name = "run")]
    RunExperiment(ExperimentSettings),
    #[command(subcommand)]
    Project(ProjectCommand),
    #[command(subcommand, name = "stat")]
    Statistics(StatisticsCommand),
    #[command()]
    Elevate,
}

#[derive(clap::Args, Debug)]
#[command(author, version, about, long_about = None)]
struct ExperimentSettings {
    #[arg(short, default_value = "30")]
    iterations: usize,

    #[arg(short, long, default_value = "30")]
    repetitions: usize,

    #[arg(short = 't', long, default_value = "30")]
    profile_time: u64,

    #[arg(short, long, default_value = "1")]
    cpu: usize,
}

#[derive(clap::Subcommand, Debug)]
enum ProjectCommand {
    Parse,
    Download,
}

#[derive(clap::Subcommand, Debug)]
enum StatisticsCommand {
    Parse,
    Merge,
}


fn main() {
    let parse = Cli::parse();
    match parse {
        Cli::RunExperiment(settings) => {
            check_capabilities();
            collect::run(settings.repetitions, settings.iterations, settings.profile_time, settings.cpu)
        }
        Cli::Project(subcommand) => {
            match subcommand {
                ProjectCommand::Parse => {
                    project::find_all_benchmarks()
                        .iter()
                        .for_each(|project| project.store().expect("Could not store project"));
                }
                ProjectCommand::Download => {
                    println!("Cloning projects that were found in targets.csv");
                    clone_projects_from_targets();
                }
            }
        }
        Cli::Statistics(subcommand) => {
            match subcommand {
                StatisticsCommand::Parse => {}
                StatisticsCommand::Merge => {}
            }
        }
        Cli::Elevate => {
            println!("{:?}", CapSet::Effective);
            // Check own process rights
            let executable = env::current_exe().unwrap();
            match Command::new("setcap").arg("CAP_SYS_RAWIO=ep").arg(&executable).output() {
                Ok(out) => {
                    if out.status.success() {
                        println!("Succesfully set capability CAP_SETFCAP for {}", &executable.to_str().unwrap())
                    } else {
                        panic!("Failed to set capability for self, try running with sudo")
                    }
                }
                Err(err) => panic!("Error setting own capabilities: {}", err)
            }
        }
    }
}


fn check_capabilities() {
    check_or_panic_rawio(CapSet::Permitted);
    check_or_panic_rawio(CapSet::Effective);

    match caps::set(None, CapSet::Inheritable, &CapsHashSet::from([Capability::CAP_SYS_RAWIO])) {
        Ok(_) => println!("Successfully set rawio to inheritable"),
        Err(err) => panic!("Failed to set rawio to inheritable with error: {}", err)
    }

    match caps::set(None, CapSet::Ambient, &CapsHashSet::from([Capability::CAP_SYS_RAWIO])) {
        Ok(_) => println!("Successfully set rawio to ambient"),
        Err(err) => panic!("Failed to set rawio to ambient with error: {}", err)
    }
}

fn check_or_panic_rawio(set: CapSet) {
    match caps::has_cap(None, set, Capability::CAP_SYS_RAWIO) {
        Ok(bool) => if !bool {
            panic!("CAP_SYS_RAWIO is not in the {:?} set. Did you run `sudo {} elevate`?", set, env::current_exe().unwrap().file_name().unwrap().to_str().unwrap())
        }
        Err(err) => panic!("Failed to read {:?} capabilities set. {}", set, err)
    }
}