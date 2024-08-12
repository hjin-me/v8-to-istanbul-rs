use serde::Deserialize;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use tracing::warn;

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

pub fn build_coverage_range_tree(
    root: Rc<RefCell<CoverRangeNode>>,
    script_fn_cov: &Vec<FunctionCoverage>,
) {
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
