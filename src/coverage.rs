use std::{env, fs};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use crate::collect::compile_benchmark_file;
use crate::data::llvmcovdata::{Filter, LlvmCovData};
use crate::data::project::{BenchFile, Project, read_target_projects};
use crate::data::syn_visit::visit_function_syn;


fn compile_for_coverage(benchmark_file: &BenchFile) -> Option<String> {
    let exe = compile_benchmark_file(
        &benchmark_file,
        Some("+nightly".to_string()),
        Some(vec!["-Zbuild-std", "--target=x86_64-unknown-linux-gnu"]),
        Some(vec!["coverage"]),
        Some(HashMap::from([
            (
                "RUSTFLAGS",
                "-Csymbol-mangling-version=v0 -g -Cinstrument-coverage -Zno-profiler-runtime",
            ),
            ("CARGO_PROFILE_BENCH_DEBUG", "true"),
            ("CARGO_PROFILE_BENCH_LTO", "no"),
            ("CARGO_PROFILE_BENCH_OPT_LEVEL", "0"),
        ])),
    );
    exe
}

fn export_profdata(profdata_path: &str, executable: &str) -> String {
    let mut llvm_export = Command::new("llvm-cov");
    llvm_export.args(["export", "--instr-profile", profdata_path, executable]);
    let vec = llvm_export.output().unwrap().stdout;
    let result = String::from_utf8(vec).unwrap();
    // println!("{:?}", result);
    result
}

fn merge_profdata(profraw_path: &str) -> String {
    let mut llvm_profdata = Command::new("llvm-profdata");
    let profdata_path = profraw_path.replace("profraw", "profdata");
    llvm_profdata.args(["merge", profraw_path, "-o", &profdata_path]);
    let _ = llvm_profdata.output().unwrap();
    profdata_path
}


fn run_with_coverage(executable: &str, benchmark_id: &str, dir: &PathBuf) -> String {
    let mut dir = dir.clone();

    let benchmark_filter = format!("^{benchmark_id}$");
    dir.push(benchmark_id);
    fs::create_dir_all(&dir).expect(&format!(
        "Could not create directory {}",
        dir.to_str().unwrap()
    ));

    let mut command = Command::new(executable);
    command.current_dir(&dir);
    let profraw_path = chrono::Local::now().format("%Y%m%d_%H%M%S.profraw").to_string();
    command.env("MINICOV_PROFILE_FILE", &profraw_path);
    command.arg(benchmark_filter);
    // println!("{:?}", command);
    let _ = command.output().unwrap();
    // println!("{}", output.status);
    dir.push(profraw_path);
    dir.to_str().unwrap().to_string()
}

fn collect_covdata(json_data: &str) -> LlvmCovData {
    // println!("{}", profdata_path);
    // let result = fs::read_to_string(profdata_path).unwrap();
    serde_json::from_str(&json_data).expect(&format!("Could not deserialize {}", json_data))
}




fn save_language_features(data: &Rc<HashMap<String, u64>>, path: String) {
    println!("Writing to {}", &path);
    let mut writer = csv::Writer::from_path(path).unwrap();
    for (k, v) in data.iter() {
        writer.serialize((k, v)).unwrap();
    }
}

pub fn gather_coverage() {
    for record in read_target_projects() {
        let project = Project::load(&record.name).expect("Could not load project");
        for benchmark_file in project.bench_files {
            let coverage_executable = compile_for_coverage(&benchmark_file);
            let coverage_executable = if coverage_executable.is_some() {
                coverage_executable.unwrap()
            } else {
                println!("Failed to compile for coverage {:?}", benchmark_file);
                continue;
            };

            let mut dir = env::current_dir().unwrap();
            dir.push("coverage");
            dir.push(&project.name);

            for id in &benchmark_file.benches {
                let profraw_path = run_with_coverage(&coverage_executable, id, &dir);
                let profdata_path = merge_profdata(&profraw_path);
                let json_string = export_profdata(&profdata_path, &coverage_executable);

                if json_string.is_empty() {
                    println!("Empty json for {}", id);
                    break;
                }

                let mut data: LlvmCovData = collect_covdata(&json_string);
                data.filter_non_zero();
                fs::write(profdata_path.replace("profdata", "json"), serde_json::to_string(&data).unwrap()).unwrap();

                let mut visit_counter = Rc::new(HashMap::<String, u64>::new());

                for entry in data.data {
                    for func in entry.functions.iter() {
                        visit_function_syn(func, &mut visit_counter);
                    }
                }

                let language_features_path = profraw_path.replace("profraw", "csv");
                save_language_features(&visit_counter, language_features_path);
            }
        }
    }
}

mod test {
    #[test]
    fn run_callgraph() {
        use crate::coverage::gather_coverage;
        gather_coverage();
    }
}



// fn run_with_callgrind(executable: &str, benchmark_id: &str, dir: &PathBuf) {
//     let mut dir = dir.clone();
//
//     let benchmark_filter = format!("^{benchmark_id}$");
//     dir.push(benchmark_id);
//     fs::create_dir_all(&dir).expect(&format!(
//         "Could not create directory {}",
//         dir.to_str().unwrap()
//     ));
//
//     // let callgrind_out_file = PathBuf::from(format!("{}/{}/callgrind.out.%p", dir.to_str().unwrap(), benchmark_id));
//     // let callgrind_out_argument = format!("--callgrind-out-file={}", callgrind_out_file.to_str().unwrap());
//
//     // valgrind --tool=callgrind --dump-line=yes --dump-instr=yes --toggle-collect="<criterion::bencher::Bencher>*" \
//     // target/x86_64-unknown-linux-gnu/release/deps/chrono-da7f19b02a5d3a3c bench_datetime_parse_from_rfc3339
//     let mut command = Command::new("valgrind");
//     command.current_dir(&dir);
//     command.args([
//         "--tool=callgrind",
//         "--dump-line=yes",
//         "--dump-instr=yes",
//         "--compress-strings=no",
//         "--compress-pos=no",
//         "--collect-jumps=yes",
//         // "--toggle-collect=<criterion::bencher::Bencher>::iter*",
//         "--zero-before=*Criterion::measurement*::start",
//         "--dump-before=*Criterion::measurement*::end",
//         // &callgrind_out_argument,
//         executable,
//         &benchmark_filter,
//     ]);
//
//     println!("{:?}", command);
//
//     let output = command.output().unwrap();
//     if !output.status.success() {
//         println!("Command failed: {:?}", command);
//         println!("Error: {:?}", String::from_utf8(output.stderr));
//     }
//     // debug!("{output:?}", );
// }

// fn convert_to_dot(id: &String, dir: &PathBuf) -> Option<String> {
//     let option = find_latest_callgrind(id, dir);
//     if let Some(callgrind_file) = option {
//         let input_path = callgrind_file.path().to_str().unwrap().to_string();
//         let output_path = format!("{input_path}.dot");
//
//         let mut gprof2dot = Command::new("gprof2dot");
//         gprof2dot.args([
//             "--format=callgrind",
//             "--node-thres=0",
//             "--edge-thres=0",
//             "-o",
//             &output_path,
//             &input_path,
//         ]);
//         let result = gprof2dot.output().unwrap();
//         if result.status.success() {
//             return Some(output_path);
//         }
//     }
//     None
// }

// fn get_id(id: &Id) -> String {
//     match id {
//         Id::Escaped(escaped) => escaped.replace("\"", ""),
//         Plain(plain) => plain.to_string(),
//         _ => String::new(),
//     }
// }

// fn convert_signature_to_fqn(x: &String) -> String {
//     x.clone()
// }

// fn extract_calls_from_dot(dotfile_path: &str) -> HashMap<String, u64> {
//     let dot = fs::read_to_string(dotfile_path).unwrap();
//     let dot = sanitize(dot);
//     let result = graphviz_rust::parse(&dot);
//     let graph = result.unwrap();
//     let stmts = match graph {
//         Graph::Graph { stmts, .. } => stmts,
//         Graph::DiGraph { stmts, .. } => stmts,
//     };
//
//     let mut map: HashMap<String, u64> = HashMap::new();
//
//     for stmt in stmts {
//         match stmt {
//             dot_structures::Stmt::Edge(edge) => {
//                 let option = edge
//                     .attributes
//                     .iter()
//                     .filter(|attr| attr.0.eq(&Plain("label".to_string())))
//                     .next();
//
//                 let label = get_id(&option.unwrap().1);
//                 // println!("{}", label);
//                 let count =
//                     u64::from_str(&label.split("\\n").last().unwrap().replace("×", "")).unwrap();
//                 // println!("{:?}", count);
//                 match edge.ty {
//                     EdgeTy::Pair(_, right) => match right {
//                         Vertex::N(n) => {
//                             let id = convert_signature_to_fqn(&get_id(&n.0));
//                             *map.entry(id).or_insert(0) += count;
//                         }
//                         Vertex::S(_) => {}
//                     },
//                     EdgeTy::Chain(_chain) => {
//                         println!("TODO");
//                     }
//                 }
//             }
//             _ => {}
//         }
//     }
//
//     // println!("{:?}", map);
//     map
// }

// fn sanitize(dot: String) -> String {
//     dot.replace("'", "")
// }

// fn remove_default_calls() {}

// fn extract_callgraph(_executable: &str, benchmark_id: &str, dir: &PathBuf) -> Option<String> {
//     let mut cmd = Command::new("callgrind_annotate");
//     cmd.args(["--threshold=100", "--tree=both"]);
//     let next = find_latest_callgrind(benchmark_id, dir);
//
//     if let Some(callgrind) = next {
//         cmd.arg(callgrind.path());
//         let out = String::from_utf8(cmd.output().unwrap().stdout).unwrap();
//         println!("{}", out);
//         println!(
//             "{}",
//             String::from_utf8(cmd.output().unwrap().stderr).unwrap()
//         );
//     }
//     None
// }

// fn find_latest_callgrind(benchmark_id: &str, dir: &PathBuf) -> Option<DirEntry> {
//     lazy_static! {
//         static ref RE: Regex = Regex::new(r"^callgrind.out.\d+$").unwrap();
//     }
//
//     let mut dir = dir.clone();
//     dir.push(benchmark_id);
//     println!("Reading dir {:?}", dir);
//     match dir.read_dir() {
//         Ok(entries) => entries
//             .map(|entry| entry.unwrap())
//             .filter(|entry| RE.is_match(entry.file_name().to_str().unwrap()))
//             .sorted_by_key(|entry| dir.metadata().unwrap().created().unwrap())
//             .last(),
//         Err(_) => None,
//     }
// }

// fn compile_for_callgrind(benchmark_file: &BenchFile) -> Option<String> {
//     let exe = compile_benchmark_file(
//         &benchmark_file,
//         Some("+nightly".to_string()),
//         Some(vec!["-Zbuild-std", "--target=x86_64-unknown-linux-gnu"]),
//         None,
//         Some(HashMap::from([
//             ("RUSTFLAGS", "-Csymbol-mangling-version=v0 -g"),
//             ("CARGO_PROFILE_BENCH_DEBUG", "true"),
//             ("CARGO_PROFILE_BENCH_LTO", "no"),
//             ("CARGO_PROFILE_BENCH_OPT_LEVEL", "0"),
//         ])),
//     );
//     exe
// }

// fn ts_node_descendant_for_point_range(
//     start: Node,
//     range_start: Point,
//     range_end: Point,
//     include_anonymous: bool,
// ) -> Node {
//     let mut node: Node = start;
//     let mut last_visible_node: Node = start;
//
//     let mut did_descend: bool = true;
//     while did_descend {
//         did_descend = false;
//
//         let x = &mut node.walk();
//         let mut iterator = node.children(x);
//         // NodeChildIterator iterator = ts_node_iterate_children(&node);
//         while let Some(child) = iterator.next() {
//             let node_end: Point = child.end_position();
//
//             // if range_end.eq(&node_end) && range_start.eq(&child.start_position()) {
//             //     return child
//             // }
//
//             // The end of this node must extend far enough forward to touch
//             // the end of the range and exceed the start of the range.
//             if point_lt(node_end, range_end) { continue; };
//             if point_lte(node_end, range_start) { continue; };
//
//             // The start of this node must extend far enough backward to
//             // touch the start of the range.
//             if point_lt(range_start, child.start_position()) { break; };
//
//             node = child;
//             // if (ts_node__is_relevant(node, include_anonymous)) {
//             last_visible_node = node;
//             // }
//             did_descend = true;
//             break;
//         }
//     }
//
//     return last_visible_node;
// }

// fn recurse_region_contains(region: &Region, node: &Node, f: &Vec<u8>, set: &mut HashSet<String>) {
//     if region.contains_range(&node.range()) {
//         set.insert(node.kind().to_str());
//         // println!("{}-{}, {:?}", "\t".repeat(depth), node.kind(), node.utf8_text(f.as_slice()));
//         // println!("{:?}", );
//         // node.kind()
//         // } else {
//     }
//     for child in node.children(&mut node.walk()) {
//         recurse_region_contains(region, &child, f, set)
//     }
//     // }
// }
