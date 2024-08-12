mod format;
mod translate;

use crate::format::istanbul;
use crate::format::istanbul::generate_source_code;
use crate::format::script_coverage::{
    build_coverage_range_tree, find_root, find_root_value_only, read_only, CoverRangeNode,
    CoverageRange, ScriptCoverage,
};
use crate::translate::source_map_link;
use anyhow::Result;
use clap::Parser;
use rayon::prelude::*;
use sourcemap::SourceMap;
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use tokio::fs;
use tracing::{info, Instrument};
use tracing_subscriber::EnvFilter;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(long)]
    coverages: Vec<String>,
}
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let args = Args::parse();

    info!("start");
    // 指定 ScriptCoverage 本地地址
    let sc: Vec<ScriptCoverage> = serde_json::from_reader(File::open(args.coverages[0].clone())?)?;
    let sc = sc[0].clone();

    info!("下载sourcemap");
    // 指定 SourceMap 下载地址
    let smb = reqwest::get(format!("{}.map", &sc.url))
        .await?
        .bytes()
        .await?;
    let sm = SourceMap::from_slice(&smb)?;
    info!("生成源码目录");
    // 生成源码目录
    let base_dir = generate_source_code(&sm).await?;
    info!("生成中间文件");
    // 生成中间文件
    let mut vm = source_map_link(&sc, &sm).await;
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
    fs::write("./demo.json", serde_json::to_string_pretty(&report)?).await?;
    info!("生成报告成功 {}", base_dir);
    Ok(())
}
