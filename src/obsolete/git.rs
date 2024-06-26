/*#![allow(dead_code, unused)]


use std::{thread, time};

use crates_io_api::SyncClient;
use lazy_static::lazy_static;
use octocrab::models::Repository;
use perf_event_open_sys::bindings::{__u32, __u64};
use regex::Regex;
use tokio::runtime::Runtime;

fn main() {
    list_rev_deps(1);
}

fn list_rev_deps(page: u64) {
    let timeout = time::Duration::from_secs(30);

    thread::sleep(timeout);
    // Instantiate the client.
    let client = SyncClient::new(
        "Mozilla/5.0 (Linux; Android 9; SAMSUNG SM-A730F) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/15.0 Chrome/94.0.4603.0 Mobile Safari/537.36",
        std::time::Duration::from_millis(1000),
    ).unwrap();
    let rt = Runtime::new().unwrap();
    // Retrieve summary data. // TODO add more pages?
    let rev_deps = client
        .crate_reverse_dependencies_page("criterion", page)
        .unwrap();
    for c in &rev_deps.dependencies {
        let total_crate_downloads: u64;
        let reverse_dep_count: usize;
        let repository: String;
        let crate_name: &String = &c.crate_version.crate_name;

        // Crate data
        match client.full_crate(&c.crate_version.crate_name, false) {
            Ok(full_crate) => {
                total_crate_downloads = full_crate.total_downloads;
                reverse_dep_count = full_crate.reverse_dependencies.dependencies.len();
                repository = match full_crate.repository {
                    None => {
                        continue;
                    }
                    Some(rep) => rep,
                };
            }
            Err(err) => {
                println!("{}", err);
                continue;
            }
        };

        // Repo data
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"https://github.com/([\w-]*)/([\w-]*)(?:(?:/.*)|(?:\.git))?").unwrap();
        }
        let caps = match RE.captures_iter(&repository).next() {
            None => {
                continue;
            }
            Some(cap) => cap,
        };

        let repo: Repository =
            match rt.block_on(octocrab::instance().repos(&caps[1], &caps[2]).get()) {
                Ok(repo) => repo,
                Err(_) => {
                    continue;
                }
            };

        let stars = repo.stargazers_count.unwrap();
        let forks = repo.forks_count.unwrap();
        let watchers = repo.watchers_count.unwrap();
        let subscribers = repo.subscribers_count.unwrap();
        let archived = repo.archived.unwrap();
        let last_edit = repo.pushed_at.unwrap();

        // Assume the following to be true since it is published as a crate.
        let has_toml = true;

        // Output as CSV
        println!(
            "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
            crate_name,
            repository,
            forks,
            stars,
            watchers,
            subscribers,
            archived,
            last_edit,
            total_crate_downloads,
            reverse_dep_count,
            has_toml
        );
    }
}*/
