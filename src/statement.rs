use crate::format::istanbul::generate_source_code;
use crate::format::script_coverage::ScriptCoverage;
use crate::format::MappingItem;
use crate::fputil::{get_uri_resource, glob_abs};
use crate::timer::Timer;
use crate::translate::source_map_link;
use anyhow::anyhow;
use anyhow::Result;
use regex::Regex;
use sourcemap::SourceMap;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, instrument, trace, warn};
use url::Url;

#[derive(Debug)]
pub struct Statement {
    pub source_url: String,
    pub code_dir: String,
    pub mapping: Vec<MappingItem>,
}

#[instrument]
pub async fn build_statements_from_local(
    source_map_pattern: &str,
    url_base: &Option<String>,
    project_dir: &str,
    source_relocate: &Option<(Regex, String)>,
) -> Result<HashMap<String, Statement>> {
    let _timer = Timer::new("本地构造Statements");
    let mut cache_data = HashMap::new();
    let all_source_map_files = glob_abs(source_map_pattern)?;
    info!("待处理的SourceMap文件列表 {:?}", &all_source_map_files);
    for p in all_source_map_files {
        let (script_name, statement) =
            handle_sourcemap_file(p.to_str().unwrap(), url_base, project_dir, source_relocate)
                .await?;
        cache_data.insert(script_name, statement);
    }

    Ok(cache_data)
}
#[instrument(skip_all, fields(file=p))]
async fn handle_sourcemap_file(
    p: &str,
    uri_base: &Option<String>,
    project_dir: &str,
    source_relocate: &Option<(Regex, String)>,
) -> Result<(String, Statement)> {
    let _timer = Timer::new("处理SourceMap文件");
    trace!("处理SourceMap文件");
    let sm = source_map_from_file(&p, source_relocate).await?;
    let script_uri = if let Some(ub) = uri_base {
        format!("{}{}", ub, sm.get_file().unwrap_or_default())
    } else {
        sm.get_file().unwrap_or_default().to_string()
    };

    debug!(script_uri = &script_uri, "下载SourceMap对应的JS文件");
    let source_content = get_uri_resource(&script_uri).await?;

    debug!("生成map中间文件");
    let vm = source_map_link(&source_content, &sm)
        .await
        .map_err(|e| anyhow!("生成覆盖率中间数据失败: {}", e))?;
    let script_name = crate::format::script_coverage::url_filename(&script_uri);
    Ok((
        script_name,
        Statement {
            source_url: script_uri,
            code_dir: project_dir.to_string(),
            mapping: vm,
        },
    ))
}

pub async fn build_statements(
    script_coverages: &Vec<&ScriptCoverage>,
    output_dir: &str,
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

#[instrument]
async fn source_map_from_file<P: AsRef<Path> + fmt::Debug>(
    p: P,
    source_relocate: &Option<(Regex, String)>,
) -> Result<SourceMap> {
    let s = fs::read_to_string(&p).await.map_err(|err| {
        anyhow!(
            "读取SourceMap失败: {}, {}",
            err,
            &p.as_ref().to_string_lossy()
        )
    })?;
    trace!("解码 source map");
    let mut sm =
        SourceMap::from_slice(s.as_bytes()).map_err(|e| anyhow!("sourcemap 解析失败: {}", e))?;

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
    Ok(sm)
}
#[instrument()]
async fn source_map_from_url(
    u: &str,
    source_relocate: Option<(Regex, String)>,
) -> Result<SourceMap> {
    let s = get_uri_resource(u).await?;
    trace!("解码 source map");
    let mut sm =
        SourceMap::from_slice(s.as_bytes()).map_err(|e| anyhow!("sourcemap 解析失败: {}", e))?;

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
    Ok(sm)
}

#[instrument(skip(source, output_dir))]
async fn gen_cache_data<'a>(
    url: &'a str,
    source: &'a str,
    output_dir: &'a str,
    use_local: bool,
    source_map_base: Option<String>,
    source_relocate: Option<(Regex, String)>,
) -> Result<Statement> {
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
        Some(s) => fs::read_to_string(&s)
            .await
            .map_err(|e| anyhow!("读取 sourcemap 路径错误: {}", e))?,
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
    let base_dir = output_dir.to_string();
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

pub fn url_normalize(u: &str) -> String {
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
        if std::fs::metadata(&path).is_ok() {
            debug!("文件名: {}", path.to_string_lossy().to_string());
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}
