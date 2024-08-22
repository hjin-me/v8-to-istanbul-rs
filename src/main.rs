mod format;
mod statement;
mod translate;

use crate::format::istanbul::IstanbulCov;
use crate::format::script_coverage::{
    build_coverage_range_tree, find_root_value_only, normalize_script_coverages, read_only,
    CoverRangeNode, CoverageRange, ScriptCoverage, ScriptCoverageRaw,
};
use crate::format::{istanbul, path_normalize};
use crate::statement::{build_statements, Statement};
use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
use glob::glob;
use rayon::prelude::*;
use regex::Regex;
use sha1::{Digest, Sha1};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use tokio::fs;
use tracing::{error, info, instrument, trace};
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
    #[arg(long, default_value = "false")]
    merge: bool,
    #[arg(long, default_value = "false")]
    use_local: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Convert(args) => {
            info!("start");
            let all_script_coverage_filess = glob_abs(&args.pattern)?;
            info!(
                "待处理的覆盖率报告文件列表 {:?}",
                &all_script_coverage_filess
            );
            let mut all_script_coverages = HashMap::new();
            for p in all_script_coverage_filess {
                trace!("处理文件 {}", p.to_str().unwrap());
                let mut s = String::new();
                File::open(&p)?.read_to_string(&mut s)?;
                let sc_arr: Vec<ScriptCoverageRaw> = serde_json::from_str(&s)
                    .map_err(|e| anyhow!("解析{:?}出错, {}", p.to_str(), e))?;
                let sc_arr = normalize_script_coverages(&sc_arr, &args.filters).await?;
                all_script_coverages.insert(p.to_str().unwrap().to_string(), sc_arr);
            }

            let output_dir = path_to_abs(&args.output)?.to_str().unwrap().to_string();
            fs::create_dir_all(format!("{}/.nyc_output/", output_dir)).await?;

            // 先把 script coverage 和 source map 这两批静态且重复的文件处理好
            let sc_arr = all_script_coverages
                .values()
                .flatten()
                .collect::<Vec<&ScriptCoverage>>();
            let statement_data =
                build_statements(&sc_arr, &output_dir, args.merge, args.use_local).await?;
            let mut reports = vec![];

            for (test_name, sc_arr) in all_script_coverages {
                let test_name_hash = hash(&test_name);
                for sc in sc_arr {
                    info!("处理脚本: {} {}", test_name, sc.url);
                    match handle_script_coverage(&statement_data, &sc).await {
                        Ok(report) => {
                            trace!("执行 nyc 生成报告");
                            if !args.merge {
                                let uid = format!("{}_{}", test_name_hash, hash(&sc.url));
                                let b = match serde_json::to_string_pretty(&report) {
                                    Ok(b) => b,
                                    Err(e) => {
                                        error!("序列化报告失败: {}", e);
                                        continue;
                                    }
                                };
                                let d = format!("{}/.nyc_output/{}.json", output_dir, uid);
                                match fs::write(&d, b).await {
                                    Ok(_) => {}
                                    Err(e) => error!("写入报告失败 [{}] {}", &d, e),
                                }
                            } else {
                                reports.push(report)
                            }
                        }
                        Err(e) => error!("{} 失败 {}", sc.url, e),
                    };
                }
            }
            if args.merge {
                let mut result: HashMap<String, IstanbulCov> = HashMap::new();
                for x in reports {
                    for (k, v) in x {
                        let e = result.entry(k).or_insert(IstanbulCov::default());
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
                let d = format!("{}/.nyc_output/merged.json", output_dir);
                let b = serde_json::to_string_pretty(&result)?;
                match fs::write(&d, b).await {
                    Ok(_) => {}
                    Err(e) => error!("写入报告失败 [{}] {}", &d, e),
                }
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

    // 生成 istanbul 文件
    trace!("生成 istanbul 文件");
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
}
