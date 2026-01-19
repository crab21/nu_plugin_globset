use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand };
use nu_protocol::{ IntoPipelineData, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};
use globset::{Glob, GlobSetBuilder};
use std::fs::File;
use std::io::{BufRead, BufReader};
use uuid::Uuid;
use serde::Serialize;
use serde_json::from_reader;


#[derive(Serialize)]
struct ResultRecord {
    matches: Vec<usize>,
    is_match: bool,
    file_path: String,
}

struct GlobSetPlugin;

impl Plugin for GlobSetPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(GlobSet),
        ]
    }
}

struct GlobSet;

impl PluginCommand for GlobSet {
    type Plugin = GlobSetPlugin;

    fn name(&self) -> &str {
        "globset"
    }

    fn description(&self) -> &str {
        "Calculates matches and saves the result array to a JSON file"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .required("path", SyntaxShape::Filepath, "the path to the file")
            .input_output_type(Type::List(Type::Any.into()), Type::String)
    }

    fn run(
        &self,
        _plugin: &GlobSetPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        // 1. 获取 Glob Patterns
        let patterns: Vec<String> = input
            .into_iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();

        // 2. 获取输入文件路径
        let input_path: String = call.req(0)?;

        // 3. 构造 GlobSet
        let mut builder = GlobSetBuilder::new();
        for pat in &patterns {
            builder.add(Glob::new(pat).map_err(|e| LabeledError::new(format!("Invalid glob: {}", e)))?);
        }
        let set = builder.build().map_err(|e| LabeledError::new(format!("Glob build error: {}", e)))?;

        // 4. 打开文件准备读取
        let input_file = File::open(&input_path).map_err(|e| {
            LabeledError::new(format!("Open error: {}", e)).with_label("Error", call.head)
        })?;
        let reader = BufReader::new(input_file);

        // 2. 解析 JSON 数组
        let targets: Vec<String> = from_reader(reader)
            .map_err(|e| LabeledError::new(format!("JSON parse error: {}", e)))?;


        // 5. 在内存中收集结果数组
        let mut results_array = Vec::new();

        for line_res in targets {
            let target = line_res;
            let result_matches = set.matches(target.as_str())
                .into_iter()
                .collect::<Vec<usize>>();
            
            // [修复点] 先计算 bool 值，避免 move 后借用错误
            let is_match = !result_matches.is_empty();

            results_array.push(ResultRecord {
                matches: result_matches, // 此时 move
                is_match,                // 使用计算好的值
                file_path: target,
            });
        }

        // 6. 准备输出文件路径
        let mut temp_file_path = std::env::temp_dir();
        let file_uuid = Uuid::new_v4().to_string();
        temp_file_path.push(format!("{}.json", file_uuid));

        let output_file = File::create(&temp_file_path).map_err(|e| {
            LabeledError::new(format!("Create error: {}", e)).with_label("Error", call.head)
        })?;

        // 7. 将数组一次性序列化写入文件
        serde_json::to_writer(output_file, &results_array).map_err(|e| {
            LabeledError::new(format!("Serialization error: {}", e))
        })?;

        // 8. 返回文件路径字符串
        Ok(Value::String {
            val: temp_file_path.to_string_lossy().to_string(),
            internal_span: call.head,
        }
        .into_pipeline_data())
    }
}

fn main() {
    serve_plugin(&GlobSetPlugin, JsonSerializer)
}
