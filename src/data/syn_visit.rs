use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use itertools::Itertools;

use syn::{Abi, Arm, Block, ExprArray, ExprAssign, ExprAsync, ExprAwait, ExprBreak, ExprCall, ExprClosure, ExprContinue, ExprField, ExprForLoop, ExprIf, ExprLet, ExprLoop, ExprMatch, ExprMethodCall, ExprReference, ExprRepeat, ExprReturn, ExprStruct, ExprTry, ExprTryBlock, ExprTuple, ExprUnsafe, ExprWhile, Index, ItemFn, ItemMod, TypePtr};
use syn::spanned::Spanned;
use syn::visit::{Visit, visit_abi, visit_arm, visit_block, visit_expr_array, visit_expr_assign, visit_expr_async, visit_expr_await, visit_expr_break, visit_expr_call, visit_expr_closure, visit_expr_continue, visit_expr_field, visit_expr_for_loop, visit_expr_if, visit_expr_let, visit_expr_loop, visit_expr_match, visit_expr_method_call, visit_expr_reference, visit_expr_repeat, visit_expr_return, visit_expr_struct, visit_expr_try, visit_expr_try_block, visit_expr_tuple, visit_expr_unsafe, visit_expr_while, visit_index, visit_item_fn, visit_item_mod, visit_type_ptr};

use crate::data::llvmcovdata::{Function, Region};

struct Visitor<'rc, 'region> {
    counter: &'rc mut Rc<HashMap<String, u64>>,
    region: &'region Region,
    _count: u64,
    loop_depth: u64,
    modpath: Vec<String>,
}



impl<'rc, 'region> Visitor<'rc, 'region> {
    pub fn new(counter: &'rc mut Rc<HashMap<String, u64>>, region: &'region Region, path: &str) -> Self {
        Visitor { counter, region, _count: region.execution_count.clone() as u64, loop_depth: 0, modpath: parse_mod_from_str(path)}
    }

    fn count(&mut self, label: &str) {
        Rc::get_mut(self.counter).unwrap().entry("count_".to_owned() + label).and_modify(|v| {*v = v.saturating_add(1);}).or_default();
        Rc::get_mut(self.counter).unwrap().entry("once_".to_owned() + label).and_modify(|v| {*v = v.saturating_add(self._count);}).or_default();
    }

    pub(crate) fn enter_mod(&mut self, modname: String) {
        self.modpath.push(modname);
    }

    fn exit_mod(&mut self) {
        self.modpath.pop();
    }
}


#[test]
fn test_string_add() {

}

impl<'ast, 'rc, 'region, 's> Visit<'ast> for Visitor<'rc, 'region> {
    fn visit_abi(&mut self, i: &'ast Abi) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.extern_token.span) {
            self.count("abi");
        }

        visit_abi(self, i);
    }

    fn visit_arm(&mut self, i: &'ast Arm) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.pat.span()) {
            self.count("match_arm_pat");
        }
        if let Some((_, expr)) = &i.guard {
            if self.region.overlaps_span(&expr.span()) {
                self.count("match_arm_guard");
            }
        }
        visit_arm(self, i);
    }

    fn visit_block(&mut self, i: &'ast Block) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        visit_block(self, i);
    }

    fn visit_expr_array(&mut self, i: &'ast ExprArray) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("array");

        visit_expr_array(self, i);
    }

    fn visit_expr_assign(&mut self, i: &'ast ExprAssign) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("assign");
        visit_expr_assign(self, i);
    }

    fn visit_expr_async(&mut self, i: &'ast ExprAsync) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.async_token.span) {
            self.count("async");
        }

        if i.capture.is_some_and(|cap| self.region.overlaps_span(&cap.span)) {
            self.count("closure_capture");
        }
        visit_expr_async(self, i);
    }

    fn visit_expr_await(&mut self, i: &'ast ExprAwait) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.await_token.span) {
            self.count("await");
        }

        visit_expr_await(self, i);
    }

    fn visit_expr_break(&mut self, i: &'ast ExprBreak) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.break_token.span) {
            self.count("break");
        }
        visit_expr_break(self, i);
    }

    fn visit_expr_call(&mut self, i: &'ast ExprCall) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("call");

        visit_expr_call(self, i);
    }

    fn visit_expr_closure(&mut self, i: &'ast ExprClosure) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.or2_token.span) && self.region.overlaps_span(&i.or1_token.span) {
            self.count("closure");
        }

        if i.asyncness.is_some_and(|tok| self.region.overlaps_span(&tok.span)) {
            self.count("closure_async");
        }

        if i.constness.is_some_and(|tok| self.region.overlaps_span(&tok.span)) {
            self.count("closure_const");
        }

        if i.movability.is_some_and(|tok| self.region.overlaps_span(&tok.span)) {
            self.count("closure_static");
        }
        visit_expr_closure(self, i);
    }

    fn visit_expr_continue(&mut self, i: &'ast ExprContinue) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.continue_token.span) {
            self.count("break");
        }
        visit_expr_continue(self, i);
    }
    fn visit_expr_field(&mut self, i: &'ast ExprField) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("field");

        visit_expr_field(self, i);
    }

    fn visit_expr_for_loop(&mut self, i: &'ast ExprForLoop) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.for_token.span) {
            self.count("loop_for");
        }

        if self.loop_depth > 0 {
            self.count("nested_loop");
        }

        self.loop_depth += 1;
        visit_expr_for_loop(self, i);
        self.loop_depth -= 1;
    }

    fn visit_expr_if(&mut self, i: &'ast ExprIf) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.if_token.span) {
            self.count("if");
        }

        if i.else_branch.as_ref().is_some_and(|(tok, _)| self.region.overlaps_span(&tok.span)) {
            self.count("else");
        }

        visit_expr_if(self, i);
    }

    fn visit_expr_let(&mut self, i: &'ast ExprLet) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.let_token.span()) {
            self.count("let_expr");
        }

        visit_expr_let(self, i);
    }

    fn visit_expr_loop(&mut self, i: &'ast ExprLoop) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.loop_token.span) {
            self.count("loop_inf")
        }
        if self.loop_depth > 0 {
            *Rc::get_mut(self.counter).unwrap().entry("nested_loop".to_string()).or_insert(0) += 1
        }

        self.loop_depth += 1;
        visit_expr_loop(self, i);
        self.loop_depth -= 1;
    }


    fn visit_expr_match(&mut self, i: &'ast ExprMatch) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.match_token.span) {
            self.count("match");
        }
        visit_expr_match(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        self.count("method_call");

        visit_expr_method_call(self, i);
    }

    fn visit_expr_reference(&mut self, i: &'ast ExprReference) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("reference");

        if i.mutability.is_some() {
            self.count("reference_mutable");
        }

        visit_expr_reference(self, i);
    }

    fn visit_expr_repeat(&mut self, i: &'ast ExprRepeat) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("array");

        visit_expr_repeat(self, i);
    }

    fn visit_expr_return(&mut self, i: &'ast ExprReturn) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.return_token.span) {
            self.count("return");
        }
        visit_expr_return(self, i);
    }

    fn visit_expr_struct(&mut self, i: &'ast ExprStruct) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("struct");

        visit_expr_struct(self, i);
    }

    fn visit_expr_try(&mut self, i: &'ast ExprTry) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.question_token.span) {
            self.count("try");
        }

        visit_expr_try(self, i);
    }

    fn visit_expr_try_block(&mut self, i: &'ast ExprTryBlock) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        if self.region.overlaps_span(&i.try_token.span) {
            self.count("try_block");
        }

        visit_expr_try_block(self, i);
    }

    fn visit_expr_tuple(&mut self, i: &'ast ExprTuple) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("tuple");

        visit_expr_tuple(self, i);
    }

    fn visit_expr_unsafe(&mut self, i: &'ast ExprUnsafe) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("unsafe");

        visit_expr_unsafe(self, i);
    }

    fn visit_expr_while(&mut self, i: &'ast ExprWhile) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.while_token.span) {
            self.count("loop_while")
        }
        if self.loop_depth > 0 {
            *Rc::get_mut(self.counter).unwrap().entry("nested_loop".to_string()).or_insert(0) += 1
        }

        self.loop_depth += 1;
        visit_expr_while(self, i);
        self.loop_depth -= 1;
    }

    fn visit_index(&mut self, i: &'ast Index) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.count("index");

        visit_index(self, i);
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }


        if self.region.contains_span(&i.sig.ident.span()) {
            self.count("item_fn");
            self.count(self.modpath.join("::").as_str());
        }


        visit_item_fn(self, i);
    }
    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }

        self.enter_mod(i.ident.to_string());
        visit_item_mod(self, i);
        self.exit_mod();
    }

    fn visit_type_ptr(&mut self, i: &'ast TypePtr) {
        if !self.region.overlaps_span(&i.span()) {
            return;
        }
        if self.region.overlaps_span(&i.star_token.span) {
            self.count("ptr_star")
        }
        visit_type_ptr(self, i);
    }
}

pub(crate) fn visit_function_syn(coverage: &Function, map: &mut Rc<HashMap<String, u64>>) {
    if coverage.filenames.len() > 1 {
        panic!("{:?} has more than one file", coverage.filenames);
    }
    let path = coverage.filenames.first().unwrap();
    println!("{}", &path);
    let result = syn::parse_file(&fs::read_to_string(path).expect(&format!("Could not open file {path}")));

    if result.is_err() {
        // Failed to parse
        return;
    }

    let parser = result.unwrap();
    for region in coverage.regions.iter() {
        let mut visitor = Visitor::new(map, &region, coverage.filenames.first().unwrap());

        for node in parser.items.iter() {
            if region.overlaps_span(&node.span()) {
                visitor.visit_item(&node);
            }
        };
    }
}

fn parse_mod_from_str(path: &str) -> Vec<String> {
    if path.is_empty() {
        return vec![]
    }

    let paths = path.split("/src/").collect_vec();
    let mut split = paths.iter().rev();

    let modules = split.next().unwrap();
    let remaining_path = split.next();

    if remaining_path.is_none() {
        return vec![]
    }

    let cleaned_path = modules.replace("mod.rs", "").replace(".rs", "");
    let path = cleaned_path.split("/").map(String::from).filter(|s| !s.is_empty());
    let krate = remaining_path.unwrap().split("/").last().unwrap();
    let unversioned = krate.rsplit_once("-").or_else(|| Some((krate, ""))).unwrap().0.to_string();

    [unversioned].iter().map(String::from).chain(path).collect()
}

mod test {
    #![allow(unused_imports)]

    use std::rc::Rc;


    use syn::visit::Visit;

    use crate::data::llvmcovdata::Region;

    use crate::data::syn_visit::{parse_mod_from_str, Visitor};
    trait Test {
        fn new_test(ls: i64, cs: i64, le: i64, ce: i64) -> Self;
    }

    impl Test for Region {
        fn new_test(ls: i64, cs: i64, le: i64, ce: i64) -> Self {
            Region {
                line_start: ls,
                column_start: cs,
                line_end: le,
                column_end: ce,
                execution_count: 1,
                file_id: 0,
                expanded_file_id: 0,
                kind: 0,
            }
        }
    }

    #[test]
    fn test_parse_mod_from_str() {
        let vec = parse_mod_from_str("/home/rens/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/adapters/map.rs");
        assert_eq!(vec, vec!["core","iter","adapters","map"]);

        let vec = parse_mod_from_str("/home/rens/.cargo/registry/src/index.crates.io-6f17d22bba15001f/hashbrown-0.13.1/src/map.rs");
        assert_eq!(vec, vec!["hashbrown", "map"]);

        let vec = parse_mod_from_str("/home/rens/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/hash/sip.rs");
        assert_eq!(vec, vec!["core", "hash", "sip"]);

        let vec = parse_mod_from_str("/home/rens/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/sys/unix/path.rs");
        assert_eq!(vec, vec!["std","sys","unix","path"]);

        let vec = parse_mod_from_str("/home/rens/.cargo/registry/src/index.crates.io-6f17d22bba15001f/regex-1.7.1/src/literal/mod.rs");
        assert_eq!(vec, vec!["regex","literal"]);
    }

    #[test]
    fn test_visit_arm() {
        let source =
            "match \"hello\" {\
    \"no\" => false,
    \"hello\" => true,
    _ => false
}";
        let stmt = syn::parse_str(source).unwrap();

        let mut map = Rc::new(Default::default());
        let region = Region::new_test(3, 5, 3, 23);
        let mut visitor = Visitor::new(&mut map, &region, "");

        visitor.visit_stmt(&stmt);

        println!("{:?}", map);
        // assert!(map.len() > 0);
        assert_eq!(*map.get("match_arm_pat").unwrap(), 1);
    }

    #[test]
    fn test_if() {
        let source =
            "    fn num_days_from_ce(&self) -> i32 {
        // See test_num_days_from_ce_against_alternative_impl below for a more straightforward
        // implementation.

        // we know this wouldn't overflow since year is limited to 1/2^13 of i32's full range.
        let mut year = self.year() - 1;
        let mut ndays = 0;
        if year < 0 {
            let excess = 1 + (-year) / 400;
            year += excess * 400;
            ndays -= excess * 146_097;
        }
        let div_100 = year / 100;
        ndays += ((year * 1461) >> 2) - div_100 + (div_100 >> 2);
        ndays + self.ordinal() as i32
    }";

        let stmt = syn::parse_str(source).unwrap();
        let mut map = Rc::new(Default::default());
        let region4 = Region { line_start: 103 - 102, column_start: 5, line_end: 110 - 102, column_end: 20, execution_count: 1, file_id: 0, expanded_file_id: 0, kind: 0 };
        let region2 = Region { line_start: 114 - 102, column_start: 10, line_end: 114 - 102, column_end: 11, execution_count: 1, file_id: 0, expanded_file_id: 0, kind: 0 };
        let region1 = Region { line_start: 115 - 102, column_start: 13, line_end: 118 - 102, column_end: 6, execution_count: 1, file_id: 0, expanded_file_id: 0, kind: 0 };

        for region in [region1, region2, region4] {
            let mut visitor = Visitor::new(&mut map, &region, "");
            visitor.visit_stmt(&stmt);
        }
        assert_eq!(*map.get("if").unwrap(), 1);
    }
}