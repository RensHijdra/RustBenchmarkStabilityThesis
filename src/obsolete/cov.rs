use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use crate::collect::compile_benchmark_file;
use crate::data::llvmcovdata::{Filter, Function, LlvmCovData};
use crate::data::project::{read_target_projects, BenchFile, Project};

fn generate_coverage_data() -> Vec<BenchFile> {
    let coverage = build_with_coverage();
    for (exe, benchfile) in &coverage {
        let mut dir = env::current_dir().unwrap();
        dir.push("coverage");
        dir.push(&benchfile.project);

        fs::create_dir_all(&dir).unwrap();

        for bench in &benchfile.benches {
            run_with_coverage(&exe, &bench, &dir);
            merge_profdata(&dir, &bench);
            export_profdata(&exe, &dir, &bench);
            // break;
        }
        break;
    }
    coverage
        .iter()
        .map(|(_e, b)| b.clone())
        .collect::<Vec<BenchFile>>()
}

fn build_with_coverage() -> Vec<(String, BenchFile)> {
    let mut wrapper_path = env::current_dir().unwrap();
    wrapper_path.push("rustc_wrapper");

    let mut vec: Vec<(String, BenchFile)> = Default::default();
    for record in read_target_projects() {
        let project = Project::load(&record.name).expect("Could not load project {");
        for benchmark_file in project.bench_files {
            let exe = compile_benchmark_file(
                &benchmark_file,
                Some(String::from("+nightly")),
                // None,
                // Some(vec!["-Z", "build-std", "--target", env!("TARGET")]),
                None,
                None,
                Some(HashMap::from([
                    /*("RUSTC_WRAPPER", wrapper_path.to_str().unwrap()),*/
                    ("RUSTFLAGS", "-C instrument-coverage"),
                ])),
            );
            if exe.is_none() {
                continue;
            }
            vec.push((exe.unwrap(), benchmark_file));
        }
        break;
    }
    vec
}

fn run_with_coverage(executable: &str, name: &str, proj: &PathBuf) {
    println!("Running {:?} ", proj.to_str());
    let mut command = Command::new(executable);
    command
        .current_dir(proj)
        .env("LLVM_PROFILE_FILE", format!("{}.profraw", name))
        .arg(format!("^{name}$")); // Filter to run only this benchmark
    println!("{:?}", command);
    let result = command.output().unwrap();
    let vec = result.stdout;
    let string = String::from_utf8(vec).unwrap();
    println!("{:?}", string);
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
    // println!("{}", string);
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

fn cov_of_file(path: &str) {
    let rust_code = std::fs::read_to_string(path).expect("Failed to read file");
    // let syn_file = syn::parse_file(&code).expect("Failed to parse code");

    if let Ok(calls) = extract_method_calls(&rust_code) {
        for call in calls {
            println!("{}", call);
        }
    }
}

fn get_statistics_from_function(_function: Function) {
    println!("{}", rustc_demangle::demangle(&_function.name).to_string());
    for _file in _function.filenames {
        // cov_of_file(&file)
    }
}

fn load_coverage(dir: &PathBuf, id: &str) -> LlvmCovData {
    let mut input = dir.clone();
    input.push(id);
    // println!("{:?}", input);
    let path = format!("{}.json", input.to_str().unwrap());
    let string = fs::read_to_string(&path).expect(&format!("File {} not found!", &path));
    serde_json::from_str::<LlvmCovData>(&string).expect("Could not read file as valid json")
}

#[test]
fn test_coverage() {
    let covered_benchmarks = generate_coverage_data();
    for bench in covered_benchmarks {
        parse_source_with_coverage(bench);
    }
}

use syn::{
    visit::{self, Visit},
    Block, Expr, Item, ItemFn, ItemTrait, Result, Stmt, TraitItemFn,
};

// Custom visitor struct to collect fully qualified method and function call names
struct MethodCallVisitor {
    calls: Vec<String>,
}

impl<'ast> Visit<'ast> for MethodCallVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::Call(expr_call) = expr {
            if let Expr::Path(path) = &*expr_call.func {
                if let Some(segment) = path.path.segments.last() {
                    self.calls.push(segment.ident.to_string());
                }
            }
        }
        visit::visit_expr(self, expr);
    }

    fn visit_block(&mut self, i: &'ast Block) {
        for stmt in i.stmts.iter() {
            match stmt {
                Stmt::Local(local) => {
                    // todo inc let
                    if let Some(localinit) = &local.init {
                        visit::visit_expr(self, &localinit.expr)
                    }
                }
                Stmt::Item(item) => match item {
                    Item::Const(_) => {}
                    Item::Enum(_) => {}
                    Item::ExternCrate(_) => {}
                    Item::Fn(_) => {}
                    Item::ForeignMod(_) => {}
                    Item::Impl(_) => {}
                    Item::Macro(_) => {}
                    Item::Mod(_) => {}
                    Item::Static(_) => {}
                    Item::Struct(_) => {}
                    Item::Trait(_) => {}
                    Item::TraitAlias(_) => {}
                    Item::Type(_) => {}
                    Item::Union(_) => {}
                    Item::Use(_) => {}
                    Item::Verbatim(_) => {}
                    _ => {}
                },
                Stmt::Expr(expr, _semi) => {
                    visit::visit_expr(self, expr);
                }
                Stmt::Macro(_makro) => {}
            }
        }
    }

    fn visit_item_fn(&mut self, item_fn: &'ast ItemFn) {
        visit::visit_item_fn(self, item_fn);
        self.calls.push(item_fn.sig.ident.to_string());
    }

    fn visit_item_trait(&mut self, item_trait: &'ast ItemTrait) {
        for item in &item_trait.items {
            if let syn::TraitItem::Fn(TraitItemFn { sig, .. }) = item {
                self.calls.push(sig.ident.to_string());
            }
        }
        visit::visit_item_trait(self, item_trait);
    }
}

// Main function to extract method and function call names from a Rust file
fn extract_method_calls(input: &str) -> Result<Vec<String>> {
    let syntax_tree = syn::parse_file(input)?;

    let mut visitor = MethodCallVisitor { calls: Vec::new() };
    visitor.visit_file(&syntax_tree);

    Ok(visitor.calls)
}

#[test]
fn test_parse() {
    let rust_code = r#"
        mod my_module {
            pub fn my_function() {}
        }

        fn main() {
            my_module::my_function();
            println!("Hello, world!");
        }
    "#;

    if let Ok(calls) = extract_method_calls(rust_code) {
        for call in calls {
            println!("{}", call);
        }
    }
}
