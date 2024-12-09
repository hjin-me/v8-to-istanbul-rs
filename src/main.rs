mod cmd;
mod format;
mod fputil;
mod statement;
mod translate;
mod traverse;

use crate::cmd::convert;
use crate::cmd::convert::ConvertArgs;
use crate::format::istanbul::IstanbulCov;
use crate::format::script_coverage::{
    build_coverage_range_tree, find_root_value_only, read_only, CoverRangeNode, CoverageRange,
    ScriptCoverage,
};
use crate::format::{istanbul, path_normalize};
use crate::statement::Statement;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use glob::glob;
use rayon::prelude::*;
use regex::Regex;
use sha1::{Digest, Sha1};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use std::{env, time};
use tracing::{info, instrument, trace};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Convert(ConvertArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let _t = Timer::new();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Convert(args) => convert::exec(args).await?,
    }
    Ok(())
}
#[instrument(skip_all, fields(url = sc.url))]
async fn handle_script_coverage(
    sd: &HashMap<String, Statement>,
    sc: &ScriptCoverage,
) -> Result<HashMap<String, IstanbulCov>> {
    let statement = match sd.get(&sc.url) {
        Some(s) => s,
        None => {
            return Err(anyhow!("未找到覆盖率数据 {}", &sc.url));
        }
    };
    let root = Rc::new(RefCell::new(CoverRangeNode::new(&CoverageRange {
        start_offset: 0,
        end_offset: sc.source.len() as u32,
        count: 0,
    })));
    trace!("构造覆盖率搜索树");
    build_coverage_range_tree(root.clone(), &sc.functions);
    let cov_tree = read_only(root);
    trace!("搜索覆盖率");
    let vm = statement
        .mapping
        .par_iter()
        .map(|m| {
            let mut m = m.clone();
            if let Some(n) = find_root_value_only(
                &cov_tree,
                &CoverageRange {
                    start_offset: m.generated_column,
                    end_offset: m.last_generated_column,
                    count: 0,
                },
            ) {
                m.count = n;
            }
            m
        })
        .collect();
    trace!("搜索覆盖率完成");

    trace!("生成istanbul报告");
    let report = istanbul::from(&vm, &statement.code_dir);
    Ok(report)
}

fn url_key(u: &str) -> String {
    // 定义一个正则表达式
    let re = Regex::new(r"\W+").unwrap();

    re.replace_all(u, "_").to_string()
}

fn glob_abs(pattern: &str) -> Result<Vec<PathBuf>> {
    let paths = glob(pattern)?;
    Ok(paths
        .filter_map(|p| p.ok())
        .filter_map(|p| path_to_abs(p).ok())
        .collect())
}

fn path_to_abs<P: AsRef<Path>>(p: P) -> Result<PathBuf>
where
    PathBuf: From<P>,
{
    let p = PathBuf::from(p);
    if !p.is_absolute() {
        let mut cwd = env::current_dir()?;
        cwd.push(p);
        Ok(PathBuf::from_str(&path_normalize(
            cwd.to_str().ok_or(anyhow!("cwd 失败"))?,
        ))?)
    } else {
        Ok(p)
    }
}

pub fn hash(s: &str) -> String {
    // Create a Sha1 object
    let mut hasher = Sha1::new();

    // Write the input data to the hasher
    hasher.update(s);

    // Finalize the hash and obtain the result as a byte array
    let result = hasher.finalize();

    // Convert the result to a hexadecimal string
    hex::encode(result)
}

struct Timer {
    start: time::Instant,
}
impl Timer {
    fn new() -> Self {
        Timer {
            start: time::Instant::now(),
        }
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let now = time::Instant::now();
        info!("任务执行耗时: {:.3}s", (now - self.start).as_secs_f32());
    }
}

fn relocate(pattern: &str) -> Result<(Regex, String)> {
    if pattern.is_empty() {
        return Err(anyhow!("pattern is empty"));
    }
    if let Some(first_char) = pattern.chars().next() {
        let mut s = pattern.split(first_char);
        s.next().ok_or(anyhow!("第一段字符串找不到"))?;
        let reg = s.next().unwrap_or_default();
        let replace = s.next().unwrap_or_default();
        let re = Regex::new(reg)?;
        Ok((re, replace.to_string()))
    } else {
        Err(anyhow!("pattern is empty"))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_key() {
        let u = "https://at.alicdn.com/t/font_1403768_rykyhcckct9.js";
        let key = url_key(u);
        assert_eq!(key, "https_at_alicdn_com_t_font_1403768_rykyhcckct9_js")
    }

    #[test]
    fn test_glob() {
        let cwd = env::current_dir().unwrap();
        assert_eq!(
            glob_abs("tests/base/**/*.json").unwrap(),
            glob_abs(&format!("{}/tests/base/**/*.json", cwd.to_str().unwrap())).unwrap()
        );
    }

    #[test]
    fn test_relocate() {
        dbg!(relocate(r"%webpack://%%").unwrap());
    }
}
