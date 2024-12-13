use crate::fputil::{get_uri_resource, glob_abs};
use crate::statement::url_normalize;
use crate::timer::Timer;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::{debug, info, instrument, warn};

#[derive(Debug, Deserialize, Clone)]
pub struct ScriptCoverageRaw {
    pub url: String,
    pub source: Option<String>,
    pub functions: Vec<FunctionCoverage>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct ScriptCoverage {
    pub url: String,
    pub source: String,
    pub functions: Vec<FunctionCoverage>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct FunctionCoverage {
    #[serde(rename = "functionName")]
    pub function_name: String,
    #[serde(rename = "ranges")]
    pub ranges: Vec<CoverageRange>,
    #[serde(rename = "isBlockCoverage")]
    pub is_block_coverage: bool,
}
#[derive(Debug, Deserialize, Clone)]
pub struct CoverageRange {
    #[serde(rename = "startOffset")]
    pub start_offset: u32,
    #[serde(rename = "endOffset")]
    pub end_offset: u32,
    pub count: u32,
}

#[derive(Clone, Default)]
pub struct CoverRangeNodeRead {
    children: Vec<CoverRangeNodeRead>,
    pub value: u32,
    left: u32,
    right: u32,
}

#[derive(Debug)]
pub struct CoverRangeNode {
    children: Vec<Rc<RefCell<CoverRangeNode>>>,
    pub value: u32,
    left: u32,
    right: u32,
}

impl CoverRangeNode {
    pub fn new(range: &CoverageRange) -> Self {
        CoverRangeNode {
            children: Vec::new(),
            value: range.count,
            left: range.start_offset,
            right: range.end_offset,
        }
    }
}

#[instrument(skip_all)]
pub fn build_coverage_range_tree(
    root: Rc<RefCell<CoverRangeNode>>,
    script_fn_cov: &Vec<FunctionCoverage>,
) {
    let _timer = Timer::new("构造覆盖率搜索树");
    let mut ranges = Vec::new();
    for x in script_fn_cov {
        for x in x.ranges.iter() {
            ranges.push(x);
        }
    }
    ranges.sort_unstable_by(|a, b| {
        let n = a.start_offset.cmp(&b.start_offset);
        if n != Ordering::Equal {
            return n;
        }
        a.end_offset.cmp(&b.end_offset)
    });
    for range in ranges {
        match find_root(root.clone(), range) {
            Some(r) => {
                r.borrow_mut()
                    .children
                    .push(Rc::new(RefCell::new(CoverRangeNode::new(range))));
            }
            None => {
                warn!("没有找到根节点: {:?}", range)
            }
        }
    }
}

pub fn find_root(
    root: Rc<RefCell<CoverRangeNode>>,
    range: &CoverageRange,
) -> Option<Rc<RefCell<CoverRangeNode>>> {
    let left = root.borrow().left;
    let right = root.borrow().right;
    if range.start_offset < left || range.end_offset > right {
        return None;
    }
    for child in root.borrow().children.iter() {
        if let Some(r) = find_root(child.clone(), range) {
            return Some(r);
        }
    }
    Some(root)
}

pub fn read_only(root: Rc<RefCell<CoverRangeNode>>) -> CoverRangeNodeRead {
    let mut r = CoverRangeNodeRead::default();
    r.value = root.borrow().value;
    r.left = root.borrow().left;
    r.right = root.borrow().right;
    for x in root.borrow().children.clone() {
        r.children.push(read_only(x.clone()));
    }
    r
}
pub fn find_root_value_only(root: &CoverRangeNodeRead, range: &CoverageRange) -> Option<u32> {
    let left = root.left;
    let right = root.right;
    if range.start_offset < left || range.end_offset > right {
        return None;
    }
    for child in root.children.iter() {
        if let Some(r) = find_root_value_only(child, range) {
            return Some(r);
        }
    }
    Some(root.value)
}

pub async fn normalize_script_coverages(
    script_coverages: &Vec<ScriptCoverageRaw>,
    filters: &Vec<String>,
) -> Result<Vec<ScriptCoverage>> {
    let mut r = Vec::new();
    for sc in script_coverages {
        let script_url = url_normalize(&sc.url);
        let script_name = url_filename(&script_url);
        if (filters.len() > 0 && filters.iter().find(|&f| script_url.contains(f)).is_some())
            || filters.is_empty()
        {
            let v = if let Some(s) = sc.source.clone() {
                ScriptCoverage {
                    url: script_name,
                    source: s.clone(),
                    functions: sc.functions.clone(),
                }
            } else {
                let s = get_uri_resource(&script_url).await.map_err(|e| anyhow!("请求URL失败: {} {}", &script_url, e))?;
                ScriptCoverage {
                    url: script_name,
                    source: s,
                    functions: sc.functions.clone(),
                }
            };
            r.push(v)
        }
    }
    Ok(r)
}

pub fn url_filename(u: &str) -> String {
    let s = u.split("?").next().unwrap_or_default();
    s.split("/").last().unwrap_or_default().to_string()
}
#[instrument]
pub async fn collect_coverage_helper(
    path_pattern: &str,
    coverage_filters: &Vec<String>,
) -> Result<HashMap<String, Vec<ScriptCoverage>>> {
    let _timer = Timer::new("收集本地覆盖率数据");
    let all_script_coverage_files = glob_abs(path_pattern)?;
    info!(
        "待处理的覆盖率报告文件列表 {:?}",
        &all_script_coverage_files
    );
    let mut all_script_coverages = HashMap::new();
    for p in all_script_coverage_files {
        debug!("处理覆盖率文件 {}", p.to_str().unwrap());
        let s = std::fs::read_to_string(&p)?;
        let sc_arr: Vec<ScriptCoverageRaw> =
            serde_json::from_str(&s).map_err(|e| anyhow!("解析{:?}出错, {}", p.to_str(), e))?;
        let sc_arr = normalize_script_coverages(&sc_arr, coverage_filters).await?;
        all_script_coverages.insert(p.to_str().unwrap().to_string(), sc_arr);
    }
    Ok(all_script_coverages)
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_build_coverage_range_tree() -> Result<()> {
        let inputs: Vec<ScriptCoverage> =
            serde_json::from_str(include_str!("../../tests/jsx/v8-coverage.json"))?;

        let root = Rc::new(RefCell::new(CoverRangeNode::new(&CoverageRange {
            start_offset: 0,
            end_offset: inputs[0].source.len() as u32,
            count: 0,
        })));
        build_coverage_range_tree(root.clone(), &inputs[0].functions);
        dbg!(&root);
        Ok(())
    }
}
