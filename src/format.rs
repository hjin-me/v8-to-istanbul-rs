use serde::{Deserialize, Serialize};

pub mod istanbul;
pub mod script_coverage;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingItem<'a> {
    #[serde(rename = "s")]
    pub source: &'a str,
    #[serde(rename = "gs")]
    pub generated_column: u32,
    #[serde(rename = "ge")]
    pub last_generated_column: u32,
    #[serde(rename = "osl")]
    pub original_line: u32,
    #[serde(rename = "osc")]
    pub original_column: u32,
    #[serde(rename = "oel")]
    pub last_original_line: u32,
    #[serde(rename = "oec")]
    pub last_original_column: u32,
    #[serde(rename = "c")]
    pub count: u32,
    pub idx: usize,
}

// pub fn source_map_key(source_map: &SourceMap) -> String {
//     let input_string = source_map
//         .sources()
//         .map(|source| source.to_string())
//         .collect::<Vec<String>>()
//         .join(",");
//
//     // Create a Sha1 object
//     let mut hasher = Sha1::new();
//
//     // Write the input data to the hasher
//     hasher.update(input_string);
//
//     // Finalize the hash and obtain the result as a byte array
//     let result = hasher.finalize();
//
//     // Convert the result to a hexadecimal string
//     hex::encode(result)
// }
