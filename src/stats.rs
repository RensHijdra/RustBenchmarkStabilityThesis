#![allow(unused)]

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use chrono::{NaiveDateTime};

use csv::{Trim};
use futures::{FutureExt, StreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use linux_perf_data::{PerfFileReader, PerfFileRecord, RawUserRecord, UserRecord, UserRecordType};
use linux_perf_data::linux_perf_event_reader::RawData;
use linux_perf_data::UserRecord::Raw;
use nix::dir::Dir;
use ra_ap_ide::ExprFillDefaultMode::Default;
use rand::distributions::{Uniform};
use rand::{thread_rng, Rng};
use regex::Regex;
use rstats::{noop, MStats, Median, Stats, RE};
use serde::{Deserialize, Serialize};
use statrs::assert_almost_eq;
use statrs::distribution::{Beta, ContinuousCDF};
use crate::project::read_target_projects;

#[derive(Debug, Deserialize)]
struct Datapoint {
    //1.001070459;1001070459;ns;duration_time;1001070459;100,00;;
    duration: f64,
    value: f64,
    unit: Option<String>,
    event_name: String,
    counter_runtime: u64,
    percentage_of_measurement: f64,
    metric: Option<f64>,
    metric_unit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DataFile {
    datapoints: Vec<Datapoint>,
}

#[derive(Debug, Serialize)]
struct Statistic {
    project: String,
    benchmark: String,
    id: String,
    datapoint: String,
    samples: usize,
    min: f64,
    max: f64,
    mean: f64,
    median: f64,
    q1: f64,
    q3: f64,
    mad: f64,
    rmad: f64,
    std: f64,
    var: f64,
    rciw_boot: f64,
    rciw_mjhd: f64,
}



struct PowerFile{name:  String, ts: i64}
struct PerfFile{name:  String, ts: i64}


#[test]
fn test_powerfile_to_benchname() {
    let PowerFile{name, ts} = powerfile_to_benchname("simple_1m_zeroes_1m_1681874840353873.txt");
            assert_eq!(name, "simple_1m_zeroes_1m");
            assert_eq!(ts, 1681874840353873_i64)
}

#[test]
fn test_perffile_to_benchname() {
    let PerfFile{name, ts} = perffile_to_benchname("simple_1m_ones_1m.profraw.2023042100382591");
    assert_eq!(name, "simple_1m_ones_1m");
    assert_eq!(ts, 2023042100382591_i64);
}

fn powerfile_to_benchname(file_name: &str) -> PowerFile {
    lazy_static! {
        static ref POWERFILE_PATTERN: Regex = Regex::new(r"(.*?)_([0-9]{16}).txt").unwrap();
    }
    POWERFILE_PATTERN.captures_iter(file_name).map(|cap| PowerFile{name: String::from(&cap[1]), ts: i64::from_str(&cap[2]).unwrap()}).next().unwrap()
}

fn perffile_to_benchname(file_name: &str) -> PerfFile {
    lazy_static! {
        static ref PERFFILE_PATTERN: Regex = Regex::new(r"(.*?).profraw.([0-9]{16})").unwrap();
    }
    // println!("{:?}", PERFFILE_PATTERN.captures_iter(file_name));
    PERFFILE_PATTERN.captures_iter(file_name).map(|cap| PerfFile{name: String::from(&cap[1]), ts: i64::from_str(&cap[2]).unwrap()}).next().unwrap()
}


#[test]
fn test_parse_perf_files() {
    parse_perf_files("/home/rens/thesis/data/round1/round1/data");
}

#[derive(Debug)]
struct EnergyReading {
    ts: NaiveDateTime,
    energy: i64
}

fn parse_perf_files(filedir: &str) {
    // TODO collect list of benchmarks
    let result = fs::read_dir(filedir).unwrap();

    let mut perf: Vec<(PathBuf, PerfFile)> = Vec::new();
    let mut pow: Vec<(PathBuf, PowerFile)> = Vec::new();


    for benchfile in result {
        let benchfile = benchfile.unwrap();
        let dir = fs::read_dir(benchfile.path()).unwrap();
        for bench_dir in dir {
            for perf_file in fs::read_dir(bench_dir.unwrap().path()).unwrap() {
                let perf_file = perf_file.unwrap();
                if perf_file.file_name().to_str().unwrap().ends_with(".txt") {
                    pow.push((perf_file.path(), powerfile_to_benchname(perf_file.file_name().to_str().unwrap())));
                } else if perf_file.file_name().to_str().unwrap().contains(".profraw.") {
                    perf.push((perf_file.path(), perffile_to_benchname(perf_file.file_name().to_str().unwrap())));
                }
            }
        }
    }

    println!("Retrieved {:?} power files total", pow.len());
    println!("Retrieved {:?} perf files total", perf.len());
    // TODO perf script each benchmark

    lazy_static! {
        static ref POWER_PAT: Regex = Regex::new(r"(?m)([0-9]{13})\n([0-9]{2,20})\n([0-9]{2,20})").unwrap();
    }

    // Process all energy data
    let mut flatten = pow.iter().map(|(path, PowerFile { name, ts })| {
        let readings = POWER_PAT.captures_iter(&String::from_utf8(fs::read(path).unwrap()).unwrap()).map(|cap| {
            let ts = chrono::NaiveDateTime::from_timestamp_millis(i64::from_str(&cap[1]).unwrap()).unwrap();
            let energy = i64::from_str(&cap[3]).unwrap() - i64::from_str(&cap[2]).unwrap();
            EnergyReading { ts, energy }
        }).collect::<Vec<EnergyReading>>();

        (name.clone(), readings)
    }).collect::<Vec<(String, Vec<EnergyReading>)>>();

    let mut energy_map = HashMap::new();
    for (k, mut v) in flatten {
        energy_map.entry(k).or_insert_with(Vec::<EnergyReading>::new).append(&mut v)
    }


    // let mut writer = csv::WriterBuilder::new().from_writer(std::io::stdout());
    // for (k, v) in energy_map {
    //     let mut data = v.iter().map(|reading| reading.energy as i32 as f64).collect::<Vec<f64>>();
    //     let statistic = data_to_statistics("idk", "nvm", &k, "energy", &mut data);
    //     writer.serialize(statistic);
    // }

    // println!("{:?}", energy_map);


    // println!("{:?}", statistic);


    // flatten.sort_by(|(name, _), (other_name, _)| name.cmp(other_name));
    // let flatten = flatten.iter().group_by(|(name, _)| name).;
    for (path, PerfFile{name, ts}) in perf {
        perf_script(path);
    }

    // TODO Convert to iters/instr per benchmark
    // TODO Enumerate iters/instrs per benchmark
    // TODO Combine with power consumption
    // TODO Output/save
}

#[test]
fn test_perf_script() {
    let data = "/home/rens/";
}


struct ScriptLine {}
fn perf_script(file: PathBuf) {

    let file = std::fs::File::open(file).unwrap();
    let reader = std::io::BufReader::new(file);
    let perfreader = PerfFileReader::parse_file(reader);
    if perfreader.is_err() {
        return;
    }
    let PerfFileReader { mut perf_file, mut record_iter } = perfreader.unwrap();
    let event_names: Vec<_> =
        perf_file.event_attributes().iter().filter_map(linux_perf_data::AttributeDescription::name).collect();
    println!("perf events: {}", event_names.join(", "));

    while let Some(record) = record_iter.next_record(&mut perf_file).unwrap() {
        match record {
            PerfFileRecord::EventRecord { attr_index, record } => {
                let record_type = record.record_type;
                let result = record.parse();
                if result.is_err() {
                    // println!("{:?}", record);
                    // println!("{:?}", result);
                    break;
                }
                let parsed_record = result.unwrap();
                println!("{:?} for event {}: {:?}", record_type, attr_index, parsed_record);
            }
            PerfFileRecord::UserRecord(record) => {
                let record_type = record.record_type;
                let parsed_record = record.parse().unwrap();
                // println!("{:?}: {:?}", record_type, parsed_record);
                match parsed_record {
                    UserRecord::ThreadMap(_) => {}
                    Raw(rec) => match rec { RawUserRecord { record_type, endian, misc, data } => {
                        println!("{:?}", record_type);
                        println!("{}", misc);
                        println!("{:?}", endian);
                        match data {
                            RawData::Single(data) => println!("{:?}", data),
                            RawData::Split(first, second) => println!("{:?}, {:?}", first, second)
                        }

                    }}
                    l => println!("Unmatched {:?}", l)
                }
            }
        }
    }
    // let output = Command::new("perf").arg("data").arg("convert").arg("-i").arg(file.to_str().unwrap()).arg("--to-json").arg(format!("{}.json", file.to_str().unwrap())).output().unwrap();

    // if !output.status.success() {
        // error_chain::bail!("Command executed with failing error code");
    // }


    // let result = String::from_utf8(fs::read(format!("{}.json", file.to_str().unwrap())).unwrap()).unwrap();

    // println!("{}", result);
    // let pattern = Regex::new(r"(?x)
    //                            ([0-9a-fA-F]+)
    //                            (.*)           ").unwrap();

    // println!("{}", String::from_utf8(output.stderr).unwrap());
    // String::from_utf8(output.stdout)?
    //     .lines()
    //     .filter_map(|line| pattern.captures(line))
    //     .map(|cap| {
    //         ScriptLine {
    //             hash: cap[1].to_string(),
    //             message: cap[2].trim().to_string(),
    //         }
    //     })
    //     .take(5)
    //     .for_each(|x| println!("{:?}", x));
    //
    // Ok(())
}


fn stats_for_project(project: &str) {
    let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(std::io::stdout());
    let result = fs::read_dir(format!("data/{}", project));
    if result.is_err(){
        return;
    }
    for benchmark_dir_entry in result.unwrap()
    {
        if let Ok(benchmark_dir) = benchmark_dir_entry {
            for bench_file_entry in benchmark_dir.path().read_dir().unwrap() {
                if let Ok(benchmark_file) = bench_file_entry {
                    // Preprocess. perf outputs floats with comma for some metrics. Serde cannot handle this
                    // println!("{:?}", benchmark_file.path());
                    let file = fs::read_to_string(benchmark_file.path())
                        .unwrap()
                        .replace(",", ".")
                        .replace("<not counted>", "-1");
                    let mut reader = csv::ReaderBuilder::new()
                        .comment(Some(b'#'))
                        .delimiter(b';')
                        .trim(Trim::All)
                        .has_headers(false)
                        .from_reader(file.as_bytes());

                    // let mut data = fs::read_to_string(file.path())
                    //     .unwrap()
                    //     .split("\n")
                    //     .map(|line| {
                    //         f64::from_str(line.split(", ").last().unwrap()).unwrap()
                    //     })
                    //     .collect::<Vec<f64>>();
                    let hash_map = reader
                        .deserialize()
                        .map(|point| point.unwrap())
                        .map(|p: Datapoint| (p.event_name.clone(), p))
                        .into_group_map();

                    let x = hash_map
                        .iter()
                        .filter(|(k, _)| k.starts_with("probe_"))
                        .filter(|(_, v)| v.iter().map(|v| v.value).sum::<f64>() != 0.0)
                        .collect::<Vec<(&String, &Vec<Datapoint>)>>();
                    if x.len() == 0 {
                        // break;
                        println!(
                            "{}/{:?}/{:?}",
                            project,
                            benchmark_dir.file_name(),
                            benchmark_file.file_name()
                        );
                    }
                    // for key in hash_map.keys() {
                    //     // if key.starts_with("probe_") {
                    //         let option = hash_map.get(key);
                    //         if option.is_some(){
                    //             let mut vec: Vec<f64>;
                    //             if key == "cpu_core/r19c/" {
                    //                 //
                    //                 vec = option.unwrap().iter().map(|p| ((p.value as u64) << 14 >> 22) as f64).collect::<Vec<f64>>()
                    //             } else {
                    //             vec = option.unwrap().iter().map(|p| p.value).collect::<Vec<f64>>();
                    //             }
                    //
                    //             let statistic = data_to_statistics(project, benchmark_dir.file_name().to_str().unwrap(), benchmark_file.file_name().to_str().unwrap(), key,&mut vec);
                    //             // if statistic.min == 0.0 && statistic.max == 0.0 {
                    //             //     break;
                    //             // }
                    //
                    //             wtr.serialize(statistic);
                    //         }
                    //     // }
                    // }
                    // let output = data_to_statistics(&mut data);
                    // println!("{:?}", output);
                }
            }
        }
    }
}

fn data_to_statistics(project: &str, benchmark: &str, id: &str, datapoint: &str, mut data: &mut Vec<f64>) -> Statistic {
    let MStats {
        centre: mean,
        dispersion: std,
    } = data.ameanstd().unwrap();
    let (q1, median, q3) = data.quartiles();
    let median = data.median(&mut noop).unwrap();
    let samples = data.len();

    // Only PartialOrd is implemented and not Ord since f64 is only partially ordered
    let min = *(&data)
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max = *(&data)
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let mad = data.mad(median, &mut noop).unwrap();
    let rmad = mad / median;

    let var = std * std;

    let confidence_level = 0.99;

    // Harrel Davis RCIW
    let lo = (&data).hd(confidence_level + ((1.0 - confidence_level) / 2.0));
    let hi = (&data).hd((1.0 - confidence_level) / 2.0);
    let rciw_mjhd = (lo - hi) / (&data).hd(0.5);

    // Bootstrap RCIW
    let bootstrap_samples = bootstrap(data, 10000);

    let lo =
        bootstrap_samples.percentile((confidence_level + ((1.0 - confidence_level) / 2.0)) * 100.0);
    let hi = bootstrap_samples.percentile(((1.0 - confidence_level) / 2.0) * 100.0);
    let rciw_boot = (lo - hi) / mean;

    let output = Statistic {
        project: project.to_string(),
        benchmark: benchmark.to_string(),
        id: id.to_string(),
        datapoint: datapoint.to_string(),
        samples,
        min,
        max,
        mean,
        median,
        q1,
        q3,
        mad,
        rmad,
        std,
        var,
        rciw_boot,
        rciw_mjhd,
    };
    output
}

fn bootstrap(data: &Vec<f64>, samples: usize) -> Vec<f64> {
    let rng = thread_rng();

    rng.sample_iter(Uniform::new(0, data.len()))
        .map(|idx| data[idx])
        .take(samples)
        .collect()
    // vec
}

trait Quantile<T> {
    fn quartiles(&self) -> (T, T, T);
    fn percentile(&self, percentile: f64) -> T;
    fn hd(&self, quantile: f64) -> T;
}

impl Quantile<f64> for Vec<f64> {
    fn percentile(&self, pct: f64) -> f64 {
        let mut tmp = self.to_vec();
        local_sort(&mut tmp);
        percentile_of_sorted(&tmp, pct)
    }

    fn quartiles(&self) -> (f64, f64, f64) {
        let mut tmp = self.to_vec();
        local_sort(&mut tmp);
        let first = 25_f64;
        let a = percentile_of_sorted(&tmp, first);
        let second = 50_f64;
        let b = percentile_of_sorted(&tmp, second);
        let third = 75_f64;
        let c = percentile_of_sorted(&tmp, third);
        (a, b, c)
    }

    /*
        if nargin<2; q=.5;end
    n=length(x);
    m1=(n+1).*q;
    m2=(n+1).*(1-q);
    vec=1:length(x);
    w=betacdf(vec./n,m1,m2)-betacdf((vec-1)./n,m1,m2);
    y=sort(x);
    thetaq=sum(w(:).*y(:));
         */
    fn hd(&self, percentile: f64) -> f64 {
        let n = self.len() as f64;
        let m1 = (n + 1.0) * percentile;
        let m2 = (n + 1.0) * (1.0 - percentile);
        let beta = Beta::new(m1, m2).unwrap();
        let vec = (1..=n as i32)
            .map(|x| beta.cdf(x as f64 / n) - beta.cdf((x as f64 - 1.0) / n))
            .collect::<Vec<f64>>();
        let mut tmp = self.to_vec();
        local_sort(&mut tmp);
        tmp.iter().zip(vec).map(|(w, y)| w * y).sum()
    }
}

fn local_sort(v: &mut [f64]) {
    v.sort_by(|x: &f64, y: &f64| x.total_cmp(y));
}

// Helper function: extract a value representing the `pct` percentile of a sorted sample-set, using
// linear interpolation. If samples are not sorted, return nonsensical value.
fn percentile_of_sorted(sorted_samples: &[f64], pct: f64) -> f64 {
    assert!(!sorted_samples.is_empty());
    if sorted_samples.len() == 1 {
        return sorted_samples[0];
    }
    let zero: f64 = 0.0;
    assert!(zero <= pct);
    let hundred = 100_f64;
    assert!(pct <= hundred);
    if pct == hundred {
        return sorted_samples[sorted_samples.len() - 1];
    }
    let length = (sorted_samples.len() - 1) as f64;
    let rank = (pct / hundred) * length;
    let lrank = rank.floor();
    let d = rank - lrank;
    let n = lrank as usize;
    let lo = sorted_samples[n];
    let hi = sorted_samples[n + 1];
    lo + (hi - lo) * d
}

// #[test]
// fn test_dirs() {
//     for project in read_target_projects() {
//         stats_for_project(&project.name.replace("-","_"));
//     }
// }

#[test]
fn test_harreldavis() {
    let a: Vec<f64> = vec![
        77.0, 87., 88., 114., 151., 210., 219., 246., 253., 262., 296., 299., 306., 376., 428.,
        515., 666., 1310., 2611.,
    ];
    assert_almost_eq!(a.hd(0.5), 271.72120054908913, 0.00000001);
}

// #[test]
// fn test_profdata() {
//     use linux_perf_data::{AttributeDescription, PerfFileReader, PerfFileRecord};
//
//     let file = std::fs::File::open("perf.data").unwrap();
//     let reader = std::io::BufReader::new(file);
//     let PerfFileReader { mut perf_file, mut record_iter } = PerfFileReader::parse_file(reader).unwrap();
//     let event_names: Vec<_> =
//         perf_file.event_attributes().iter().filter_map(AttributeDescription::name).collect();
//     println!("perf events: {}", event_names.join(", "));
//
//     while let Some(record) = record_iter.next_record(&mut perf_file).unwrap() {
//         match record {
//             PerfFileRecord::EventRecord { attr_index, record } => {
//                 let record_type = record.record_type;
//                 let parsed_record = record.parse().unwrap();
//                 println!("{:?} for event {}: {:?}", record_type, attr_index, parsed_record);
//             }
//             PerfFileRecord::UserRecord(record) => {
//                 let record_type = record.record_type;
//                 let parsed_record = record.parse().unwrap();
//                 println!("{:?}: {:?}", record_type, parsed_record);
//             }
//         }
//     }
// }

fn main() {}
