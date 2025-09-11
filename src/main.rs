

use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand };
use nu_protocol::{ IntoPipelineData, LabeledError, PipelineData, Record, Signature, SyntaxShape, Type, Value};

use globset::{Glob, GlobSetBuilder};




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
        .required("path", SyntaxShape::List(SyntaxShape::String.into()), "the path to the file")
            .input_output_type(Type::List(Type::Any.into()), Type::List(Type::Int.into()))
    }

    fn run(
        &self,
        _plugin: &GlobSetPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        // 从 input 提取 glob patterns
        let patterns: Vec<String> = input
            .into_iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();

        // 从 call 获取目标路径 (第一个参数)
        let targets: Vec<String> = call.req(0)?;

        // 构造 globset
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

        targets.iter().for_each(|target| {
            let result_matches =  set.matches(target.as_str());
            let mut rc  = Record::new();
            rc.push("matches", Value::List { vals: result_matches.iter().map(|i| Value::Int { val: *i as i64, internal_span: call.head }).collect(), internal_span: call.head });
            rc.push("is_match", Value::Bool { val: !result_matches.is_empty(), internal_span: call.head });
            rc.push("file_path", Value::String { val: target.clone(), internal_span: call.head });
            // let cv = CustomValue::new(Box::new(rc), "globset
            result.push(Value::Record { val: rc.into(), internal_span: call.head  });
            // result_matches.iter().for_each(|_i| {
            //     result.push(Value::List {
            //         val: result_matches,
            //         internal_span: call.head,
            //     });
            // });
          }
        );
        

        // // 逐个测试，收集匹配到的 index
        // let mut result = Vec::new();
        // for (i, pat) in patterns.iter().enumerate() {
        //     let g = Glob::new(pat)
        //         .map_err(|e| LabeledError::new(format!("Invalid glob: {}", e)))?
        //         .compile_matcher();
        //     if g.is_match(&target) {
        //         result.push(Value::Int {
        //             val: i as i64,
        //             internal_span: call.head,
        //         });
        //     }
        // }

        // 返回 list<int>
        Ok(Value::List {
            vals: result,
            internal_span: call.head,
        }
        .into_pipeline_data())
      }
}


// use nu_plugin_test_support::PluginTest;

// #[test]
// fn test_len_matches() -> Result<(), _> {
//     let mut test = PluginTest::new("globset", GlobSetPlugin.into())?;

//     // 执行命令
//     let result = test.eval(r#"["*.rs", "src/**/*.rs"] | globset "test.rs""#)?;
//     println!("======={:?}", result);
//     let vals: Vec<Value> = result.into_iter().collect();
//     let ints: Vec<i64> = vals.into_iter().filter_map(|v| match v {
//         Value::Int { val, .. } => Some(val),
//         _ => None,
//     }).collect();

//     println!("匹配到的索引: {:?}", ints);

//     // 断言


//     Ok(())
// }

fn main() {
    serve_plugin(&GlobSetPlugin, JsonSerializer)
}