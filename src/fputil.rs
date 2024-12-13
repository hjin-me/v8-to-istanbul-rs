use crate::format::path_normalize;
use crate::timer::Timer;
use anyhow::{anyhow, Result};
use glob::glob;
use regex::Regex;
use sha1::{Digest, Sha1};
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::fs;
use tracing::{instrument, trace};
use url::Url;

const NOT_EXIST_DIR: &'static str = "/abc/def/xyz/817457891234/";
#[instrument]
pub fn is_legal_source_path(s: &str) -> bool {
    if s.starts_with("external script ")
        || s.starts_with("webpack:")
        || s.starts_with("http:/")
        || s.starts_with("https:/")
        || s.contains("node_modules")
    {
        return false;
    }
    let path = PathBuf::from(NOT_EXIST_DIR).join(s);
    if !path_normalize(&path.to_str().unwrap()).starts_with(NOT_EXIST_DIR) {
        trace!("source 路径跳出了当前目录，忽略这个文件: {}", s);
        return false;
    }
    true
}

pub fn url_key(u: &str) -> String {
    // 定义一个正则表达式
    let re = Regex::new(r"\W+").unwrap();

    re.replace_all(u, "_").to_string()
}

pub fn glob_abs(pattern: &str) -> anyhow::Result<Vec<PathBuf>> {
    let paths = glob(pattern)?;
    Ok(paths
        .filter_map(|p| p.ok())
        .filter_map(|p| path_to_abs(p).ok())
        .collect())
}

pub fn path_to_abs<P: AsRef<Path>>(p: P) -> anyhow::Result<PathBuf>
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

#[instrument]
pub async fn get_uri_resource(uri: &str) -> Result<String> {
    let _timer = Timer::new(&format!("获取资源{}", uri));
    let u = Url::parse(uri)?;
    match u.scheme() {
        "file" => {
            let p = PathBuf::from(u.path());
            Ok(fs::read_to_string(p).await?)
        }
        "https" | "http" => {
            let resp = reqwest::get(uri).await?;
            if !resp.status().is_success() {
                return Err(anyhow!("请求远程资源失败: [http={}]{}", resp.status(), &uri));
            }
            Ok(resp.text().await?)
        }
        _ => Err(anyhow!("unsupported scheme: {}", u.scheme())),
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use url::Url;

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
    fn test_url() {
        dbg!(Url::parse("file://user/local/abc"));
        dbg!(Url::parse("https://wer.com/asdf.jasd"));
        dbg!(Url::parse("ftp://asdfjkl.zxjcvklasdf/asdjfkl"));
        dbg!(Url::parse("//asdfjkl.zxjcvklasdf/asdjfkl"));
    }
}
