use proc_macro2::Span;
// Generated using quicktype
// Modified to fit use-case and specification:
// https://github.com/llvm/llvm-project/blob/main/llvm/tools/llvm-cov/CoverageExporterJson.cpp
use serde::{Deserialize, Serialize};
use serde_tuple::*;

// use tree_sitter::{Point, Range};

pub trait Filter {
    fn filter_non_zero(&mut self) -> ();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlvmCovData {
    pub(crate) data: Vec<LlvmCovDataEntry>,
    #[serde(rename = "type")]
    pub(crate) _type: String,
    pub(crate) version: String,
}

impl Filter for LlvmCovData {
    fn filter_non_zero(&mut self) -> () {
        self.data
            .iter_mut()
            .for_each(|entry| entry.filter_non_zero())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlvmCovDataEntry {
    pub(crate) files: Vec<File>,
    pub(crate) functions: Vec<Function>,
    pub(crate) totals: CoverageSummary,
}

impl Filter for LlvmCovDataEntry {
    fn filter_non_zero(&mut self) -> () {
        self.files
            .iter_mut()
            .for_each(|file| file.filter_non_zero());
        self.functions.retain(|func| func.count > 0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub(crate) branches: Vec<Branch>,
    pub(crate) expansions: Vec<Expansion>,
    pub(crate) filename: String,
    pub(crate) segments: Vec<Segment>,
    pub(crate) summary: CoverageSummary,
}

impl Filter for File {
    fn filter_non_zero(&mut self) -> () {}
}
/*
json::Object renderExpansion(const coverage::CoverageMapping &Coverage,
                             const coverage::ExpansionRecord &Expansion) {
  std::vector<llvm::coverage::ExpansionRecord> Expansions = {Expansion};
  return json::Object(
      {{"filenames", json::Array(Expansion.Function.Filenames)},
       // Mark the beginning and end of this expansion in the source file.
       {"source_region", renderRegion(Expansion.Region)},
       // Enumerate the coverage information for the expansion.
       {"target_regions", renderRegions(Expansion.Function.CountedRegions)},
       // Enumerate the branch coverage information for the expansion.
       {"branches",
        renderBranchRegions(collectNestedBranches(Coverage, Expansions))}});
}
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expansion {
    filenames: Vec<String>,
    source_region: Region,
    target_regions: Vec<Region>,
    branches: Vec<Branch>,
}

/*
json::Array renderBranch(const coverage::CountedRegion &Region) {
  return json::Array(
      {Region.LineStart, Region.ColumnStart, Region.LineEnd, Region.ColumnEnd,
       clamp_uint64_to_int64(Region.ExecutionCount),
       clamp_uint64_to_int64(Region.FalseExecutionCount), Region.FileID,
       Region.ExpandedFileID, int64_t(Region.Kind)});
}
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    line_start: i64,
    column_start: i64,
    line_end: i64,
    column_end: i64,
    execution_count: i64,
    false_execution_count: i64,
    file_id: i64,
    expanded_file_id: i64,
    kind: i64,
}
/*
json::Array renderSegment(const coverage::CoverageSegment &Segment) {
  return json::Array({Segment.Line, Segment.Col, int64_t(Segment.Count),
                      Segment.HasCount, Segment.IsRegionEntry});
} */
#[derive(Debug, Clone, Serialize_tuple, Deserialize_tuple)]
pub struct Segment {
    pub(crate) line: i64,
    pub(crate) col: i64,
    pub(crate) count: i64,
    pub(crate) has_count: bool,
    pub(crate) is_region_entry: bool,
    pub(crate) is_gap_region: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub(crate) branches: SummaryStats,
    pub(crate) functions: SummaryStats,
    pub(crate) instantiations: SummaryStats,
    pub(crate) lines: SummaryStats,
    pub(crate) regions: SummaryStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryStats {
    pub(crate) count: i64,
    pub(crate) covered: i64,
    #[serde(rename = "notcovered")]
    pub(crate) not_covered: Option<i64>,
    pub(crate) percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub(crate) branches: Vec<Branch>,
    pub(crate) count: i64,
    pub(crate) filenames: Vec<String>,
    pub(crate) name: String,
    pub(crate) regions: Vec<Region>,
}

/* json::Array renderRegion(const coverage::CountedRegion &Region) {
  return json::Array({Region.LineStart, Region.ColumnStart, Region.LineEnd,
                      Region.ColumnEnd, clamp_uint64_to_int64(Region.ExecutionCount),
                      Region.FileID, Region.ExpandedFileID,
                      int64_t(Region.Kind)});
}
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub(crate) line_start: i64,
    pub(crate) column_start: i64,
    pub(crate) line_end: i64,
    pub(crate) column_end: i64,
    pub(crate) execution_count: i64,
    pub(crate) file_id: i64,
    pub(crate) expanded_file_id: i64,
    pub(crate) kind: i64,
}

impl Region {

    pub fn start(&self) -> proc_macro2::LineColumn {
        proc_macro2::LineColumn { line: self.line_start.clone() as usize, column: self.column_start.clone() as usize }
    }

    pub fn end(&self) -> proc_macro2::LineColumn {
        proc_macro2::LineColumn { line: self.line_end.clone() as usize, column: self.column_end.clone() as usize }
    }

    #[inline]
    pub fn contains_span(&self, span: &Span) -> bool {
        Self::lte(&self.start(), &span.start()) && Self::lte(&span.end(), &self.end())
    }

    #[allow(unused)]
    pub fn contained_in_span(&self, span: &Span) -> bool {
        Self::lte(&span.start(), &self.start()) && Self::lte(&self.end(), &span.end())
    }

    #[inline]
    pub fn overlaps_span(&self, span: &Span) -> bool {
        Self::lte(&span.start(), &self.end()) && Self::lte(&self.start(), &span.end())
    }

    #[inline]
    fn lte(lhs: &LineColumn, rhs: &LineColumn) -> bool {
        lhs.line < rhs.line || (lhs.line == rhs.line && lhs.column <= rhs.column)
    }

}

use proc_macro2::LineColumn;
// #[inline]
// pub(crate) fn point_lte(a: Point, b: Point) -> bool {
//     return (a.row < b.row) || (a.row == b.row && a.column <= b.column);
// }
//
// #[inline]
// pub(crate) fn point_lt(a: Point, b: Point) -> bool {
//     return (a.row < b.row) || (a.row == b.row && a.column < b.column);
// }
//
// #[inline]
// pub(crate) fn point_gt(a: Point, b: Point) -> bool {
//     return (a.row > b.row) || (a.row == b.row && a.column > b.column);
// }
//
// #[inline]
// pub(crate) fn point_gte(a: Point, b: Point) -> bool {
//     return (a.row > b.row) || (a.row == b.row && a.column >= b.column);
// }
