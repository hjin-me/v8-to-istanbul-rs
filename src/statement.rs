use crate::format::istanbul::generate_source_code;
use crate::format::script_coverage::ScriptCoverage;
use crate::format::MappingItem;
use crate::translate::source_map_link;
use anyhow::anyhow;
use anyhow::Result;
use regex::Regex;
use sourcemap::SourceMap;
use std::collections::HashMap;
use tracing::{info, instrument, trace, warn};

pub struct Statement {
    pub source_url: String,
    pub code_dir: String,
    pub mapping: Vec<MappingItem>,
}

pub async fn build_statements(
    script_coverages: &Vec<&ScriptCoverage>,
    output_dir: &str,
    merge: bool,
    use_local: bool,
) -> Result<HashMap<String, Statement>> {
    let mut source_map_url = HashMap::new();
    for &sc in script_coverages {
        source_map_url.insert(&sc.url, &sc.source);
    }
    let mut cache_data = HashMap::new();
    for (url, source) in source_map_url {
        match gen_cache_data(url, source, output_dir, merge, use_local).await {
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
    merge: bool,
    use_local: bool,
) -> Result<Statement> {
    let uid = url_key(&url);
    info!("下载 source map 文件 {}.map", &url);
    let smb = reqwest::get(&format!("{}.map", &url))
        .await?
        .bytes()
        .await?;
    trace!("解码 source map");
    let sm = SourceMap::from_slice(&smb).map_err(|e| anyhow!("sourcemap 解析失败: {}", e))?;
    // 生成源码目录
    let base_dir = if merge {
        output_dir.to_string()
    } else {
        format!("{}/{}", output_dir, uid)
    };
    if !use_local {
        trace!("生成源码目录 {}", base_dir);
        generate_source_code(&sm, &base_dir).await?;
    }
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
