mod format;
mod translate;

use crate::format::istanbul;
use crate::format::istanbul::generate_source_code;
use crate::format::script_coverage::{
    build_coverage_range_tree, find_root, find_root_value_only, read_only, CoverRangeNode,
    CoverageRange, ScriptCoverage,
};
use crate::translate::source_map_link;
use anyhow::{anyhow, Result};
use clap::Parser;
use rayon::prelude::*;
use regex::Regex;
use sourcemap::SourceMap;
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use tokio::fs;
use tracing::{error, info, instrument, warn, Instrument};
use tracing_subscriber::EnvFilter;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(long)]
    coverages: Vec<String>,
    #[arg(long)]
    filters: Vec<String>,
}
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let args = Args::parse();

    info!("start");
    // 指定 ScriptCoverage 本地地址
    let sc_arr: Vec<ScriptCoverage> =
        serde_json::from_reader(File::open(args.coverages[0].clone())?)?;
    for sc in sc_arr {
        info!("处理脚本: {}", sc.url);
        match handle_script_coverage(&sc).await {
            Ok(s) => info!("{} 处理完成，结果在 {}", sc.url, s),
            Err(e) => error!("{} 失败 {}", sc.url, e),
        };
    }
    Ok(())
}
#[instrument(skip(sc), fields(url = sc.url))]
async fn handle_script_coverage(sc: &ScriptCoverage) -> Result<String> {
    let uid = url_key(&sc.url);
    info!("下载sourcemap");
    // 指定 SourceMap 下载地址
    let smb = reqwest::get(format!("{}.map", &sc.url))
        .await?
        .bytes()
        .await?;
    let sm = SourceMap::from_slice(&smb).map_err(|e| anyhow!("sourcemap 解析失败: {}", e))?;
    info!("生成源码目录");
    // 生成源码目录
    let base_dir = generate_source_code(&sm, &uid).await?;
    info!("生成中间文件");
    // 生成中间文件
    let mut vm = source_map_link(&sc, &sm)
        .await
        .map_err(|e| anyhow!("生成覆盖率中间数据失败, {}", e))?;
    let root = Rc::new(RefCell::new(CoverRangeNode::new(&CoverageRange {
        start_offset: 0,
        end_offset: sc.source.len() as u32,
        count: 0,
    })));
    info!("构造覆盖率搜索树");
    build_coverage_range_tree(root.clone(), &sc.functions);
    let cov_tree = read_only(root);
    info!("搜索覆盖率");
    let vm = vm
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
    info!("搜索覆盖率完成");

    // 生成 istanbul 文件
    info!("生成 istanbul 文件");
    let report = istanbul::from(&vm, &base_dir);
    // 执行 nyc 生成报告
    info!("执行 nyc 生成报告");
    fs::create_dir_all(format!("{}/.nyc_output/", base_dir)).await?;
    fs::write(
        format!("{}/.nyc_output/coverage.json", base_dir),
        serde_json::to_string_pretty(&report)?,
    )
    .await?;
    info!("生成报告成功 {}", base_dir);
    Ok(base_dir)
}

fn url_key(u: &str) -> String {
    // 定义一个正则表达式
    let re = Regex::new(r"\W+").unwrap();

    re.replace_all(u, "_").to_string()
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
}
