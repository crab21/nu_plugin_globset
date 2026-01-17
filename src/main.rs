use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand };
use nu_protocol::{ IntoPipelineData, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};
use globset::{Glob, GlobSetBuilder};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use uuid::Uuid;
use serde::Serialize; // 引入序列化 trait

// 定义输出结构，方便直接转 JSON
#[derive(Serialize)]
struct OutputRecord {
    matches: Vec<usize>,
    is_match: bool,
    file_path: String, // 对应原来的 file_path 字段 (实际是行内容)
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
        "Matches lines from a file and saves results to a temporary JSONL file"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .required("path", SyntaxShape::Filepath, "the path to the file")
            // 输入是 List<String> (glob patterns)
            // 输出改为了 String (生成的文件路径)
            .input_output_type(Type::List(Type::Any.into()), Type::String)
    }

    fn run(
        &self,
        _plugin: &GlobSetPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        // 1. 获取 Patterns
        let patterns: Vec<String> = input
            .into_iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();

        // 2. 获取输入文件路径
        let input_path: String = call.req(0)?;

        // 3. 准备输出文件路径 (/tmp/uuid.jsonl)
        let mut temp_file_path = std::env::temp_dir();
        let file_uuid = Uuid::new_v4().to_string();
        temp_file_path.push(format!("{}.json", file_uuid)); // 使用 .jsonl 后缀表示 JSON Lines

        // 4. 打开输入文件
        let input_file = File::open(&input_path).map_err(|e| {
            LabeledError::new(format!("无法打开输入文件 '{}': {}", input_path, e))
                .with_label("文件访问错误", call.head)
        })?;

        // 5. 创建输出文件
        let output_file = File::create(&temp_file_path).map_err(|e| {
            LabeledError::new(format!("无法创建临时文件 '{:?}': {}", temp_file_path, e))
                .with_label("IO 错误", call.head)
        })?;

        // 6. 构造 GlobSet
        let mut builder = GlobSetBuilder::new();
        for pat in &patterns {
            builder.add(Glob::new(pat).map_err(|e| {
                LabeledError::new(format!("无效的 Glob 模式: {}", e))
            })?);
        }
        let set = builder.build().map_err(|e| {
            LabeledError::new(format!("GlobSet 构建失败: {}", e))
        })?;

        // 7. 处理并写入
        // 使用 BufWriter 提高写入性能
        let mut writer = BufWriter::new(output_file);
        let reader = BufReader::new(input_file);

        for line_res in reader.lines() {
            let line_content = line_res.unwrap_or_default();
            
            let result_matches = set.matches(&line_content);
            let is_match = !result_matches.is_empty();

            // 构造结构体
            let record = OutputRecord {
                matches: result_matches,
                is_match,
                file_path: line_content, 
            };

            // 序列化为 JSON 并写入文件，每一行一个 JSON 对象
            // 这种格式叫 JSON Lines (ndjson)，非常适合大数据处理
            serde_json::to_writer(&mut writer, &record).map_err(|e| {
                LabeledError::new(format!("JSON 序列化失败: {}", e))
            })?;
            
            // 写入换行符
            writeln!(writer).map_err(|e| {
                LabeledError::new(format!("写入文件失败: {}", e))
            })?;
        }
        
        // 确保所有缓冲区数据都写入磁盘
        writer.flush().map_err(|e| LabeledError::new(format!("Flush 失败: {}", e)))?;

        // 8. 返回生成的文件路径字符串
        // 将 PathBuf 转为 String
        let output_path_str = temp_file_path.to_string_lossy().to_string();

        Ok(Value::String {
            val: output_path_str,
            internal_span: call.head,
        }
        .into_pipeline_data())
    }
}

fn main() {
    serve_plugin(&GlobSetPlugin, JsonSerializer)
}
