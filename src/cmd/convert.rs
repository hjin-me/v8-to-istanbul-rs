use crate::format::istanbul::IstanbulCov;
use crate::format::script_coverage::{collect_coverage_helper, ScriptCoverage};
use crate::statement::build_statements_from_local;
use crate::{handle_script_coverage, path_to_abs, relocate};
use anyhow::{anyhow, Result};
use clap::Args;
use std::collections::HashMap;
use tokio::fs;
use tracing::{error, info, trace, warn};

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
    fs::write(&d, b)
        .await
        .map_err(|e| anyhow!("写入报告失败 [{}] {}", &d, e))?;
    info!("搞定");
    Ok(())
}
