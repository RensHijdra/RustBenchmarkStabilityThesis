// Generated using quicktype
// Modified to fit use-case and specification:
// https://github.com/llvm/llvm-project/blob/main/llvm/tools/llvm-cov/CoverageExporterJson.cpp
use rustc_demangle::demangle;
use serde::{Deserialize, Serialize};
use serde_tuple::*;

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
    branches: Vec<Branch>
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
    kind: i64
}

#[derive(Debug, Clone, Serialize_tuple, Deserialize_tuple)]
pub struct Segment {
    pub(crate) line: i64,
    pub(crate) col: i64,
    pub(crate) count: i64,
    pub(crate) has_count: bool,
    pub(crate) is_region_entry: bool,
    pub(crate) is_gap_region: bool,
}

/*
json::Array renderSegment(const coverage::CoverageSegment &Segment) {
  return json::Array({Segment.Line, Segment.Col, int64_t(Segment.Count),
                      Segment.HasCount, Segment.IsRegionEntry});
}

 */

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

impl Function {
    fn demangle(&mut self) {
        self.name = self.get_demangled()
    }

    pub(crate) fn get_demangled(&self) -> String {
        demangle(&self.name).to_string()
    }
}

/* json::Array renderRegion(const coverage::CountedRegion &Region) {
  return json::Array({Region.LineStart, Region.ColumnStart, Region.LineEnd,
                      Region.ColumnEnd, clamp_uint64_to_int64(Region.ExecutionCount),
                      Region.FileID, Region.ExpandedFileID,
                      int64_t(Region.Kind)});
}
 */
pub struct Region {
    line_start: i64,
    column_start: i64,
    line_end: i64,
    column_end: i64,
    execution_count: i64,
    file_id: i64,
    expanded_file_id: i64,
    kind: i64
}
