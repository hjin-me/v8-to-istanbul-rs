use crate::format::istanbul::generate_source_code;
use crate::format::script_coverage::ScriptCoverage;
use crate::format::MappingItem;
use crate::translate::source_map_link;
use anyhow::anyhow;
use anyhow::Result;
use regex::Regex;
use sourcemap::SourceMap;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, instrument, trace, warn};
use url::Url;

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
    source_map_base: Option<String>,
    source_relocate: Option<(Regex, String)>,
) -> Result<HashMap<String, Statement>> {
    let mut source_map_url = HashMap::new();
    for &sc in script_coverages {
        source_map_url.insert(&sc.url, &sc.source);
    }
    let mut cache_data = HashMap::new();
    for (url, source) in source_map_url {
        match gen_cache_data(
            url,
            source,
            output_dir,
            merge,
            use_local,
            source_map_base.clone(),
            source_relocate.clone(),
        )
        .await
        {
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
    source_map_base: Option<String>,
    source_relocate: Option<(Regex, String)>,
) -> Result<Statement> {
    let uid = url_key(&url);
    let url = url_normalize(url);

    let sm_path = match source_map_base {
        Some(s) => {
            // 获取 source map 地址
            get_js_filename(&url, &s)
        }
        None => None,
    };
    // source map 可以从网络下载，或者本地查找
    let smb = match sm_path {
        Some(s) => fs::read_to_string(&s).map_err(|e| anyhow!("读取 sourcemap 路径错误: {}", e))?,
        None => reqwest::get(&format!("{}.map", &url)).await?.text().await?,
    };

    trace!("解码 source map");
    let mut sm =
        SourceMap::from_slice(smb.as_bytes()).map_err(|e| anyhow!("sourcemap 解析失败: {}", e))?;

    // source 字段对应的文件路径需要重新定位一下
    if let Some((re, replace)) = source_relocate {
        let n = sm.get_source_count();
        for i in 0..n {
            if let Some(s) = sm.get_source(i) {
                let s = re.replace(s, replace.as_str()).to_string();
                sm.set_source(i, s.as_str())
            }
        }
    }
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

fn url_normalize(u: &str) -> String {
    if u.starts_with("//") {
        format!("https:{}", u)
    } else {
        u.to_string()
    }
}

fn get_js_filename(u: &str, source_map_base: &str) -> Option<String> {
    let u = match Url::parse(u) {
        Ok(url) => url,
        Err(err) => {
            warn!("解析URL错误: {}", err);
            return None;
        }
    };
    // 获取路径
    let path_segments = u.path_segments()?;

    let mut path = PathBuf::new();
    path.push(source_map_base);
    // 获取最后一个路径段作为文件名
    if let Some(filename) = path_segments.last() {
        path.push(format!("{}.map", filename));
        if fs::metadata(&path).is_ok() {
            debug!("文件名: {}", path.to_string_lossy().to_string());
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}
