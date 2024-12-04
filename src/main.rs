mod format;
mod statement;
mod translate;
mod traverse;

use crate::format::istanbul::IstanbulCov;
use crate::format::script_coverage::{
    build_coverage_range_tree, collect_coverage_helper, find_root_value_only, read_only,
    CoverRangeNode, CoverageRange, ScriptCoverage,
};
use crate::format::{istanbul, path_normalize};
use crate::statement::{build_statements_from_local, Statement};
use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
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
use tokio::fs;
use tracing::{debug, error, info, instrument, trace, warn};
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
#[derive(Args)]
struct ConvertArgs {
    #[arg(long)]
    pattern: String,
    #[arg(long)]
    filters: Vec<String>,
    #[arg(long)]
    output: String,
    #[arg(long)]
    url_base: Option<String>, // 用来补全 source map 里面 file 的路径
    #[arg(long)]
    source_map_base: String, // 本地 source map 文件所在的根目录
    #[arg(long)]
    source_relocate: Option<String>, // 用来替换 source map 里面 sources 的路径
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
        Commands::Convert(args) => {
            info!("开干");
            // 先处理输入参数
            let source_relocate = args
                .source_relocate
                .clone()
                .map(|s| match relocate(&s) {
                    Ok(r) => Some(r),
                    Err(err) => {
                        warn!("解析 source_relocate 失败 {}", err);
                        None
                    }
                })
                .flatten();

            let mut all_script_coverages =
                collect_coverage_helper(&args.pattern, &args.filters).await?;

            let output_dir = path_to_abs(&args.output)?.to_str().unwrap().to_string();
            fs::create_dir_all(format!("{}/.nyc_output/", output_dir)).await?;

            let statement_data = build_statements_from_local(
                &args.source_map_base,
                &args.url_base,
                &output_dir,
                &source_relocate,
            )
            .await?;
            dbg!(&statement_data.keys());

            let mut merged_result: HashMap<String, IstanbulCov> = HashMap::new();
            // 创建空覆盖率报告
            all_script_coverages.insert(
                "empty_report".to_string(),
                statement_data
                    .iter()
                    .map(|(k, _)| ScriptCoverage {
                        url: k.to_string(),
                        source: "".to_string(),
                        functions: vec![],
                    })
                    .collect(),
            );

            for (test_name, sc_arr) in all_script_coverages {
                // let test_name_hash = hash(&test_name);
                for sc in sc_arr {
                    info!("处理脚本: {} {}", test_name, sc.url);
                    match handle_script_coverage(&statement_data, &sc).await {
                        Ok(report) => {
                            trace!("执行 nyc 生成报告");
                            for (k, v) in report {
                                let e = merged_result.entry(k).or_insert(IstanbulCov::default());
                                e.path = v.path;
                                for (index, s) in v.statement_map {
                                    e.statement_map.insert(index, s);
                                }
                                for (index, count) in v.s {
                                    let c = e.s.entry(index).or_insert(0);
                                    *c += count;
                                }
                            }
                        }
                        Err(e) => error!("{}", e),
                    };
                }
            }

            let d = format!("{}/.nyc_output/merged.json", output_dir);
            let b = serde_json::to_string_pretty(&merged_result)?;
            match fs::write(&d, b).await {
                Ok(_) => info!("搞定"),
                Err(e) => error!("写入报告失败 [{}] {}", &d, e),
            }
        }
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
