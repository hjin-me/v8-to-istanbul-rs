use crate::format::istanbul;
use crate::format::istanbul::IstanbulCov;
use crate::format::script_coverage::{
    build_coverage_range_tree, collect_coverage_helper, find_root_value_only, read_only,
    CoverRangeNode, CoverageRange, ScriptCoverage,
};
use crate::fputil::path_to_abs;
use crate::statement::{build_statements_from_local, Statement};
use crate::timer::Timer;
use anyhow::{anyhow, Result};
use clap::Args;
use rayon::prelude::*;
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tokio::fs;
use tracing::{error, info, instrument, trace, warn};

#[derive(Args)]
pub struct ConvertArgs {
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
#[instrument(skip(args))]
pub async fn exec(args: &ConvertArgs) -> Result<()> {
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

    let mut all_script_coverages = collect_coverage_helper(&args.pattern, &args.filters).await?;

    let output_dir = path_to_abs(&args.output)?.to_str().unwrap().to_string();
    fs::create_dir_all(format!("{}/.nyc_output/", output_dir)).await?;

    let statement_data = build_statements_from_local(
        &args.source_map_base,
        &args.url_base,
        &output_dir,
        &source_relocate,
    )
    .await?;

    let mut merged_result: HashMap<String, IstanbulCov> = HashMap::new();
    // 创建空覆盖率报告
    all_script_coverages.insert(
        "默认空覆盖率".to_string(),
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
        for sc in sc_arr {
            info!(test_name = test_name, url = sc.url, "关联ScriptCoverage");
            match handle_script_coverage(&statement_data, &sc) {
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
                Err(e) => error!("处理ScriptCoverage出错了:{}", e),
            };
        }
    }

    let d = format!("{}/.nyc_output/merged.json", output_dir);
    let b = serde_json::to_string_pretty(&merged_result)?;
    fs::write(&d, b)
        .await
        .map_err(|e| anyhow!("写入报告失败 [{}] {}", &d, e))?;
    info!("搞定");
    Ok(())
}

#[instrument(skip_all, fields(script = sc.url))]
fn handle_script_coverage(
    sd: &HashMap<String, Statement>,
    sc: &ScriptCoverage,
) -> Result<HashMap<String, IstanbulCov>> {
    let _timer = Timer::new("生成覆盖率报告");
    let statement = match sd.get(&sc.url) {
        Some(s) => s,
        None => {
            warn!("未找到覆盖率数据");
            return Ok(HashMap::new());
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
    fn test_relocate() {
        dbg!(relocate(r"%webpack://%%").unwrap());
    }
}
