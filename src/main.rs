use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand };
use nu_protocol::{ IntoPipelineData, LabeledError, PipelineData, Record, Signature, SyntaxShape, Type, Value};
use globset::{Glob, GlobSetBuilder};
use std::fs::File;
use std::io::{BufRead, BufReader}; // 引入 IO 库

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
        "calculates the length of its input"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            // 修改点 1: 参数类型改为 Filepath，名称保持 "path" 不变
            // 这样 Nushell 会自动处理路径补全，传入的参数现在是单一的文件路径字符串
            .required("path", SyntaxShape::Filepath, "the path to the file") 
            .input_output_type(Type::List(Type::Any.into()), Type::List(Type::Int.into()))
    }

    fn run(
        &self,
        _plugin: &GlobSetPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        // 从 input 提取 glob patterns (保持不变)
        let patterns: Vec<String> = input
            .into_iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();

        // 修改点 2: 获取文件路径 (字符串)，而不是之前的 Vec
        let path: String = call.req(0)?;

        // 修改点 3: 读取文件内容
        // 这里处理了 "文件不存在" (Err) 和 "文件为空" (Ok 但 lines 为空)
        let targets: Vec<String> = match File::open(&path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                // 逐行读取，忽略读取错误的行(或根据需求处理)
                reader.lines()
                    .map(|line| line.unwrap_or_default())
                    .collect()
            },
            Err(e) => {
                // 如果文件不存在或无法读取，返回带标签的错误
                return Err(LabeledError::new(format!("无法打开文件 '{}': {}", path, e))
                    .with_label("文件访问错误", call.head));
            }
        };

        // 构造 globset (保持不变)
        let mut builder = GlobSetBuilder::new();
        for pat in &patterns {
            builder
                .add(Glob::new(pat).map_err(|e| {
                    LabeledError::new(format!("Invalid glob: {}", e))
                })?);
        }
        let set = builder.build().map_err(|e| {
            LabeledError::new(format!("Glob build error: {}", e))
        })?;


        let mut result = Vec::new();

        // 这里的 targets 现在是从文件读取出来的每一行
        // 如果文件为空，targets 为空，循环不会执行，result 为空，符合预期
        targets.iter().for_each(|target| {
            let result_matches =  set.matches(target.as_str());
            let mut rc  = Record::new();
            rc.push("matches", Value::List { vals: result_matches.iter().map(|i| Value::Int { val: *i as i64, internal_span: call.head }).collect(), internal_span: call.head });
            rc.push("is_match", Value::Bool { val: !result_matches.is_empty(), internal_span: call.head });
            // 保持原本的 key 名字 "file_path"，但现在 value 是文件里的一行内容
            rc.push("file_path", Value::String { val: target.clone(), internal_span: call.head });
            
            result.push(Value::Record { val: rc.into(), internal_span: call.head  });
          }
        );
        

        // 返回 list
        Ok(Value::List {
            vals: result,
            internal_span: call.head,
        }
        .into_pipeline_data())
      }
}


fn main() {
    serve_plugin(&GlobSetPlugin, JsonSerializer)
}
