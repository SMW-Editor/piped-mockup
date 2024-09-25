use std::sync::Arc;
use std::sync::RwLock;

use glam::Vec2;

use iced::Point;
use iced::{
    advanced::Shell,
    event::Status,
    mouse::{self, Cursor},
    widget::shader::{self, wgpu, wgpu::util::DeviceExt, Event, Viewport},
    Element, Rectangle,
};

// We have to alias the shader element because it has the same name as the iced::widget::shader module, and the `self` syntax only imports the module.
use iced::widget::shader as shader_element;

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    resolution: Vec2,
    padding: u32,
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SpriteTile {
    x: u32,
    y: u32,
    id: u32,
    pal: u8,
    scale: u8,
    flags: u16,
}

#[derive(Debug)]
struct PaletteShaderPipeline {
    pipeline: wgpu::RenderPipeline,
    palette_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl PaletteShaderPipeline {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PaletteShaderPipeline shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "palette_shader.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PaletteShaderPipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let mut palette = image::open("assets/palette.png").unwrap().to_rgba32f();
        palette
            .as_flat_samples_mut()
            .samples
            .iter_mut()
            .for_each(|c| *c = c.powf(2.2));
        let palette_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shader_quad palette buffer"),
            contents: bytemuck::cast_slice(palette.as_flat_samples().samples),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shader_quad bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: palette_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            palette_buffer,
            bind_group,
        }
    }

    fn write_uniforms(&mut self, queue: &wgpu::Queue, uniforms: &Uniforms) {
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clip_bounds: Rectangle<u32>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fill color test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_viewport(
            clip_bounds.x as f32,
            clip_bounds.y as f32,
            clip_bounds.width as f32,
            clip_bounds.height as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.bind_group, &[]);

        pass.draw(0..4, 0..1);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    CursorMoved(Point),
}

#[derive(Default, Debug, Clone)]
pub struct PaletteState {
    pipeline: Arc<RwLock<Option<PaletteShaderPipeline>>>,
}

#[derive(Debug)]
pub struct PalettePrimitive {
    state: PaletteState,
}

impl PalettePrimitive {
    fn new(state: PaletteState) -> Self {
        Self {
            state,
        }
    }
}

impl shader::Primitive for PalettePrimitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        _storage: &mut shader::Storage,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        /*
        if !storage.has::<PaletteShaderPipeline>() {
            storage.store(PaletteShaderPipeline::new(
                self.palette_bytes.clone(),
                device,
                format,
            ));
        }

        let pipeline = storage.get_mut::<PaletteShaderPipeline>().unwrap();
        */
        let mut pipeline = self.state.pipeline.write().unwrap();
        let pipeline = pipeline.get_or_insert_with(|| {
            PaletteShaderPipeline::new(device, format)
        });
        pipeline.write_uniforms(
            queue,
            &Uniforms {
                resolution: Vec2::new(bounds.width as f32, bounds.height as f32),
                padding: 0,
            },
        );
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        _storage: &shader::Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        //let pipeline = storage.get::<PaletteShaderPipeline>().unwrap();
        self.state
            .pipeline
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .render(target, encoder, *clip_bounds);
    }
}

pub struct Palette {
    palette_program: PaletteProgram,
}
impl Palette {
    pub fn new() -> Self {
        Self {
            palette_program: PaletteProgram::new(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        shader_element(&self.palette_program)
            .width(256)
            .height(256)
            .into()
    }
}
struct PaletteProgram {
    state: PaletteState,
}

impl PaletteProgram {
    fn new() -> Self {
        Self {
            state: Default::default(),
        }
    }
}

impl shader::Program<Message> for PaletteProgram {
    type State = ();
    type Primitive = PalettePrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        PalettePrimitive::new(self.state.clone())
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
        _shell: &mut Shell<'_, Message>,
    ) -> (Status, Option<Message>) {
        #[allow(clippy::single_match)]
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    return (Status::Ignored, Some(Message::CursorMoved(pos)));
                }
            }
            _ => {}
        };

        (Status::Ignored, None)
    }
}
