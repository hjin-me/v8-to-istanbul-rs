use crate::format::{source_map_key, MappingItem};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sourcemap::SourceMap;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IstanbulCov {
    pub path: String,
    #[serde(rename = "statementMap")]
    pub statement_map: HashMap<String, StatementMap>,
    pub s: HashMap<String, u32>,
    #[serde(rename = "branchMap")]
    pub branch_map: HashMap<String, BranchMap>,
    pub b: HashMap<String, Vec<u32>>,
    #[serde(rename = "fnMap")]
    pub fn_map: HashMap<String, FnMap>,
    pub f: HashMap<String, u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Location {
    pub start: Position,
    pub end: Position,
}
#[derive(Debug, Serialize, Deserialize)]
struct Position {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct FnMap {
    pub name: String,
    pub line: i32,
    pub loc: Location,
    pub decl: Option<Location>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BranchMap {
    pub line: i32,
    #[serde(rename = "type")]
    pub r#type: String,
    pub locations: Vec<Location>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StatementMap {
    pub start: Position,
    pub end: Position,
}

// nyc 生成覆盖率报告需要源代码
// 这里使用 source-map 生成源代码
pub async fn generate_source_code(source_map: &SourceMap) -> Result<String> {
    let key = source_map_key(source_map);
    let tmp_dir = env::temp_dir().join(format!("source-map/{}", key));
    // 递归创建 tmp_dir 目录
    fs::create_dir_all(&tmp_dir).await?;
    for (i, content) in source_map.source_contents().enumerate() {
        if let Some(p) = source_map.get_source(i as u32) {
            let path = tmp_dir.join(p);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(&path, content.unwrap_or_default()).await?;
        }
    }

    Ok(tmp_dir.to_str().unwrap().to_string())
}

pub fn from(vs: &Vec<MappingItem>, base_dir: &str) -> HashMap<String, IstanbulCov> {
    let base = Path::new(base_dir);
    let mut m = HashMap::new();
    for (key, x) in vs.iter().enumerate() {
        let abs_path = path_normalize(base.join(x.source).to_str().unwrap_or_default());
        let mut ic = IstanbulCov::default();
        ic.path = abs_path.to_string();
        let entry = m.entry(ic.path.clone()).or_insert(ic);
        entry.statement_map.insert(
            key.to_string(),
            StatementMap {
                start: Position {
                    line: x.original_line + 1,
                    column: x.original_column,
                },
                end: Position {
                    line: x.last_original_line + 1,
                    column: x.last_original_column,
                },
            },
        );
        entry.s.insert(key.to_string(), x.count);
    }
    m
}
fn path_normalize(path: &str) -> String {
    let mut r = vec![];
    for x in path.split('/') {
        match x {
            "." => {}
            ".." => {
                r.pop();
            }
            _ => {
                r.push(x);
            }
        }
    }
    r.join("/")
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_source_code() -> Result<()> {
        let source_map =
            SourceMap::from_slice(include_bytes!("../../tests/base/main.min.js.map"))?;
        dbg!(generate_source_code(&source_map).await?);

        Ok(())
    }
    #[test]
    fn test_join() {
        let mut a1 = PathBuf::from("/abc/def");
        let a2 = PathBuf::from("../xyz");
        assert_eq!(
            path_normalize(a1.join(a2).as_path().to_str().unwrap()),
            "/abc/xyz".to_string()
        );
        assert_eq!(
            path_normalize("./abc"),
            "abc".to_string()
        );
        assert_eq!(
            path_normalize("./abc/.."),
            "".to_string()
        );
        // dbg!(a1.clone().join(a2.clone()));
        // dbg!(a1.push(a2));
        // dbg!(a1.canonicalize().unwrap());
        //
        // let b = Path::new("/abc");
        // let t = Path::new("../a");
        // dbg!(b.join(t));
    }
}
