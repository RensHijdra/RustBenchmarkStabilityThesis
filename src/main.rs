use std::env;
use std::process::Command;

use caps::{CapSet, Capability, CapsHashSet};
use clap::Parser;
use crate::coverage::{gather_coverage, gather_instructions};

use crate::data::project::{
    cargo_check_all_projects, clone_projects_from_targets, find_all_benchmarks,
};

mod collect;
mod coverage;
mod data;

#[derive(Parser, Debug)]
#[command(name = "power")]
#[command(bin_name = "power")]
enum Cli {
    #[command(name = "run")]
    Experiment(ExperimentSettings),
    #[command(subcommand)]
    Project(ProjectCommand),
    #[command(subcommand, name = "stat")]
    Statistics(StatisticsCommand),
    #[command(about = "Run the necessary commands to set up the environment. Needs root.")]
    Prep,
    #[command(about = "Run `cargo check --benches` on all projects")]
    Check,
    #[command(about= "Collect coverage data from all projects.")]
    Coverage,
    #[command()]
    Instructions,
}

#[derive(clap::Args, Debug)]
#[command(author, version, about, long_about = None)]
struct ExperimentSettings {
    #[arg(short, default_value = "30")]
    repetitions: usize,

    #[arg(short, long, default_value = "30")]
    measurement_time: u64,

    #[arg(short, long, default_value = "5")]
    warmup_time: u64,

    #[arg(short, long, default_value = "300")]
    sample_size: u64,

    #[arg(long)]
    no_rmit: bool
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
        Cli::Experiment(settings) => {
            check_capabilities();
            collect::run(
                settings.repetitions,
                settings.measurement_time,
                settings.warmup_time,
                settings.sample_size,
                settings.no_rmit
            )
        }
        Cli::Project(subcommand) => match subcommand {
            ProjectCommand::Parse => {
                find_all_benchmarks()
                    .iter()
                    .for_each(|project| project.store().expect("Could not store project"));
            }
            ProjectCommand::Download => {
                println!("Cloning projects that were found in targets.csv");
                clone_projects_from_targets();
            }
        },
        Cli::Statistics(subcommand) => match subcommand {
            StatisticsCommand::Parse => {}
            StatisticsCommand::Merge => {}
        },
        Cli::Prep => {
            println!("{:?}", CapSet::Effective);
            // Check own process rights
            let executable = env::current_exe().unwrap();
            match Command::new("setcap")
                .arg("CAP_SYS_RAWIO=ep")
                .arg(&executable)
                .output()
            {
                Ok(out) => {
                    if out.status.success() {
                        println!(
                            "Succesfully set capability CAP_SETFCAP for {}",
                            &executable.to_str().unwrap()
                        )
                    } else {
                        panic!("Failed to set capability for self, try running with sudo")
                    }
                }
                Err(err) => panic!("Error setting own capabilities: {}", err),
            }
        }
        Cli::Check => {
            cargo_check_all_projects();
        },
        Cli::Coverage => {
            gather_coverage();
        },
        Cli::Instructions => {
            gather_instructions();
        }
    }
}

fn check_capabilities() {
    check_or_panic_rawio(CapSet::Permitted);
    check_or_panic_rawio(CapSet::Effective);

    match caps::set(
        None,
        CapSet::Inheritable,
        &CapsHashSet::from([Capability::CAP_SYS_RAWIO]),
    ) {
        Ok(_) => println!("Successfully set rawio to inheritable"),
        Err(err) => panic!("Failed to set rawio to inheritable with error: {:?}", err),
    }

    match caps::set(
        None,
        CapSet::Ambient,
        &CapsHashSet::from([Capability::CAP_SYS_RAWIO]),
    ) {
        Ok(_) => println!("Successfully set rawio to ambient"),
        Err(err) => panic!("Failed to set rawio to ambient with error: {:?}", err),
    }
}

fn check_or_panic_rawio(set: CapSet) {
    match caps::has_cap(None, set, Capability::CAP_SYS_RAWIO) {
        Ok(bool) => {
            if !bool {
                panic!(
                    "CAP_SYS_RAWIO is not in the {:?} set. Did you run `sudo {} elevate`?",
                    set,
                    env::current_exe()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                )
            }
        }
        Err(err) => panic!("Failed to read {:?} capabilities set. {:?}", set, err),
    }
}
