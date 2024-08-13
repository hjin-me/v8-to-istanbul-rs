use crate::format::istanbul::generate_source_code;
use crate::format::script_coverage::ScriptCoverage;
use crate::format::MappingItem;
use crate::translate::source_map_link;
use anyhow::anyhow;
use anyhow::Result;
use derive_builder::Builder;
use rayon::prelude::*;
use regex::Regex;
use reqwest::Url;
use sourcemap::SourceMap;
use std::collections::{HashMap, HashSet};
use tracing::{error, info, instrument, trace, warn};

pub struct Statement {
    pub source_url: String,
    pub code_dir: String,
    pub mapping: Vec<MappingItem>,
}

pub async fn build_statements(
    script_coverages: &Vec<&ScriptCoverage>,
    filters: &Vec<String>,
    output_dir: &str,
) -> Result<HashMap<String, Statement>> {
    let mut source_map_url = HashMap::new();
    for &sc in script_coverages {
        if filters.len() > 0 && filters.iter().find(|&f| sc.url.contains(f)).is_some() {
            source_map_url.insert(&sc.url, &sc.source);
        }
    }
    let mut cache_data = HashMap::new();
    for (url, source) in source_map_url {
        match gen_cache_data(url, source, output_dir).await {
            Ok(d) => {
                cache_data.insert(url.to_string(), d);
            }
            Err(e) => warn!("{} 失败 {}", url, e),
        };
    }

    Ok(cache_data)
}

#[instrument(skip(source, output_dir))]
async fn gen_cache_data<'a>(
    url: &'a str,
    source: &'a str,
    output_dir: &'a str,
) -> Result<Statement> {
    let uid = url_key(&url);
    trace!("下载source map 文件");
    let smb = reqwest::get(&format!("{}.map", &url))
        .await?
        .bytes()
        .await?;
    trace!("解码 source map");
    let sm = SourceMap::from_slice(&smb).map_err(|e| anyhow!("sourcemap 解析失败: {}", e))?;
    // 生成源码目录
    let base_dir = format!("{}/{}", output_dir, uid);
    trace!("生成源码目录 {}", base_dir);
    generate_source_code(&sm, &base_dir).await?;
    // 生成中间文件
    trace!("生成中间文件");
    let vm = source_map_link(&source, &sm)
        .await
        .map_err(|e| anyhow!("生成覆盖率中间数据失败, {}", e))?;
    trace!("源码预处理完成");
    Ok(Statement {
        source_url: url.to_string(),
        code_dir: base_dir,
        mapping: vm,
    })
}

fn url_key(u: &str) -> String {
    // 定义一个正则表达式
    let re = Regex::new(r"\W+").unwrap();

    re.replace_all(u, "_").to_string()
}
