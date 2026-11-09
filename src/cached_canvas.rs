use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget,
        graphics::{geometry, mesh},
        layout, renderer,
        widget::{Tree, tree},
    },
    mouse, wgpu,
    widget::{canvas, shader},
    window,
};
use std::{collections::BTreeMap, marker::PhantomData, ops::Range, sync::Weak};

pub struct CachedCanvas<P, Message> {
    width: Length,
    height: Length,
    program: P,
    message: PhantomData<Message>,
    last_mouse_interaction: Option<mouse::Interaction>,
}

pub fn cached_canvas<P, Message>(program: P) -> CachedCanvas<P, Message>
where
    P: canvas::Program<Message>,
{
    CachedCanvas::new(program)
}

impl<P, Message> CachedCanvas<P, Message>
where
    P: canvas::Program<Message>,
{
    pub fn new(program: P) -> Self {
        Self {
            width: Length::Fixed(100.0),
            height: Length::Fixed(100.0),
            program,
            message: PhantomData,
            last_mouse_interaction: None,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<P, Message> Widget<Message, Theme, Renderer> for CachedCanvas<P, Message>
where
    P: canvas::Program<Message>,
{
    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<P::State>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(P::State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<P::State>();
        let is_redraw_request =
            matches!(event, Event::Window(window::Event::RedrawRequested(_now)));

        if let Some(action) = self.program.update(state, event, bounds, cursor) {
            let (message, redraw_request, event_status) = action.into_inner();
            shell.request_redraw_at(redraw_request);
            if let Some(message) = message {
                shell.publish(message);
            }
            if event_status == iced::event::Status::Captured {
                shell.capture_event();
            }
        }

        if shell.redraw_request() != window::RedrawRequest::NextFrame {
            let interaction = self.mouse_interaction(tree, layout, cursor, viewport, renderer);
            if is_redraw_request {
                self.last_mouse_interaction = Some(interaction);
            } else if self
                .last_mouse_interaction
                .is_some_and(|last| last != interaction)
            {
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        self.program.mouse_interaction(
            tree.state.downcast_ref::<P::State>(),
            layout.bounds(),
            cursor,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::renderer::Renderer as _;

        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }

        let state = tree.state.downcast_ref::<P::State>();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            let layers = self.program.draw(state, renderer, theme, bounds, cursor);
            for layer in layers {
                draw_layer(renderer, bounds, layer);
            }
        });
    }
}

impl<'a, P, Message> From<CachedCanvas<P, Message>> for Element<'a, Message>
where
    Message: 'a,
    P: 'a + canvas::Program<Message>,
{
    fn from(canvas: CachedCanvas<P, Message>) -> Self {
        Element::new(canvas)
    }
}

const MAX_PRIMITIVE_LOGICAL_DIMENSION: f32 = 2048.0;

fn draw_layer(renderer: &mut Renderer, bounds: Rectangle, layer: canvas::Geometry) {
    match layer {
        canvas::Geometry::<Renderer>::Primary(layer) => match layer {
            iced_wgpu::Geometry::Live { .. } => geometry::Renderer::draw_geometry(
                renderer,
                canvas::Geometry::<Renderer>::Primary(layer),
            ),
            iced_wgpu::Geometry::Cached(mut cache) => {
                if let Some(meshes) = cache.meshes.take() {
                    if can_cache(bounds, &meshes) {
                        iced_wgpu::primitive::Renderer::draw_primitive(
                            renderer,
                            Rectangle::with_size(bounds.size()),
                            CachedMeshes { cache: meshes },
                        );
                    } else {
                        mesh::Renderer::draw_mesh_cache(renderer, meshes);
                    }
                }
                if cache.images.is_some() || cache.text.is_some() {
                    geometry::Renderer::draw_geometry(
                        renderer,
                        canvas::Geometry::<Renderer>::Primary(iced_wgpu::Geometry::Cached(cache)),
                    );
                }
            }
        },
        canvas::Geometry::<Renderer>::Secondary(layer) => {
            geometry::Renderer::draw_geometry(
                renderer,
                canvas::Geometry::<Renderer>::Secondary(layer),
            );
        }
    }
}

fn can_cache(bounds: Rectangle, cache: &mesh::Cache) -> bool {
    bounds.width <= MAX_PRIMITIVE_LOGICAL_DIMENSION
        && bounds.height <= MAX_PRIMITIVE_LOGICAL_DIMENSION
        && is_cacheable(cache)
}

fn is_cacheable(cache: &mesh::Cache) -> bool {
    !cache.is_empty()
        && cache.batch().iter().all(
            |mesh| matches!(mesh, mesh::Mesh::Solid { buffers, .. } if !buffers.indices.is_empty()),
        )
}

#[derive(Debug)]
struct CachedMeshes {
    cache: mesh::Cache,
}

impl shader::Primitive for CachedMeshes {
    type Pipeline = CachedMeshPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        pipeline.prepare(device, queue, &self.cache, bounds, viewport);
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        pipeline.draw(self.cache.id(), render_pass)
    }
}

struct CachedMeshPipeline {
    format: wgpu::TextureFormat,
    mesh_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    entries: BTreeMap<mesh::Id, Entry>,
}

struct Entry {
    version: usize,
    width: u32,
    height: u32,
    meshes: Weak<[mesh::Mesh]>,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    ranges: Vec<DrawRange>,
    kind: EntryKind,
}

enum EntryKind {
    Textured { bind_group: wgpu::BindGroup },
    Direct,
}

struct DrawRange {
    indices: Range<u32>,
    clip: Option<Rectangle<u32>>,
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl shader::Pipeline for CachedMeshPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("maolan cached canvas mesh shader"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });
        let mesh_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("maolan cached canvas mesh layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("maolan cached canvas mesh pipeline"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("maolan cached canvas texture layout"),
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("maolan cached canvas sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("maolan cached canvas blit shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });
        let blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("maolan cached canvas blit layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("maolan cached canvas blit pipeline"),
            layout: Some(&blit_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            format,
            mesh_pipeline,
            blit_pipeline,
            texture_layout,
            sampler,
            entries: BTreeMap::new(),
        }
    }

    fn trim(&mut self) {
        self.entries
            .retain(|_, entry| entry.meshes.strong_count() > 0);
    }
}

impl CachedMeshPipeline {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mesh::Cache,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let scale = viewport.scale_factor();
        let width = (bounds.width * scale).ceil().max(1.0) as u32;
        let height = (bounds.height * scale).ceil().max(1.0) as u32;
        if self.entries.get(&cache.id()).is_some_and(|entry| {
            entry.version == cache.version() && entry.width == width && entry.height == height
        }) {
            return;
        }

        let limit = device.limits().max_texture_dimension_2d;
        let textured = width <= limit && height <= limit;
        let buffers = upload_geometry(device, cache, width, height, scale);
        let kind = if textured {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("maolan cached canvas texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("maolan cached canvas bind group"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("maolan cached canvas encoder"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("maolan cached canvas render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_vertex_buffer(0, buffers.vertices.slice(..));
                pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
                for range in &buffers.ranges {
                    if let Some(clip) = range.clip {
                        pass.set_scissor_rect(clip.x, clip.y, clip.width, clip.height);
                        pass.draw_indexed(range.indices.clone(), 0, 0..1);
                    }
                }
            }
            queue.submit([encoder.finish()]);
            EntryKind::Textured { bind_group }
        } else {
            EntryKind::Direct
        };

        self.entries.insert(
            cache.id(),
            Entry {
                version: cache.version(),
                width,
                height,
                meshes: cache.downgrade(),
                vertices: buffers.vertices,
                indices: buffers.indices,
                ranges: buffers.ranges,
                kind,
            },
        );
    }

    fn draw(&self, id: mesh::Id, pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(entry) = self.entries.get(&id) else {
            return false;
        };
        match &entry.kind {
            EntryKind::Textured { bind_group } => {
                pass.set_pipeline(&self.blit_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..6, 0..1);
            }
            EntryKind::Direct => {
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_vertex_buffer(0, entry.vertices.slice(..));
                pass.set_index_buffer(entry.indices.slice(..), wgpu::IndexFormat::Uint32);
                for range in &entry.ranges {
                    if range.clip.is_some() {
                        pass.draw_indexed(range.indices.clone(), 0, 0..1);
                    }
                }
            }
        }
        true
    }
}

struct GeometryBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    ranges: Vec<DrawRange>,
}

fn upload_geometry(
    device: &wgpu::Device,
    cache: &mesh::Cache,
    width: u32,
    height: u32,
    scale: f32,
) -> GeometryBuffers {
    use wgpu::util::DeviceExt;

    let projection =
        iced::Transformation::orthographic(width, height) * iced::Transformation::scale(scale);
    let target = Rectangle::<f32>::from(Rectangle::with_size(Size::new(width, height)));
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut ranges = Vec::new();

    for mesh in cache.batch() {
        let mesh::Mesh::Solid {
            buffers,
            transformation,
            ..
        } = mesh
        else {
            continue;
        };
        let vertex_offset = vertices.len() as u32;
        vertices.extend(buffers.vertices.iter().map(|vertex| {
            let position =
                Point::new(vertex.position[0], vertex.position[1]) * *transformation * projection;
            Vertex {
                position: [position.x, position.y],
                color: vertex.color.components(),
            }
        }));
        let index_start = indices.len() as u32;
        indices.extend(buffers.indices.iter().map(|index| index + vertex_offset));
        let index_end = indices.len() as u32;
        let clip = (mesh.clip_bounds() * iced::Transformation::scale(scale))
            .intersection(&target)
            .and_then(Rectangle::snap);
        ranges.push(DrawRange {
            indices: index_start..index_end,
            clip,
        });
    }

    GeometryBuffers {
        vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("maolan cached canvas vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }),
        indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("maolan cached canvas indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }),
        ranges,
    }
}

const MESH_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = vec4<f32>(input.color.rgb * input.color.a, input.color.a);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

const BLIT_SHADER: &str = r#"
@group(0) @binding(0) var cached_texture: texture_2d<f32>;
@group(0) @binding(1) var cached_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );
    var output: VertexOutput;
    output.uv = uvs[index];
    output.position = vec4<f32>(output.uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(cached_texture, cached_sampler, input.uv);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::graphics::color;
    use std::sync::Arc;

    fn solid_cache() -> mesh::Cache {
        let mesh = mesh::Mesh::Solid {
            buffers: mesh::Indexed {
                vertices: vec![mesh::SolidVertex2D {
                    position: [0.0, 0.0],
                    color: color::pack(iced::Color::WHITE),
                }],
                indices: vec![0, 0, 0],
            },
            transformation: iced::Transformation::IDENTITY,
            clip_bounds: Rectangle::new(Point::ORIGIN, Size::new(10.0, 10.0)),
        };
        mesh::Cache::new(Arc::from(vec![mesh]))
    }

    #[test]
    fn solid_mesh_caches_are_eligible() {
        assert!(is_cacheable(&solid_cache()));
        assert!(!is_cacheable(&mesh::Cache::new(Arc::from(Vec::new()))));
    }

    #[test]
    fn oversized_canvases_use_the_standard_renderer() {
        let cache = solid_cache();
        let normal = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));
        let oversized = Rectangle::new(
            Point::ORIGIN,
            Size::new(MAX_PRIMITIVE_LOGICAL_DIMENSION + 1.0, 100.0),
        );
        assert!(can_cache(normal, &cache));
        assert!(!can_cache(oversized, &cache));
    }

    #[test]
    fn cache_identity_is_stable_and_revision_tracks_changes() {
        let mut cache = solid_cache();
        let clone = cache.clone();
        assert_eq!(cache.id(), clone.id());
        assert_eq!(cache.version(), clone.version());

        cache.update(Arc::from(Vec::new()));
        assert_eq!(cache.id(), clone.id());
        assert_eq!(cache.version(), clone.version() + 1);
    }
}
