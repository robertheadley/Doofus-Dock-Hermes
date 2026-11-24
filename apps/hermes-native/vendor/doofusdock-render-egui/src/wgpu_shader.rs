//! Optional offscreen WGPU fullscreen effects for hosts that own a WGPU device.
//!
//! The existing egui glow and CPU-software adapters do not use this module.
//! Hosts opt into the `wgpu-shaders` feature, request an adapter, and submit
//! effects against caller-owned source/target textures. Core layout state is
//! not reachable from this backend.

use crate::{Capabilities, shader_validation};
use shader_validation::{GlslStage, WgslStage};
use std::borrow::Cow;

const BUILTIN_FULLSCREEN_VERTEX_WGSL: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[index], 0.0, 1.0);
    return out;
}
"#;

/// Small, source-embedded fullscreen effects for hosts that want a safe
/// starting palette without loading or compiling theme-provided shader text.
/// These are optional GPU effects and do not enable shader-driven chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WgpuBuiltinEffect {
    /// Convert RGB to relative-luminance grayscale while preserving alpha.
    Grayscale,
    /// Apply a fixed sepia color grade while preserving alpha.
    Sepia,
    /// Darken alternating target scanlines by 50%, preserving alpha.
    Scanlines,
    /// Sample the center of four-source-texel blocks for a pixelated image.
    /// For stable block geometry, source and target dimensions should match.
    Pixelate,
}

impl WgpuBuiltinEffect {
    fn fragment_source(self) -> &'static str {
        match self {
            Self::Grayscale => GRAYSCALE_FRAGMENT_WGSL,
            Self::Sepia => SEPIA_FRAGMENT_WGSL,
            Self::Scanlines => SCANLINES_FRAGMENT_WGSL,
            Self::Pixelate => PIXELATE_FRAGMENT_WGSL,
        }
    }
}

const GRAYSCALE_FRAGMENT_WGSL: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(source_texture));
    let texel = clamp(vec2<i32>(position.xy), vec2<i32>(0), dimensions - vec2<i32>(1));
    let color = textureLoad(source_texture, texel, 0);
    let luminance = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    return vec4<f32>(vec3<f32>(luminance), color.a);
}
"#;

const SEPIA_FRAGMENT_WGSL: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(source_texture));
    let texel = clamp(vec2<i32>(position.xy), vec2<i32>(0), dimensions - vec2<i32>(1));
    let color = textureLoad(source_texture, texel, 0);
    let sepia = vec3<f32>(
        dot(color.rgb, vec3<f32>(0.393, 0.769, 0.189)),
        dot(color.rgb, vec3<f32>(0.349, 0.686, 0.168)),
        dot(color.rgb, vec3<f32>(0.272, 0.534, 0.131)),
    );
    return vec4<f32>(min(sepia, vec3<f32>(1.0)), color.a);
}
"#;

const SCANLINES_FRAGMENT_WGSL: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs_main(
    @builtin(position) position: vec4<f32>,
) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(source_texture));
    let texel = clamp(vec2<i32>(position.xy), vec2<i32>(0), dimensions - vec2<i32>(1));
    let color = textureLoad(source_texture, texel, 0);
    let line = u32(floor(position.y));
    let shade = select(1.0, 0.5, (line & 1u) == 1u);
    return vec4<f32>(color.rgb * shade, color.a);
}
"#;

const PIXELATE_FRAGMENT_WGSL: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(source_texture));
    let last = max(dimensions - vec2<i32>(1), vec2<i32>(0));
    let texel = vec2<i32>(position.xy);
    let sample_texel = min(vec2<i32>((texel.x / 4) * 4 + 2, (texel.y / 4) * 4 + 2), last);
    return textureLoad(source_texture, sample_texel, 0);
}
"#;

#[derive(Clone, Copy, Debug)]
pub enum WgpuShaderSource<'a> {
    Wgsl {
        source: &'a str,
        entry_point: &'a str,
    },
    Glsl {
        source: &'a str,
        stage: GlslStage,
    },
    Hlsl {
        source: &'a str,
        entry_point: &'a str,
        stage: crate::hlsl::HlslStage,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgpuShaderError {
    Adapter(String),
    Device(String),
    Source(String),
    WrongStage {
        expected: &'static str,
        actual: &'static str,
    },
    Pipeline(String),
}

impl std::fmt::Display for WgpuShaderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adapter(error) => write!(formatter, "WGPU adapter request failed: {error}"),
            Self::Device(error) => write!(formatter, "WGPU device request failed: {error}"),
            Self::Source(error) => write!(formatter, "Shader source was rejected: {error}"),
            Self::WrongStage { expected, actual } => {
                write!(
                    formatter,
                    "Expected a {expected} shader stage, got {actual}."
                )
            }
            Self::Pipeline(error) => write!(formatter, "WGPU pipeline validation failed: {error}"),
        }
    }
}

impl std::error::Error for WgpuShaderError {}

/// A headless WGPU device requested without a surface or native window.
pub struct WgpuShaderBackend {
    _instance: wgpu::Instance,
    adapter_info: wgpu::AdapterInfo,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl WgpuShaderBackend {
    /// Request an adapter for offscreen use. Set `force_fallback_adapter` to
    /// prefer a software adapter for low-impact headless validation.
    pub async fn request_headless(force_fallback_adapter: bool) -> Result<Self, WgpuShaderError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter,
                compatible_surface: None,
            })
            .await
            .map_err(|error| WgpuShaderError::Adapter(error.to_string()))?;
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("DoofusDock optional shader backend"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| WgpuShaderError::Device(error.to_string()))?;
        Ok(Self {
            _instance: instance,
            adapter_info,
            device,
            queue,
        })
    }

    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Report shader effects as available only after adapter/device creation
    /// succeeded. Shader-driven chrome remains on its established fallback.
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            software: false,
            raster_skin: true,
            shaders: true,
        }
    }

    /// Compile one vertex and one fragment source and create a texture-backed
    /// fullscreen effect. Both entry points use the fixed bindings documented
    /// by `FULLSCREEN_EFFECT_BINDINGS`.
    pub async fn create_fullscreen_effect(
        &self,
        vertex: WgpuShaderSource<'_>,
        fragment: WgpuShaderSource<'_>,
        target_format: wgpu::TextureFormat,
    ) -> Result<WgpuFullscreenEffect, WgpuShaderError> {
        let vertex = compile_stage(vertex, WgpuPipelineStage::Vertex)?;
        let fragment = compile_stage(fragment, WgpuPipelineStage::Fragment)?;

        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let vertex_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("DoofusDock fullscreen vertex shader"),
                source: wgpu::ShaderSource::SpirV(Cow::Owned(vertex.words)),
            });
        let fragment_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("DoofusDock fullscreen fragment shader"),
                source: wgpu::ShaderSource::SpirV(Cow::Owned(fragment.words)),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("DoofusDock fullscreen effect source texture"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let bind_group_layouts = [&bind_group_layout];
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("DoofusDock fullscreen effect layout"),
                bind_group_layouts: &bind_group_layouts,
                push_constant_ranges: &[],
            });
        let color_targets = [Some(wgpu::ColorTargetState {
            format: target_format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("DoofusDock fullscreen effect"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vertex_module,
                    entry_point: Some(&vertex.entry_point),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_module,
                    entry_point: Some(&fragment.entry_point),
                    compilation_options: Default::default(),
                    targets: &color_targets,
                }),
                multiview: None,
                cache: None,
            });
        if let Some(error) = self.device.pop_error_scope().await {
            return Err(WgpuShaderError::Pipeline(error.to_string()));
        }

        Ok(WgpuFullscreenEffect {
            bind_group_layout,
            pipeline,
            target_format,
        })
    }

    /// Create one of the source-embedded, parameter-free effects. These use
    /// the same bounded validation, fixed bindings, caller-owned textures, and
    /// render pipeline as custom effects. The host remains responsible for
    /// sizing/synchronizing textures and selecting a visual fallback.
    pub async fn create_builtin_effect(
        &self,
        effect: WgpuBuiltinEffect,
        target_format: wgpu::TextureFormat,
    ) -> Result<WgpuFullscreenEffect, WgpuShaderError> {
        self.create_fullscreen_effect(
            WgpuShaderSource::Wgsl {
                source: BUILTIN_FULLSCREEN_VERTEX_WGSL,
                entry_point: "vs_main",
            },
            WgpuShaderSource::Wgsl {
                source: effect.fragment_source(),
                entry_point: "fs_main",
            },
            target_format,
        )
        .await
    }
}

/// Binding contract for fullscreen shader sources: group 0, binding 0 is a
/// filterable 2D source texture; group 0, binding 1 is its filtering sampler.
pub const FULLSCREEN_EFFECT_BINDINGS: &str =
    "@group(0) @binding(0) texture_2d<f32>; @group(0) @binding(1) sampler";

pub struct WgpuFullscreenEffect {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    target_format: wgpu::TextureFormat,
}

impl WgpuFullscreenEffect {
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// Create source-texture bindings. Cache the returned bind group while the
    /// source texture and sampler remain unchanged.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        source: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("DoofusDock fullscreen effect input"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Encode one full-target draw. Source and target textures must be distinct.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        target: &wgpu::TextureView,
    ) {
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("DoofusDock fullscreen shader effect pass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

struct CompiledStage {
    words: Vec<u32>,
    entry_point: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WgpuPipelineStage {
    Vertex,
    Fragment,
}

impl WgpuPipelineStage {
    fn label(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Fragment => "fragment",
        }
    }
}

fn compile_stage(
    source: WgpuShaderSource<'_>,
    expected: WgpuPipelineStage,
) -> Result<CompiledStage, WgpuShaderError> {
    match source {
        WgpuShaderSource::Wgsl {
            source,
            entry_point,
        } => {
            let compiled =
                shader_validation::compile_wgsl_entry_point_to_spirv(source, entry_point)
                    .map_err(|error| WgpuShaderError::Source(error.to_string()))?;
            let (actual_stage, stage) = match compiled.stage {
                WgslStage::Vertex => (WgpuPipelineStage::Vertex, "vertex"),
                WgslStage::Fragment => (WgpuPipelineStage::Fragment, "fragment"),
                WgslStage::Compute => (expected, "compute"),
                WgslStage::Task => (expected, "task"),
                WgslStage::Mesh => (expected, "mesh"),
            };
            if actual_stage != expected || !matches!(stage, "vertex" | "fragment") {
                return Err(WgpuShaderError::WrongStage {
                    expected: expected.label(),
                    actual: stage,
                });
            }
            Ok(CompiledStage {
                words: compiled.words,
                entry_point: compiled.name,
            })
        }
        WgpuShaderSource::Glsl { source, stage } => {
            let compiled = shader_validation::compile_glsl_to_spirv(source, stage)
                .map_err(|error| WgpuShaderError::Source(error.to_string()))?;
            let (actual_stage, label) = match stage {
                GlslStage::Vertex => (WgpuPipelineStage::Vertex, "vertex"),
                GlslStage::Fragment => (WgpuPipelineStage::Fragment, "fragment"),
                GlslStage::Compute => (expected, "compute"),
            };
            if actual_stage != expected || label == "compute" {
                return Err(WgpuShaderError::WrongStage {
                    expected: expected.label(),
                    actual: label,
                });
            }
            Ok(CompiledStage {
                words: compiled.words,
                entry_point: compiled.name,
            })
        }
        WgpuShaderSource::Hlsl {
            source,
            entry_point,
            stage,
        } => {
            let actual_stage = match stage {
                crate::hlsl::HlslStage::Vertex => WgpuPipelineStage::Vertex,
                crate::hlsl::HlslStage::Fragment => WgpuPipelineStage::Fragment,
                crate::hlsl::HlslStage::Compute => expected,
            };
            if actual_stage != expected || matches!(stage, crate::hlsl::HlslStage::Compute) {
                return Err(WgpuShaderError::WrongStage {
                    expected: expected.label(),
                    actual: match stage {
                        crate::hlsl::HlslStage::Vertex => "vertex",
                        crate::hlsl::HlslStage::Fragment => "fragment",
                        crate::hlsl::HlslStage::Compute => "compute",
                    },
                });
            }
            let words = crate::hlsl::compile_hlsl_to_spirv(source, stage, entry_point)
                .map_err(|error| WgpuShaderError::Source(error.to_string()))?;
            Ok(CompiledStage {
                words,
                entry_point: entry_point.to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERTEX: &str = r#"
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Out {
    var positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var out: Out;
    out.position = vec4(positions[index], 0.0, 1.0);
    out.uv = vec2(0.5, 0.5);
    return out;
}
"#;

    const FRAGMENT: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@fragment fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, source_sampler, uv);
    return vec4(1.0 - color.rgb, color.a);
}
"#;

    const HLSL_VERTEX: &str = r#"
struct Out { float4 position : SV_Position; float2 uv : TEXCOORD0; };
Out vs_main(uint vertex_id : SV_VertexID) {
    float2 positions[3] = { float2(-1.0, -1.0), float2(3.0, -1.0), float2(-1.0, 3.0) };
    float2 uvs[3] = { float2(0.0, 1.0), float2(2.0, 1.0), float2(0.0, -1.0) };
    Out output;
    output.position = float4(positions[vertex_id], 0.0, 1.0);
    output.uv = uvs[vertex_id];
    return output;
}
"#;

    const HLSL_FRAGMENT: &str = r#"
[[vk::binding(0, 0)]] Texture2D<float4> source_texture : register(t0);
[[vk::binding(1, 0)]] SamplerState source_sampler : register(s0);
float4 fs_main(float2 uv : TEXCOORD0) : SV_Target0 {
    float4 color = source_texture.Sample(source_sampler, uv);
    return float4(1.0 - color.rgb, color.a);
}
"#;

    async fn render_builtin_rgba8(
        backend: &WgpuShaderBackend,
        effect_kind: WgpuBuiltinEffect,
        width: u32,
        height: u32,
        source_pixels: &[[u8; 4]],
    ) -> Vec<[u8; 4]> {
        assert!(width > 0 && height > 0 && width <= 64 && height <= 64);
        assert_eq!(source_pixels.len(), (width * height) as usize);
        let device = backend.device();
        let source = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("DoofusDock built-in effect headless input"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let source_bytes: Vec<u8> = source_pixels
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .collect();
        backend.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &source,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &source_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("DoofusDock built-in effect headless output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let effect = backend
            .create_builtin_effect(effect_kind, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .expect("compile bounded built-in effect");
        let bindings = effect.create_bind_group(device, &source_view, &sampler);
        let padded_bytes_per_row = 256;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DoofusDock built-in effect pixel readback"),
            size: u64::from(padded_bytes_per_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("DoofusDock built-in effect headless commands"),
        });
        effect.encode(&mut encoder, &bindings, &target_view);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        backend.queue().submit([encoder.finish()]);
        let slice = output.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::Wait)
            .expect("wait for built-in effect readback");
        receiver
            .recv()
            .expect("built-in effect readback callback")
            .expect("map built-in effect readback");
        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height as usize {
            for x in 0..width as usize {
                let offset = y * padded_bytes_per_row as usize + x * 4;
                pixels.push(mapped[offset..offset + 4].try_into().unwrap());
            }
        }
        drop(mapped);
        output.unmap();
        pixels
    }

    #[test]
    fn built_in_effects_compile_render_and_read_back_offscreen() {
        pollster::block_on(async {
            let backend = match WgpuShaderBackend::request_headless(true).await {
                Ok(backend) => backend,
                Err(fallback_error) => WgpuShaderBackend::request_headless(false)
                    .await
                    .unwrap_or_else(|error| {
                        panic!("fallback adapter unavailable ({fallback_error}); regular headless adapter unavailable ({error})")
                    }),
            };
            let input = [[100, 150, 200, 255]];
            let grayscale =
                render_builtin_rgba8(&backend, WgpuBuiltinEffect::Grayscale, 1, 1, &input).await;
            assert_eq!(grayscale, [[143, 143, 143, 255]]);

            let sepia =
                render_builtin_rgba8(&backend, WgpuBuiltinEffect::Sepia, 1, 1, &input).await;
            for (actual, expected) in sepia[0].iter().zip([192, 171, 134, 255]) {
                assert!(actual.abs_diff(expected) <= 1, "sepia={sepia:?}");
            }

            let scanlines = render_builtin_rgba8(
                &backend,
                WgpuBuiltinEffect::Scanlines,
                1,
                4,
                &[[100, 150, 200, 255]; 4],
            )
            .await;
            assert_eq!(scanlines[0], [100, 150, 200, 255]);
            assert_eq!(scanlines[1], [50, 75, 100, 255]);
            assert_eq!(scanlines[2], [100, 150, 200, 255]);
            assert_eq!(scanlines[3], [50, 75, 100, 255]);

            let mut blocks = vec![[0, 0, 0, 255]; 8 * 8];
            for y in 0..8 {
                for x in 0..8 {
                    blocks[(y * 8 + x) as usize] = match (x / 4, y / 4) {
                        (0, 0) => [255, 0, 0, 255],
                        (1, 0) => [0, 255, 0, 255],
                        (0, 1) => [0, 0, 255, 255],
                        _ => [255, 255, 0, 255],
                    };
                }
            }
            let pixelated =
                render_builtin_rgba8(&backend, WgpuBuiltinEffect::Pixelate, 8, 8, &blocks).await;
            assert_eq!(pixelated[0], [255, 0, 0, 255]);
            assert_eq!(pixelated[3], [255, 0, 0, 255]);
            assert_eq!(pixelated[4], [0, 255, 0, 255]);
            assert_eq!(pixelated[7], [0, 255, 0, 255]);
            assert_eq!(pixelated[4 * 8], [0, 0, 255, 255]);
            assert_eq!(pixelated[7 * 8 + 7], [255, 255, 0, 255]);

            eprintln!(
                "headless built-in shader adapter: {:?}",
                backend.adapter_info()
            );
        });
    }

    #[test]
    fn headless_hlsl_sources_create_an_offscreen_effect_pipeline() {
        pollster::block_on(async {
            let backend = match WgpuShaderBackend::request_headless(true).await {
                Ok(backend) => backend,
                Err(fallback_error) => WgpuShaderBackend::request_headless(false)
                    .await
                    .unwrap_or_else(|error| {
                        panic!("fallback adapter unavailable ({fallback_error}); regular headless adapter unavailable ({error})")
                    }),
            };
            let _effect = backend
                .create_fullscreen_effect(
                    WgpuShaderSource::Hlsl {
                        source: HLSL_VERTEX,
                        entry_point: "vs_main",
                        stage: crate::hlsl::HlslStage::Vertex,
                    },
                    WgpuShaderSource::Hlsl {
                        source: HLSL_FRAGMENT,
                        entry_point: "fs_main",
                        stage: crate::hlsl::HlslStage::Fragment,
                    },
                    wgpu::TextureFormat::Rgba8Unorm,
                )
                .await
                .expect("create headless HLSL effect pipeline");
            eprintln!("headless HLSL WGPU adapter: {:?}", backend.adapter_info());
        });
    }

    #[test]
    fn headless_tessellated_egui_geometry_renders_to_gpu_texture_and_reads_back() {
        pollster::block_on(async {
            let backend = match WgpuShaderBackend::request_headless(true).await {
                Ok(backend) => backend,
                Err(fallback_error) => WgpuShaderBackend::request_headless(false)
                    .await
                    .unwrap_or_else(|error| {
                        panic!("fallback adapter unavailable ({fallback_error}); regular headless adapter unavailable ({error})")
                    }),
            };
            let adapter_info = backend.adapter_info().clone();

            let egui_context = egui::Context::default();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(32.0));
            let mut full_output = egui_context.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(egui::pos2(8.0, 8.0), egui::pos2(24.0, 24.0)),
                        0,
                        egui::Color32::RED,
                    );
                },
            );
            // This contract feeds untextured solid-color egui meshes to a
            // minimal GPU pipeline, so the font-atlas texture delta is unused.
            full_output.textures_delta.clear();
            let primitives =
                egui_context.tessellate(full_output.shapes, full_output.pixels_per_point);
            let meshes: Vec<_> = primitives
                .into_iter()
                .filter_map(|primitive| match primitive.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => Some((primitive.clip_rect, mesh)),
                    egui::epaint::Primitive::Callback(_) => None,
                })
                .collect();
            assert!(!meshes.is_empty(), "egui produced no mesh geometry");

            let device = backend.device();
            let queue = backend.queue();
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("DoofusDock egui offscreen readback shader"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position.x / 16.0 - 1.0, 1.0 - position.y / 16.0, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#
                    .into(),
                ),
            });
            const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
                wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4];
            let vertex_layout = wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            };
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("DoofusDock egui mesh offscreen pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[vertex_layout],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("DoofusDock egui offscreen render target"),
                size: wgpu::Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("DoofusDock egui offscreen readback commands"),
            });
            {
                let attachments = [Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })];
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("DoofusDock egui offscreen render pass"),
                    color_attachments: &attachments,
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&pipeline);
                for (clip_rect, mesh) in &meshes {
                    if mesh.indices.is_empty() || mesh.vertices.is_empty() {
                        continue;
                    }
                    let mut vertex_bytes = Vec::with_capacity(mesh.vertices.len() * 12);
                    for vertex in &mesh.vertices {
                        vertex_bytes.extend_from_slice(&vertex.pos.x.to_le_bytes());
                        vertex_bytes.extend_from_slice(&vertex.pos.y.to_le_bytes());
                        vertex_bytes.extend_from_slice(&vertex.color.to_array());
                    }
                    let mut index_bytes = Vec::with_capacity(mesh.indices.len() * 4);
                    for index in &mesh.indices {
                        index_bytes.extend_from_slice(&index.to_le_bytes());
                    }
                    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("DoofusDock egui mesh vertices"),
                        size: vertex_bytes.len() as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&vertex_buffer, 0, &vertex_bytes);
                    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("DoofusDock egui mesh indices"),
                        size: index_bytes.len() as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&index_buffer, 0, &index_bytes);
                    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    let clip = clip_rect.intersect(screen);
                    let x = clip.min.x.floor().clamp(0.0, 32.0) as u32;
                    let y = clip.min.y.floor().clamp(0.0, 32.0) as u32;
                    let right = clip.max.x.ceil().clamp(0.0, 32.0) as u32;
                    let bottom = clip.max.y.ceil().clamp(0.0, 32.0) as u32;
                    if right <= x || bottom <= y {
                        continue;
                    }
                    pass.set_scissor_rect(x, y, right - x, bottom - y);
                    pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
                }
            }

            let output = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("DoofusDock egui GPU readback buffer"),
                size: 32 * 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &target,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &output,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(32),
                    },
                },
                wgpu::Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);

            let slice = output.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            device
                .poll(wgpu::PollType::Wait)
                .expect("wait for egui GPU readback");
            receiver
                .recv()
                .expect("egui GPU readback callback")
                .expect("map egui GPU readback");
            let mapped = slice.get_mapped_range();
            let pixel = |x: usize, y: usize| {
                let offset = y * 256 + x * 4;
                &mapped[offset..offset + 4]
            };
            assert_eq!(pixel(16, 16), &[255, 0, 0, 255]);
            assert_eq!(pixel(2, 2), &[0, 0, 0, 255]);
            eprintln!("headless egui GPU readback adapter: {adapter_info:?}");
        });
    }

    #[test]
    fn headless_adapter_executes_offscreen_fullscreen_effect() {
        pollster::block_on(async {
            let backend = match WgpuShaderBackend::request_headless(true).await {
                Ok(backend) => backend,
                Err(fallback_error) => WgpuShaderBackend::request_headless(false)
                    .await
                    .unwrap_or_else(|error| {
                        panic!("fallback adapter unavailable ({fallback_error}); regular headless adapter unavailable ({error})")
                    }),
            };
            eprintln!("headless WGPU adapter: {:?}", backend.adapter_info());
            let effect = backend
                .create_fullscreen_effect(
                    WgpuShaderSource::Wgsl {
                        source: VERTEX,
                        entry_point: "vs_main",
                    },
                    WgpuShaderSource::Wgsl {
                        source: FRAGMENT,
                        entry_point: "fs_main",
                    },
                    wgpu::TextureFormat::Rgba8Unorm,
                )
                .await
                .expect("create shader pipeline");

            let source = backend.device().create_texture(&wgpu::TextureDescriptor {
                label: Some("headless shader source"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            backend.queue().write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &source,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &[64, 128, 192, 255],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            let target = backend.device().create_texture(&wgpu::TextureDescriptor {
                label: Some("headless shader target"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let sampler = backend
                .device()
                .create_sampler(&wgpu::SamplerDescriptor::default());
            let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
            let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let bindings = effect.create_bind_group(backend.device(), &source_view, &sampler);
            let output = backend.device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("headless shader readback"),
                size: 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder =
                backend
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("headless shader commands"),
                    });
            effect.encode(&mut encoder, &bindings, &target_view);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &target,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &output,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(1),
                    },
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            backend.queue().submit([encoder.finish()]);

            let slice = output.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            backend
                .device()
                .poll(wgpu::PollType::Wait)
                .expect("wait for headless render");
            receiver
                .recv()
                .expect("readback callback")
                .expect("map headless output");
            let mapped = slice.get_mapped_range();
            let pixel = &mapped[..4];
            assert!((pixel[0] as i16 - 191).abs() <= 1, "pixel={pixel:?}");
            assert!((pixel[1] as i16 - 127).abs() <= 1, "pixel={pixel:?}");
            assert!((pixel[2] as i16 - 63).abs() <= 1, "pixel={pixel:?}");
            assert_eq!(pixel[3], 255);
        });
    }
}
