use std::{fs, io};
use std::iter::Map;
use std::path::Iter;
use std::str::FromStr;

use itertools::Itertools;
use rand::{Rng, thread_rng};
use rand::distributions::{DistIter, Uniform};
use rand::rngs::ThreadRng;
use rstats::{MedError, Median, MStats, noop, Stats};
use serde::Serialize;
use statrs::assert_almost_eq;
use statrs::distribution::{Beta, ContinuousCDF};

#[derive(Debug, Serialize)]
struct Statistic {
    project: String,
    group: String,
    id: String,
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

fn stats_for_project(project: &str) {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    for dir in fs::read_dir(format!("data/{}", project)).expect(&format!("Could not find project {} in data/", project)) {
        match dir {
            Ok(entry) => {
                for file_ in entry.path().read_dir().unwrap() {
                    match file_ {
                        Ok(file) => {
                            let mut data = fs::read_to_string(file.path()).unwrap()
                                .split("\n")
                                .map(|line| f64::from_str(line.split(", ").last().unwrap()).unwrap())
                                .collect::<Vec<f64>>();
                            let MStats { centre: mean, dispersion: std } = data.ameanstd().unwrap();
                            let (q1, median, q3) = data.quartiles();
                            let median = data.median(&mut noop).unwrap();
                            let samples = data.len();

                            // Only PartialOrd is implemented and not Ord since f64 is only partially ordered
                            let min = *(&data).iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
                            let max = *(&data).iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

                            let mad = data.mad(median, &mut noop).unwrap();
                            let rmad = mad / median;

                            let var = std * std;

                            let filename = file.file_name().to_str().unwrap().replace(".csv", "");
                            let group = entry.file_name().to_str().unwrap().to_string();

                            let confidence_level = 0.99;

                            // Harrel Davis RCIW
                            let lo = (&data).hd(confidence_level + ((1.0 - confidence_level) / 2.0));
                            let hi = (&data).hd((1.0 - confidence_level) / 2.0);
                            let rciw_mjhd = (lo - hi) / (&data).hd(0.5);

                            // Bootstrap RCIW
                            let bootstrap_samples = bootstrap(data, 10000);

                            let lo = bootstrap_samples.percentile((confidence_level + ((1.0 - confidence_level) / 2.0)) * 100.0);
                            let hi = bootstrap_samples.percentile(((1.0 - confidence_level) / 2.0) * 100.0);
                            let rciw_boot = (lo - hi) / mean;

                            let output = Statistic { project: project.to_string(), samples, group, id: filename, min, max, mean, median, q1, q3, mad, rmad, std, var, rciw_boot, rciw_mjhd };

                            // println!("{:?}", vec);
                            wtr.serialize(output);
                            // println!("{:?}", output);
                            // println!("{{}, {}, {:?}, {}, {:?}, {}}", filename, min, q1, median, q3, max);
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(_) => {}
        }
    }
}

fn bootstrap(data: Vec<f64>, samples: usize) -> Vec<f64> {
    let mut rng = thread_rng();

    rng.sample_iter(Uniform::new(0, data.len())).map(|idx| data[idx]).take(samples).collect()
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
        let vec = (1..=n as i32).map(|x| beta.cdf(x as f64 / n) - beta.cdf((x as f64 - 1.0) / n)).collect::<Vec<f64>>();
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

#[test]
fn test_dirs() {
    stats_for_project("pyo3");
}

#[test]
fn test_harreldavis() {
    let a: Vec<f64> = vec![77.0, 87., 88., 114., 151., 210., 219., 246., 253., 262., 296., 299., 306., 376., 428., 515., 666., 1310., 2611.];
    assert_almost_eq!(a.hd(0.5), 271.72120054908913, 0.00000001);
}