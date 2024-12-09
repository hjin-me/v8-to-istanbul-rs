use crate::format::path_normalize;
use std::path::PathBuf;
use tracing::{instrument, trace};

const NOT_EXIST_DIR: &'static str = "/abc/def/xyz/817457891234/";
#[instrument]
pub fn is_legal_source_path(s: &str) -> bool {
    if s.starts_with("external script ") || s.starts_with("webpack:") || s.contains("node_modules")
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
