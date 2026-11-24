//! Optional bounded HLSL-to-SPIR-V compilation through shaderc.
//!
//! The compiler is built from the pinned shaderc source dependency. This uses
//! shaderc's legacy HLSL frontend; callers should prefer WGSL or GLSL for new
//! shaders. Includes are rejected, and the source/output sizes are bounded.
//! Compilation runs in process and is not an adversarial-code sandbox.

pub const MAX_HLSL_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_HLSL_SPIRV_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HlslStage {
    Vertex,
    Fragment,
    Compute,
}

impl HlslStage {
    fn shaderc_kind(self) -> shaderc::ShaderKind {
        match self {
            Self::Vertex => shaderc::ShaderKind::Vertex,
            Self::Fragment => shaderc::ShaderKind::Fragment,
            Self::Compute => shaderc::ShaderKind::Compute,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HlslCompileError {
    SourceTooLarge { actual: usize, maximum: usize },
    SourceContainsNul,
    InvalidEntryPoint,
    Compiler(String),
    IncludeRejected(String),
    Compilation(String),
    OutputTooLarge { actual: usize, maximum: usize },
    InvalidSpirv(String),
}

impl std::fmt::Display for HlslCompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "HLSL source is {actual} bytes; maximum is {maximum}."
                )
            }
            Self::SourceContainsNul => write!(formatter, "HLSL source contains a NUL byte."),
            Self::InvalidEntryPoint => write!(
                formatter,
                "HLSL entry point must be a bounded ASCII identifier."
            ),
            Self::Compiler(error) => write!(formatter, "Could not initialize shaderc: {error}"),
            Self::IncludeRejected(name) => {
                write!(formatter, "HLSL include is not allowed: {name}")
            }
            Self::Compilation(error) => write!(formatter, "HLSL compilation failed: {error}"),
            Self::OutputTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "HLSL output is {actual} bytes; maximum is {maximum}."
                )
            }
            Self::InvalidSpirv(error) => write!(formatter, "Invalid generated SPIR-V: {error}"),
        }
    }
}

impl std::error::Error for HlslCompileError {}

/// Compile bounded HLSL to Vulkan 1.1 SPIR-V with includes disabled.
pub fn compile_hlsl_to_spirv(
    source: &str,
    stage: HlslStage,
    entry_point: &str,
) -> Result<Vec<u32>, HlslCompileError> {
    if source.len() > MAX_HLSL_SOURCE_BYTES {
        return Err(HlslCompileError::SourceTooLarge {
            actual: source.len(),
            maximum: MAX_HLSL_SOURCE_BYTES,
        });
    }
    if source.as_bytes().contains(&0) {
        return Err(HlslCompileError::SourceContainsNul);
    }
    if !valid_entry_point(entry_point) {
        return Err(HlslCompileError::InvalidEntryPoint);
    }

    let compiler =
        shaderc::Compiler::new().map_err(|error| HlslCompileError::Compiler(error.to_string()))?;
    let mut options = shaderc::CompileOptions::new()
        .map_err(|error| HlslCompileError::Compiler(error.to_string()))?;
    options.set_source_language(shaderc::SourceLanguage::HLSL);
    options.set_target_env(
        shaderc::TargetEnv::Vulkan,
        shaderc::EnvVersion::Vulkan1_1 as u32,
    );
    options.set_target_spirv(shaderc::SpirvVersion::V1_3);
    // The GOOGLE extension is not accepted by wgpu's SPIR-V frontend.
    options.set_hlsl_functionality1(false);
    options.set_include_callback(|name, _, _, _| Err(format!("{name} (includes are disabled)")));

    let artifact = compiler
        .compile_into_spirv(
            source,
            stage.shaderc_kind(),
            "source.hlsl",
            entry_point,
            Some(&options),
        )
        .map_err(|error| {
            let message = error.to_string();
            if message.contains("includes are disabled") {
                HlslCompileError::IncludeRejected(message)
            } else {
                HlslCompileError::Compilation(message)
            }
        })?;
    let words = artifact.as_binary();
    let output_bytes = words.len().saturating_mul(std::mem::size_of::<u32>());
    if output_bytes > MAX_HLSL_SPIRV_BYTES {
        return Err(HlslCompileError::OutputTooLarge {
            actual: output_bytes,
            maximum: MAX_HLSL_SPIRV_BYTES,
        });
    }
    if words.first() != Some(&0x0723_0203) {
        return Err(HlslCompileError::InvalidSpirv(
            "missing SPIR-V magic header".into(),
        ));
    }
    Ok(words.to_vec())
}

fn valid_entry_point(entry_point: &str) -> bool {
    if entry_point.is_empty() || entry_point.len() > 128 || !entry_point.is_ascii() {
        return false;
    }
    let mut characters = entry_point.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERTEX_HLSL: &str = r#"
struct Output { float4 position : SV_Position; };
Output vs_main(uint vertex_id : SV_VertexID) {
    float2 positions[3] = { float2(-1.0, -1.0), float2(3.0, -1.0), float2(-1.0, 3.0) };
    Output output;
    output.position = float4(positions[vertex_id], 0.0, 1.0);
    return output;
}
"#;

    #[test]
    fn hlsl_compiles_to_bounded_spirv() {
        let words = compile_hlsl_to_spirv(VERTEX_HLSL, HlslStage::Vertex, "vs_main")
            .unwrap_or_else(|error| panic!("shaderc HLSL compilation failed: {error}"));
        assert_eq!(words.first(), Some(&0x0723_0203));
        assert!(words.len() * 4 <= MAX_HLSL_SPIRV_BYTES);
    }

    #[test]
    fn malformed_hlsl_and_includes_fail_closed() {
        assert!(matches!(
            compile_hlsl_to_spirv("#include \"local.hlsl\"", HlslStage::Vertex, "main"),
            Err(HlslCompileError::IncludeRejected(_))
        ));
        assert!(matches!(
            compile_hlsl_to_spirv("not valid shader source", HlslStage::Vertex, "main"),
            Err(HlslCompileError::Compilation(_))
        ));
    }

    #[test]
    fn source_and_entry_point_bounds_are_checked_before_compilation() {
        assert!(matches!(
            compile_hlsl_to_spirv(
                &"x".repeat(MAX_HLSL_SOURCE_BYTES + 1),
                HlslStage::Vertex,
                "main"
            ),
            Err(HlslCompileError::SourceTooLarge { .. })
        ));
        assert!(matches!(
            compile_hlsl_to_spirv(VERTEX_HLSL, HlslStage::Vertex, "-E"),
            Err(HlslCompileError::InvalidEntryPoint)
        ));
        assert!(matches!(
            compile_hlsl_to_spirv("bad\0source", HlslStage::Vertex, "main"),
            Err(HlslCompileError::SourceContainsNul)
        ));
    }
}
