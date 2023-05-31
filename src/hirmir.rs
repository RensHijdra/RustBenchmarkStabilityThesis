// #![feature(rustc_private)]
// extern crate rustc_error_codes;
// extern crate rustc_errors;
// extern crate rustc_hash;
// extern crate rustc_hir;
// extern crate rustc_interface;
// extern crate rustc_span;
// extern crate rustc_driver;
// use std::path::PathBuf;

// use rustc_driver::Compilation;

// struct CompilerCallbacks {}

// impl rustc_driver::Callbacks for CompilerCallbacks {
//     fn after_analysis<'tcx>(
//         &mut self,
//         compiler: &Compiler,
//         queries: &'tcx Queries<'tcx>,
//     ) -> Compilation { 
//         compiler.session().abort_if_errors();
        

//         // rudra::log::setup_logging(self.config.verbosity).expect("Rudra failed to initialize");

//         // debug!(
//         //     "Input file name: {}",
//         //     compiler.input().source_name().prefer_local()
//         // );
//         // debug!("Crate name: {}", queries.crate_name().unwrap().peek_mut());

//         // progress_info!("Rudra started");
//         queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
//             analyze(tcx, self.config);
//         });
//         // progress_info!("Rudra finished");

//         compiler.session().abort_if_errors();
//         Compilation::Stop
//     }
// }

// fn main() {
//     load_project(PathBuf::from("/home/rens/thesis/projects/chrono"))
// }

// fn load_project(path: PathBuf) {
//     let default_args = &["-Zalways-encode-mir", "-Zmir-opt-level=0" ].map(String::from);
//     // Invoke compiler, and handle return code.
//     let exit_code = rustc_driver::catch_with_exit_code(move || {
//         rustc_driver::RunCompiler::new(default_args, callbacks).run()
//     });

// }

fn main() {
    
}