use crate::format::MappingItem;
use anyhow::{anyhow, Result};
use sourcemap::SourceMap;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, instrument};

#[instrument(skip(source_content, source_map))]
pub async fn source_map_link<'a>(
    source_content: &'a str,
    source_map: &'a SourceMap,
) -> Result<Vec<MappingItem>> {
    let mut generated_source_sect = vec![0];
    for s in source_content.split('\n') {
        let last = generated_source_sect.last().unwrap();
        generated_source_sect.push(last + s.chars().count() as u32 + 1)
    }

    let mut line_length_map: HashMap<&str, Vec<u32>> = HashMap::new();
    for (i, s) in source_map.sources().enumerate() {
        if s.starts_with("external script ")
            || s.starts_with("webpack:")
            || s.contains("node_modules")
        {
            continue;
        }
        debug!("source path = {}", s);
        line_length_map.insert(
            s,
            source_map
                .get_source_contents(i as u32)
                .ok_or(anyhow!("source contents not found"))?
                .split('\n')
                .map(|s| s.len() as u32)
                .collect(),
        );
    }

    let mut sector_map = vec![];
    let n = source_map.get_token_count() as usize;
    for i in 0..n - 1 {
        if let Some(x) = source_map.get_token(i) {
            let source = x.get_source().unwrap_or_default();
            let (line, col) = x.get_dst();
            let start = generated_source_sect[line as usize] + col;
            let next = if let Some(n) = source_map.get_token(i + 1) {
                let (line, col) = n.get_dst();
                generated_source_sect[line as usize] + col - 1
            } else {
                start
            };
            if source.starts_with("external script ")
                || source.starts_with("webpack:")
                || source.contains("node_modules")
            {
                continue;
            }
            let m = MappingItem {
                source: source.to_string(),
                generated_column: start,
                last_generated_column: next,
                original_line: x.get_src_line(),
                original_column: x.get_src_col(),
                last_original_line: x.get_src_line(),
                last_original_column: x.get_src_col(),
                count: 0,
                idx: i,
            };
            sector_map.push(m);
        }
    }
    if let Some(x) = source_map.get_token(n - 1) {
        let (line, col) = x.get_dst();
        let start = generated_source_sect[line as usize] + col;
        let end = generated_source_sect[line as usize + 1] - 1;
        let source = x.get_source().unwrap_or_default();
        let m = MappingItem {
            source: source.to_string(),
            generated_column: start,
            last_generated_column: end,
            original_line: x.get_src_line(),
            original_column: x.get_src_col(),
            last_original_line: x.get_src_line(),
            last_original_column: x.get_src_col(),
            count: 0,
            idx: n - 1,
        };
        if !(source.starts_with("external script ")
            || source.starts_with("webpack:")
            || source.contains("node_modules"))
        {
            sector_map.push(m);
        }
    }

    sector_map.sort_unstable_by(|a, b| {
        let n = a.source.cmp(&b.source);
        if n != Ordering::Equal {
            return n;
        }
        let n = a.original_line.cmp(&b.original_line);
        if n != Ordering::Equal {
            return n;
        }
        let n = a.original_column.cmp(&b.original_column);
        if n != Ordering::Equal {
            return n;
        }
        a.generated_column.cmp(&b.generated_column)
    });
    let mut last_idx = vec![0];
    for i in 1..sector_map.len() {
        // 上一个 token 和这个token 是同一个源文件的
        if sector_map[last_idx[0]].source == sector_map[i].source
            && sector_map[last_idx[0]].original_line == sector_map[i].original_line
            && sector_map[last_idx[0]].original_column == sector_map[i].original_column
        {
            last_idx.push(i);
            continue;
        }
        // 收集一下待处理的状态
        let (source, start_line, start_column) = (
            sector_map[last_idx[0]].source.as_str(),
            sector_map[last_idx[0]].original_line,
            sector_map[last_idx[0]].original_column,
        );
        let (cross_line, lines_length) = match line_length_map.get(source) {
            Some(v) => {
                if v.len() <= start_line as usize {
                    return Err(anyhow!("行坐标比代码行数更大, {}>{}", start_line, v.len()));
                }
                (
                    start_column >= v[start_line as usize], // 当前 token 起始位置已经超过行的长度
                    v,
                )
            }
            None => {
                // dbg!(
                //     source,
                //     line_length_map.get(source),
                //     &sector_map[last_idx[0]]
                // );
                last_idx = vec![i];
                continue;
            }
        };
        let (next_source, next_start_line, next_start_column) = (
            sector_map[i].source.as_str(),
            sector_map[i].original_line,
            sector_map[i].original_column,
        );
        let cross_source = source != next_source;

        // dbg!(
        //     cross_line,
        //     cross_source,
        //     source != next_source,
        //     source,
        //     start_line,
        //     start_column,
        //     next_source,
        //     next_start_line,
        //     next_start_column
        // );
        for idx in last_idx.iter() {
            match (cross_line, cross_source) {
                (true, true) => {
                    let mut prev_i = lines_length.len() - 1;
                    while prev_i >= start_line as usize && lines_length[prev_i] == 0 {
                        prev_i -= 1;
                    }
                    // dbg!(prev_i, lines_length);
                    sector_map[*idx].original_line = sector_map[*idx].original_line + 1;
                    sector_map[*idx].original_column = 0;
                    sector_map[*idx].last_original_line = prev_i as u32;
                    sector_map[*idx].last_original_column = lines_length[prev_i] - 1;
                }
                (true, false) => {
                    if next_start_column > 0 {
                        sector_map[*idx].last_original_line = next_start_line;
                        sector_map[*idx].last_original_column = next_start_column - 1;
                    } else {
                        let mut prev_i = next_start_line as usize - 1;
                        while prev_i >= start_line as usize && lines_length[prev_i] == 0 {
                            prev_i -= 1;
                        }
                        if prev_i == *idx {
                            sector_map[*idx].last_original_line = sector_map[*idx].original_line;
                            sector_map[*idx].last_original_column =
                                sector_map[*idx].original_column;
                        } else {
                            sector_map[*idx].last_original_line = prev_i as u32;
                            sector_map[*idx].last_original_column = lines_length[prev_i] - 1;
                        }
                    }
                    sector_map[*idx].original_line = sector_map[*idx].original_line + 1;
                    sector_map[*idx].original_column = 0;
                }
                (false, true) => {
                    sector_map[*idx].last_original_line = sector_map[*idx].original_line;
                    sector_map[*idx].last_original_column = lines_length[start_line as usize] - 1;
                }
                (false, false) => {
                    if sector_map[*idx].original_line == sector_map[i].original_line {
                        sector_map[*idx].last_original_line = sector_map[i].original_line;
                        sector_map[*idx].last_original_column = sector_map[i].original_column - 1;
                    } else {
                        sector_map[*idx].last_original_line = sector_map[*idx].original_line;
                        sector_map[*idx].last_original_column =
                            lines_length[start_line as usize] - 1;
                    }
                }
            }
            // dbg!(&sector_map[*idx]);
        }
        last_idx = vec![i]
    }

    if !sector_map.is_empty() {
        // 收集一下待处理的状态
        let (source, start_line, start_column) = (
            sector_map[last_idx[0]].source.as_str(),
            sector_map[last_idx[0]].original_line,
            sector_map[last_idx[0]].original_column,
        );
        if let Some(v) = line_length_map.get(source) {
            let (cross_line, lines_length) = (
                start_column >= v[start_line as usize], // 当前 token 起始位置已经超过行的长度
                v,
            );
            for idx in last_idx.iter() {
                match cross_line {
                    true => {
                        let mut prev_i = lines_length.len() - 1;
                        while prev_i >= start_line as usize && lines_length[prev_i] == 0 {
                            prev_i -= 1;
                        }
                        sector_map[*idx].last_original_line = prev_i as u32;
                        sector_map[*idx].last_original_column = lines_length[prev_i] - 1;
                    }

                    false => {
                        sector_map[*idx].last_original_line = sector_map[*idx].original_line;
                        sector_map[*idx].last_original_column =
                            lines_length[start_line as usize] - 1;
                    }
                }
            }
        }
    }

    Ok(sector_map
        .into_iter()
        .filter(|s| is_file_extension_allowed(s.source.as_str(), &["js", "jsx", "ts", "tsx"]))
        .collect())
}

fn is_file_extension_allowed<P: AsRef<Path>>(path: P, file_extensions: &[&str]) -> bool {
    let ext = path
        .as_ref()
        .extension()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or_default();
    file_extensions.contains(&ext)
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::{anyhow, Result};
    use assert_json_diff::assert_json_eq;

    #[tokio::test]
    async fn test_source_map_link_base() -> Result<()> {
        let script_coverage = serde_json::from_str::<
            Vec<crate::format::script_coverage::ScriptCoverage>,
        >(include_str!("../tests/base/v8-coverage.json"))
        .map_err(|e| anyhow!("parse script coverage error: {}", e))?;
        let source_map = SourceMap::from_slice(include_bytes!("../tests/base/main.min.js.map"))?;
        let r = source_map_link(&script_coverage[0].source, &source_map).await?;
        // tokio::fs::write(
        //     "tests/base/source_map_link.json",
        //     serde_json::to_string_pretty(&r)?,
        // )
        // .await?;
        let expect: Vec<MappingItem> =
            serde_json::from_str(include_str!("../tests/base/source_map_link.json"))?;
        assert_json_eq!(r, expect);

        Ok(())
    }
    #[tokio::test]
    async fn test_source_map_link_jsx() -> Result<()> {
        let script_coverage = serde_json::from_str::<
            Vec<crate::format::script_coverage::ScriptCoverage>,
        >(include_str!("../tests/jsx/v8-coverage.json"))
        .map_err(|e| anyhow!("parse script coverage error: {}", e))?;
        let source_map =
            SourceMap::from_slice(include_bytes!("../tests/jsx/main.f272a57c.chunk.js.map"))?;
        let r = source_map_link(&script_coverage[0].source, &source_map).await?;

        // tokio::fs::write(
        //     "tests/jsx/source_map_link.json",
        //     serde_json::to_string_pretty(&r)?,
        // )
        // .await?;
        let expect: Vec<MappingItem> =
            serde_json::from_str(include_str!("../tests/jsx/source_map_link.json"))?;
        assert_json_eq!(r, expect);

        Ok(())
    }

    #[test]
    fn test_str_len() {
        let s = "1234567890";
        assert_eq!(s.len(), 10);
        let s = "中文字符";
        assert_eq!(s.chars().count(), 4);
        let s = "中文字符";
        assert_ne!(s.len(), 4);
    }

    #[test]
    fn test_ext() {
        assert!(is_file_extension_allowed(
            "/home/user/example.txt",
            &["txt"]
        ));
        assert!(!is_file_extension_allowed(
            "/home/user/example.txt",
            &["js"]
        ));
    }
}
