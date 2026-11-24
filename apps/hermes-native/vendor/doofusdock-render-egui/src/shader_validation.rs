//! Bounded, headless WGSL/GLSL parsing, validation, and SPIR-V generation.
//!
//! SPIR-V output is offline code generation. This module does not compile a
//! native GPU pipeline, execute shader code, or change capability reports.

use naga::{ShaderStage, valid::Capabilities};

pub const MAX_WGSL_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_GLSL_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_WGSL_ENTRY_POINTS: usize = 32;
pub const MAX_SPIRV_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WgslStage {
    Vertex,
    Fragment,
    Compute,
    Task,
    Mesh,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgslEntryPoint {
    pub name: String,
    pub stage: WgslStage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedWgsl {
    pub source_bytes: usize,
    pub entry_points: Vec<WgslEntryPoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledSpirvEntryPoint {
    pub name: String,
    pub stage: WgslStage,
    pub words: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlslStage {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedGlsl {
    pub source_bytes: usize,
    pub stage: GlslStage,
    pub entry_point: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlslValidationError {
    SourceTooLarge { actual_bytes: usize, limit: usize },
    Parse(String),
    InvalidModule(String),
    Codegen(String),
    SpirvOutputTooLarge { actual_bytes: usize, limit: usize },
}

impl std::fmt::Display for GlslValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceTooLarge {
                actual_bytes,
                limit,
            } => write!(
                formatter,
                "GLSL source is {actual_bytes} bytes; the limit is {limit} bytes."
            ),
            Self::Parse(message) => write!(formatter, "GLSL parse failed: {message}"),
            Self::InvalidModule(message) => write!(formatter, "GLSL validation failed: {message}"),
            Self::Codegen(message) => write!(formatter, "GLSL SPIR-V generation failed: {message}"),
            Self::SpirvOutputTooLarge {
                actual_bytes,
                limit,
            } => write!(
                formatter,
                "Generated SPIR-V is {actual_bytes} bytes; the limit is {limit} bytes."
            ),
        }
    }
}

impl std::error::Error for GlslValidationError {}

/// Parse and validate bounded GLSL source for a caller-selected pipeline
/// stage. This does not create a host GPU pipeline or execute the shader.
pub fn validate_glsl(source: &str, stage: GlslStage) -> Result<ValidatedGlsl, GlslValidationError> {
    let (_, _, _) = parse_and_validate_glsl(source, stage)?;
    Ok(ValidatedGlsl {
        source_bytes: source.len(),
        stage,
        entry_point: "main".to_owned(),
    })
}

/// Generate an offline SPIR-V module from bounded, validated GLSL source.
/// This does not negotiate a GPU adapter, create a pipeline, or execute code.
pub fn compile_glsl_to_spirv(
    source: &str,
    stage: GlslStage,
) -> Result<CompiledSpirvEntryPoint, GlslValidationError> {
    let (module, info, naga_stage) = parse_and_validate_glsl(source, stage)?;
    let pipeline = naga::back::spv::PipelineOptions {
        shader_stage: naga_stage,
        entry_point: "main".to_owned(),
    };
    let words = naga::back::spv::write_vec(
        &module,
        &info,
        &naga::back::spv::Options::default(),
        Some(&pipeline),
    )
    .map_err(|error| GlslValidationError::Codegen(error.to_string()))?;
    let actual_bytes = words.len().saturating_mul(std::mem::size_of::<u32>());
    if actual_bytes > MAX_SPIRV_OUTPUT_BYTES {
        return Err(GlslValidationError::SpirvOutputTooLarge {
            actual_bytes,
            limit: MAX_SPIRV_OUTPUT_BYTES,
        });
    }
    Ok(CompiledSpirvEntryPoint {
        name: "main".to_owned(),
        stage: from_naga_stage(naga_stage),
        words,
    })
}

fn parse_and_validate_glsl(
    source: &str,
    stage: GlslStage,
) -> Result<(naga::Module, naga::valid::ModuleInfo, ShaderStage), GlslValidationError> {
    if source.len() > MAX_GLSL_SOURCE_BYTES {
        return Err(GlslValidationError::SourceTooLarge {
            actual_bytes: source.len(),
            limit: MAX_GLSL_SOURCE_BYTES,
        });
    }
    let naga_stage = to_naga_stage(stage);
    let mut frontend = naga::front::glsl::Frontend::default();
    let module = frontend
        .parse(&naga::front::glsl::Options::from(naga_stage), source)
        .map_err(|error| GlslValidationError::Parse(error.to_string()))?;
    let info =
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), Capabilities::empty())
            .validate(&module)
            .map_err(|error| GlslValidationError::InvalidModule(error.to_string()))?;
    Ok((module, info, naga_stage))
}

fn to_naga_stage(stage: GlslStage) -> ShaderStage {
    match stage {
        GlslStage::Vertex => ShaderStage::Vertex,
        GlslStage::Fragment => ShaderStage::Fragment,
        GlslStage::Compute => ShaderStage::Compute,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WgslValidationError {
    SourceTooLarge { actual_bytes: usize, limit: usize },
    Parse(String),
    InvalidModule(String),
    TooManyEntryPoints { actual: usize, limit: usize },
    EntryPointNotFound(String),
    Codegen(String),
    SpirvOutputTooLarge { actual_bytes: usize, limit: usize },
}

impl std::fmt::Display for WgslValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceTooLarge {
                actual_bytes,
                limit,
            } => write!(
                formatter,
                "WGSL source is {actual_bytes} bytes; the limit is {limit} bytes."
            ),
            Self::Parse(message) => write!(formatter, "WGSL parse failed: {message}"),
            Self::InvalidModule(message) => write!(formatter, "WGSL validation failed: {message}"),
            Self::TooManyEntryPoints { actual, limit } => write!(
                formatter,
                "WGSL module has {actual} entry points; the limit is {limit}."
            ),
            Self::EntryPointNotFound(name) => {
                write!(formatter, "WGSL entry point was not found: {name}")
            }
            Self::Codegen(message) => write!(formatter, "SPIR-V generation failed: {message}"),
            Self::SpirvOutputTooLarge {
                actual_bytes,
                limit,
            } => write!(
                formatter,
                "Generated SPIR-V is {actual_bytes} bytes; the limit is {limit} bytes."
            ),
        }
    }
}

impl std::error::Error for WgslValidationError {}

/// Parse and validate bounded WGSL with Naga's complete validation flags and
/// no optional device capabilities enabled.
pub fn validate_wgsl(source: &str) -> Result<ValidatedWgsl, WgslValidationError> {
    let (module, _) = parse_and_validate_wgsl(source)?;
    let entry_points = module
        .entry_points
        .iter()
        .map(|entry| WgslEntryPoint {
            name: entry.name.clone(),
            stage: from_naga_stage(entry.stage),
        })
        .collect();
    Ok(ValidatedWgsl {
        source_bytes: source.len(),
        entry_points,
    })
}

/// Generate a bounded SPIR-V module for one selected WGSL entry point.
///
/// This is offline backend code generation. It does not create a GPU device,
/// negotiate a host adapter, create a pipeline, or execute the module.
pub fn compile_wgsl_entry_point_to_spirv(
    source: &str,
    entry_point: &str,
) -> Result<CompiledSpirvEntryPoint, WgslValidationError> {
    let (module, info) = parse_and_validate_wgsl(source)?;
    let Some(selected) = module
        .entry_points
        .iter()
        .find(|candidate| candidate.name == entry_point)
    else {
        return Err(WgslValidationError::EntryPointNotFound(
            entry_point.to_owned(),
        ));
    };
    let stage = selected.stage;
    let pipeline = naga::back::spv::PipelineOptions {
        shader_stage: stage,
        entry_point: entry_point.to_owned(),
    };
    let words = naga::back::spv::write_vec(
        &module,
        &info,
        &naga::back::spv::Options::default(),
        Some(&pipeline),
    )
    .map_err(|error| WgslValidationError::Codegen(error.to_string()))?;
    let actual_bytes = words.len().saturating_mul(std::mem::size_of::<u32>());
    if actual_bytes > MAX_SPIRV_OUTPUT_BYTES {
        return Err(WgslValidationError::SpirvOutputTooLarge {
            actual_bytes,
            limit: MAX_SPIRV_OUTPUT_BYTES,
        });
    }
    Ok(CompiledSpirvEntryPoint {
        name: entry_point.to_owned(),
        stage: from_naga_stage(stage),
        words,
    })
}

fn parse_and_validate_wgsl(
    source: &str,
) -> Result<(naga::Module, naga::valid::ModuleInfo), WgslValidationError> {
    if source.len() > MAX_WGSL_SOURCE_BYTES {
        return Err(WgslValidationError::SourceTooLarge {
            actual_bytes: source.len(),
            limit: MAX_WGSL_SOURCE_BYTES,
        });
    }
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|error| WgslValidationError::Parse(error.emit_to_string(source)))?;
    if module.entry_points.len() > MAX_WGSL_ENTRY_POINTS {
        return Err(WgslValidationError::TooManyEntryPoints {
            actual: module.entry_points.len(),
            limit: MAX_WGSL_ENTRY_POINTS,
        });
    }
    let info =
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), Capabilities::empty())
            .validate(&module)
            .map_err(|error| WgslValidationError::InvalidModule(error.to_string()))?;
    Ok((module, info))
}

fn from_naga_stage(stage: ShaderStage) -> WgslStage {
    match stage {
        ShaderStage::Vertex => WgslStage::Vertex,
        ShaderStage::Fragment => WgslStage::Fragment,
        ShaderStage::Compute => WgslStage::Compute,
        ShaderStage::Task => WgslStage::Task,
        ShaderStage::Mesh => WgslStage::Mesh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
        @vertex
        fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
            let x = f32(i32(index) - 1);
            return vec4<f32>(x, 0.0, 0.0, 1.0);
        }

        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(0.2, 0.4, 0.8, 1.0);
        }
    "#;

    #[test]
    fn glsl_frontend_parses_and_validates_stage_specific_sources() {
        let vertex = "#version 450 core\nvoid main() { gl_Position = vec4(0.0); }";
        let result = validate_glsl(vertex, GlslStage::Vertex).unwrap();
        assert_eq!(result.source_bytes, vertex.len());
        assert_eq!(result.stage, GlslStage::Vertex);
        assert_eq!(result.entry_point, "main");

        let fragment = "#version 450 core\nlayout(location=0) out vec4 color; void main() { color = vec4(1.0); }";
        assert_eq!(
            validate_glsl(fragment, GlslStage::Fragment).unwrap().stage,
            GlslStage::Fragment
        );
        let compiled = compile_glsl_to_spirv(vertex, GlslStage::Vertex).unwrap();
        assert_eq!(compiled.name, "main");
        assert_eq!(compiled.stage, WgslStage::Vertex);
        assert_eq!(compiled.words.first(), Some(&0x0723_0203));
    }

    #[test]
    fn glsl_frontend_rejects_invalid_and_over_limit_sources() {
        assert!(matches!(
            validate_glsl("#version 450 core\nvoid main( {", GlslStage::Vertex),
            Err(GlslValidationError::Parse(_))
        ));
        let oversized = " ".repeat(MAX_GLSL_SOURCE_BYTES + 1);
        assert_eq!(
            validate_glsl(&oversized, GlslStage::Vertex),
            Err(GlslValidationError::SourceTooLarge {
                actual_bytes: MAX_GLSL_SOURCE_BYTES + 1,
                limit: MAX_GLSL_SOURCE_BYTES,
            })
        );
    }

    #[test]
    fn valid_wgsl_reports_entry_points_without_claiming_execution() {
        let validated = validate_wgsl(VALID).unwrap();
        assert_eq!(validated.source_bytes, VALID.len());
        assert_eq!(validated.entry_points.len(), 2);
        assert_eq!(validated.entry_points[0].name, "vs_main");
        assert_eq!(validated.entry_points[0].stage, WgslStage::Vertex);
        assert_eq!(validated.entry_points[1].name, "fs_main");
        assert_eq!(validated.entry_points[1].stage, WgslStage::Fragment);
    }

    #[test]
    fn syntax_and_semantic_errors_are_reported() {
        assert!(matches!(
            validate_wgsl("@vertex fn bad( {"),
            Err(WgslValidationError::Parse(_))
        ));
        let invalid_module = "@compute @workgroup_size(0) fn main() {}";
        assert!(matches!(
            validate_wgsl(invalid_module),
            Err(WgslValidationError::InvalidModule(_))
        ));
    }

    #[test]
    fn source_size_is_rejected_before_parsing() {
        let source = " ".repeat(MAX_WGSL_SOURCE_BYTES + 1);
        assert_eq!(
            validate_wgsl(&source),
            Err(WgslValidationError::SourceTooLarge {
                actual_bytes: MAX_WGSL_SOURCE_BYTES + 1,
                limit: MAX_WGSL_SOURCE_BYTES,
            })
        );
    }

    #[test]
    fn entry_point_metadata_is_bounded() {
        let mut source = String::new();
        for index in 0..=MAX_WGSL_ENTRY_POINTS {
            source.push_str(&format!(
                "@compute @workgroup_size(1) fn main_{index}() {{}}\n"
            ));
        }
        assert_eq!(
            validate_wgsl(&source),
            Err(WgslValidationError::TooManyEntryPoints {
                actual: MAX_WGSL_ENTRY_POINTS + 1,
                limit: MAX_WGSL_ENTRY_POINTS,
            })
        );
    }

    #[test]
    fn valid_entry_point_generates_bounded_spirv_without_a_device() {
        let compiled = compile_wgsl_entry_point_to_spirv(VALID, "vs_main").unwrap();
        assert_eq!(compiled.name, "vs_main");
        assert_eq!(compiled.stage, WgslStage::Vertex);
        assert_eq!(compiled.words.first(), Some(&0x0723_0203));
        assert!(compiled.words.len() * std::mem::size_of::<u32>() <= MAX_SPIRV_OUTPUT_BYTES);
    }

    #[test]
    fn missing_entry_point_is_reported_before_codegen() {
        assert_eq!(
            compile_wgsl_entry_point_to_spirv(VALID, "missing"),
            Err(WgslValidationError::EntryPointNotFound("missing".into()))
        );
    }
}
