use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand };
use nu_protocol::{ IntoPipelineData, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};
use globset::{Glob, GlobSetBuilder};
use std::fs::File;
use std::io::{BufRead, BufReader};
use uuid::Uuid;
use serde::Serialize;

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
        let patterns: Vec<String> = input
            .into_iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();

        let input_path: String = call.req(0)?;

        let mut builder = GlobSetBuilder::new();
        for pat in &patterns {
            builder.add(Glob::new(pat).map_err(|e| LabeledError::new(format!("Invalid glob: {}", e)))?);
        }
        let set = builder.build().map_err(|e| LabeledError::new(format!("Glob build error: {}", e)))?;

        let input_file =
