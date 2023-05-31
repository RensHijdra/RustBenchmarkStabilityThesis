#![allow(unused)]
// #[derive(Debug, Serialize)]
// pub struct LocalAstResponse {
//     /// The id associated to a request for an `AST`
//     id: String,
//     /// The root node of an `AST`
//     ///
//     /// If `None`, an error has occurred
//     root: Option<AstNode>,
// }
//
// fn enter_func(space: FuncSpace, depth: usize) {
//     let tabs = "\t".repeat(depth);
//     println!("{}Space: {:?}, {:?}", tabs, space.name, space.kind);
//     println!("{}- {:?}", tabs, space.metrics);
//
//     for subspace in space.spaces {
//         enter_func(subspace, depth + 1);
//     }
// }
//
// fn enter_ops(ops: Ops, depth: usize) {
//     let tabs = "\t".repeat(depth);
//     println!("{}Space: {:?}, {:?}", tabs, ops.name, ops.kind);
//     // println!("{}{:?}", tabs, ops.metrics);
//     println!("{}- {:?}", tabs, ops.operands);
//     println!("{}- {:?}", tabs, ops.operators);
//     for subspace in ops.spaces {
//         enter_ops(subspace, depth + 1);
//     }
//     // println!();
// }

extern crate core;

use ra_ap_hir::db::HirDatabase;
use ra_ap_ide::{Analysis, Cancellable, FilePosition, NavigationTarget, TextSize};
use ra_ap_paths::AbsPath;
use ra_ap_project_model::ProjectManifest;
use ra_ap_rust_analyzer::cli::load_cargo::LoadCargoConfig;
use ra_ap_syntax::{SourceFile, SyntaxKind, SyntaxNode};
use ra_ap_vfs::VfsPath;
use reqwest::get;
use std::fs;
use std::ops::Index;
use std::path::Path;
use std::process::Command;

fn main() {
    // let path = Path::new("../chrono/benches/chrono.rs");
    // let file = read_file(&path).unwrap();
    // let parser = RustParser::new(file.clone(), path, None);
    // let root = parser.get_root();
    //
    // // let spaces = get_function_spaces(&LANG::Rust, file, path, None).unwrap();
    // // // println!("{:?}", option);
    // // // enter_func(spaces, 0);
    // //
    // // let operands = operands_and_operators(&parser, path).unwrap();
    // // // println!("{:?}", option);
    // // enter_ops(operands, 0);
    // // return;
    //
    // // let dump = dump_node(&file, &root, -1, None, None).unwrap();
    // let cfg = AstCfg {
    //     id: String::new(),
    //     comment: false,
    //     span: false,
    // };
    // let response = action::<AstCallback>(
    //     &LANG::Rust,
    //     file.clone(),
    //     path,
    //     None,
    //     cfg,
    // );
    // // println!("{:?}", response);
    // let mutted: LocalAstResponse = unsafe { mem::transmute(response) };
    //
    // // println!("{:?}", mutted.root);
    // let root = mutted.root.unwrap();
    //
    // ast_call_enter(root, 0);
    // // println!("{:?}: {:?}", root.value, root.r#type);
    // // for child in root.children {
    // //     match child.r#type {
    // //         "function_item" => {
    // //             println!("{:?}", child.children);
    // //         }
    // //         _ => {}
    // //     }
    // //     println!("{:?}, {:?}", child.value, child.r#type);
    // //
    // // }
    //
    // // ra_ap_hir::db::

    // mutted.root.unwrap().children;

    // project_model::ProjectWorkspace::load();
    // ra_ap_rust_analyzer::config::
    // use rp_ap_project_model;
    // rp_ap_pro

    fn take_str(h: String) {
        println!("{}", h);
    }
    // ra_ap_rust_analyzer::cli::load_cargo::load_workspace("../projects/chrono", &Default::default(), &LoadCargoConfig {load_out_dirs_from_check: false, with_proc_macro: false, prefill_caches: false});
    let buf = fs::canonicalize(Path::new("./projects/chrono")).unwrap();
    let abs_path = AbsPath::assert(buf.as_path());
    let manifest = ProjectManifest::discover_single(&abs_path).unwrap();
    println!("{:?}", manifest);
    let workspace =
        ra_ap_project_model::ProjectWorkspace::load(manifest, &Default::default(), &take_str)
            .unwrap();
    let load_cargo_config = LoadCargoConfig {
        load_out_dirs_from_check: false,
        with_proc_macro: false,
        prefill_caches: false,
    };

    let (analysis_host, vfs, _proc) = ra_ap_rust_analyzer::cli::load_cargo::load_workspace(
        workspace,
        &Default::default(),
        &load_cargo_config,
    )
    .unwrap();

    let path = VfsPath::new_real_path(buf.join("benches/chrono.rs").to_str().unwrap().to_string());
    let id = vfs.file_id(&path).unwrap();

    let analysis = analysis_host.analysis();
    // println!("{:?}", analysis.syntax_tree(id, None).unwrap());
    // let structure = analysis.file_structure(id).unwrap();

    // println!("{:?}", structure);

    // let result = analysis.parse(id).unwrap();
    // let node = SourceFile::parse(&result.to_string()).syntax_node();
    // println!("{}", result);
    // println!("{:?}", node.kind());
    // syntax_node_decender(node, 0);
    // println!("{}", node);

    let bench_function = FilePosition {
        file_id: id,
        offset: TextSize::from(297),
    };
    let hierarchy = analysis.call_hierarchy(bench_function).unwrap();
    let calls = analysis.outgoing_calls(bench_function).unwrap();
    let incalls = analysis.incoming_calls(bench_function).unwrap();
    let impls = analysis.goto_implementation(bench_function).unwrap();
    let decs = analysis.goto_declaration(bench_function).unwrap();
    let defs = analysis.goto_definition(bench_function).unwrap();
    let typdef = analysis.goto_type_definition(bench_function).unwrap();

    println!("hierarchy: {:?}", hierarchy);
    println!("outcalls: {:?}", calls);
    println!("incalls: {:?}", incalls);
    println!("impls {:?}", impls);
    println!("declarations {:?}", decs);
    println!("definitions: {:?}", defs);
    println!("typedefs: {:?}", typdef);
    let db = analysis_host.raw_database();

    println!("hir: {:?}", analysis.view_hir(bench_function));

    // analysis_host.analysis().crate_edition().unwrap().
    // for navtarget in typdef.unwrap().info {
    //     navtarget.
    // }
    // let mut target = hierarchy.unwrap().info[0].clone();
    // while let Some(parent) = get_parent(&analysis, &target) {
    //     println!("{:?}", parent);
    //     target = parent;
    // }

    // for node in structure {
    //     node.navigation_range
    // }
    // analysis.call_hierarchy(FilePosition);

    // println!("{:?}", analysis_host.raw_database());
}

fn get_parent(analysis: &Analysis, navtarget: &NavigationTarget) -> String {
    let vec = analysis
        .parent_module(FilePosition {
            file_id: navtarget.file_id,
            offset: navtarget.full_range.start(),
        })
        .unwrap();
    let name = &vec[0].name;
    // if name == "" {
    //     let crate_id = analysis.crates_for(navtarget.file_id).unwrap()[0];
    //     crate_id as Idx<CrateData>
    //
    // }
    String::new()
}

fn syntax_node_decender(node: SyntaxNode, depth: usize) {
    let tabs = "\t".repeat(depth);
    println!("{}{:?}", tabs, node.kind());
    for child in node.children() {
        match child.kind() {
            SyntaxKind::FN => {
                syntax_node_decender(child, depth + 1);
            }
            SyntaxKind::BLOCK_EXPR => {
                syntax_node_decender(child, depth + 1);
            }
            SyntaxKind::STMT_LIST => {
                println!("{}{:?}", tabs, child.kind());
                // syntax_node_decender(child, depth + 1);
                for stmt in child.children() {
                    // println!("{:?}", stmt);
                    syntax_node_decender(stmt, depth + 1);
                    //
                }
            }
            SyntaxKind::METHOD_CALL_EXPR => {
                println!(
                    "{}METHOD_CALL {}",
                    tabs,
                    child.to_string().replace("\n", " ")
                );
                // syntax_node_decender(child, depth + 1);
                for sub in child.children() {
                    syntax_node_decender(sub, depth + 1)
                }
            }
            SyntaxKind::CLOSURE_EXPR => {
                println!("{}CLOSURE: {}", tabs, child.to_string().replace("\n", " "));
                for sub in child.children() {
                    syntax_node_decender(sub, depth + 1);
                }
            }
            // SyntaxKind::
            SyntaxKind::NAME => {
                println!("{}-NAME: {}", tabs, child);
            }
            _ => {
                println!("{}{:?}", tabs, child.kind());
            }
        }
    }
}
//
// fn ast_call_enter(root: AstNode, depth: usize) {
//     // println!("{:?}", mutted.root);
//     let tabs = "\t".repeat(depth);
//     println!("{}{:?}: {:?}",tabs,  root.value, root.r#type);
//     for child in root.children {
//         match child.r#type {
//             "function_item" => {
//                 ast_call_enter(child, depth + 1);
//                 // println!("{:?}", child.children);
//             }
//             "block" => {
//                 ast_call_enter(child, depth + 1);
//             }
//             "call_expression" => {
//                 ast_call_enter(child, depth + 1);
//             }
//             "field_expression" => {
//                 println!("{}()", get_full_identifier(&child));
//                 ast_call_enter(child, depth + 1);
//             }
//             "scoped_identifier" => {
//                 println!("{:?}", child);
//                 // ast_call_enter(child, depth + 1);
//                 // println!("{} ==> {} {}", tabs, root.r#type, root.value);
//             }
//             _ => {
//                 println!("\t{}- {}", tabs, child.r#type);
//             }
//         }
//
//     }
// }
//
// fn get_full_identifier(field_expression: &AstNode) -> String {
//     field_expression.children.iter().map(|ast| &ast.value).join("")
// }
