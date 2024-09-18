use std::sync::Arc;

use glam::Vec2;
// We have to alias the shader element because it has the same name as the iced::widget::shader module, and the `self` syntax only imports the module.
use iced::widget::shader as shader_element;
use iced::{
    advanced::Shell,
    event::Status,
    mouse::{self, Cursor},
    widget::{
        column, container, row,
        shader::{self, wgpu, wgpu::util::DeviceExt, Event},
        slider, text,
    },
    Alignment, Command, Element, Length, Rectangle, Size,
};

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    resolution: Vec2,
    origin: Vec2,
    scale: f32,
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

struct TilemapShaderPipeline {
    pipeline: wgpu::RenderPipeline,
    tiles: Vec<SpriteTile>,
    instance_buffer: wgpu::Buffer,
    palette_buffer: wgpu::Buffer,
    graphics_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl TilemapShaderPipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TilemapShaderPipeline shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "tilemap_shader.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TilemapShaderPipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SpriteTile>() as _,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Uint32x4,
                    }],
                }],
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

        let mut tiles = vec![];
        for i in 0..64u32 {
            let tx = i % 8 * 16;
            let ty = i / 8 * 16;
            tiles.push(SpriteTile {
                x: tx + 0,
                y: ty + 0,
                id: i * 4 + 0,
                flags: 0,
                pal: 3,
                scale: 1,
            });
            tiles.push(SpriteTile {
                x: tx + 8,
                y: ty + 0,
                id: i * 4 + 1,
                flags: 0,
                pal: 3,
                scale: 1,
            });
            tiles.push(SpriteTile {
                x: tx + 0,
                y: ty + 8,
                id: i * 4 + 2,
                flags: 0,
                pal: 3,
                scale: 1,
            });
            tiles.push(SpriteTile {
                x: tx + 8,
                y: ty + 8,
                id: i * 4 + 3,
                flags: 0,
                pal: 3,
                scale: 1,
            });
        }

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shader_quad instance buffer"),
            contents: bytemuck::cast_slice(&tiles),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut palette = image::open("assets/palette.png").unwrap().to_rgba32f();
        palette
            .as_flat_samples_mut()
            .samples
            .into_iter()
            .for_each(|c| *c = c.powf(2.2));
        let palette_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shader_quad palette buffer"),
            contents: bytemuck::cast_slice(&palette.as_flat_samples().samples),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let graphics = std::fs::read("assets/global.bin").unwrap();
        let graphics_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shader_quad graphics buffer"),
            contents: &graphics,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_quad uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as _,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: graphics_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            tiles,
            uniform_buffer,
            instance_buffer,
            palette_buffer,
            graphics_buffer,
            bind_group,
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: Rectangle<u32>,
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
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));

        pass.draw(0..4, 0..self.tiles.len() as u32);
    }
}

#[derive(Debug, Clone, Copy)]
struct Controls {
    max_iter: u32,
    zoom: f32,
    center: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    UpdateMaxIterations(u32),
    UpdateZoom(f32),
    PanningDelta(Vec2),
    ZoomDelta(Vec2, Rectangle, f32),
}

impl Controls {
    fn scale(&self) -> f32 {
        1.0 / 2.0_f32.powf(self.zoom) / ZOOM_PIXELS_FACTOR
    }
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            max_iter: ITERS_DEFAULT,
            zoom: ZOOM_DEFAULT,
            center: CENTER_DEFAULT,
        }
    }
}

#[derive(Debug)]
pub struct TilemapPrimitive {
    controls: Controls,
}

impl TilemapPrimitive {
    fn new(controls: Controls) -> Self {
        Self { controls }
    }
}

impl shader::Primitive for TilemapPrimitive {
    fn prepare(
        &self,
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: Rectangle,
        target_size: Size<u32>,
        _scale_factor: f32,
        storage: &mut shader::Storage,
    ) {
        if !storage.has::<TilemapShaderPipeline>() {
            storage.store(TilemapShaderPipeline::new(device, format));
        }

        let pipeline = storage.get_mut::<TilemapShaderPipeline>().unwrap();

        pipeline.update(
            queue,
            &Uniforms {
                resolution: Vec2::new(bounds.width as f32, bounds.height as f32),
                origin: self.controls.center,
                scale: self.controls.scale(),
                padding: 0,
            },
        );
    }

    fn render(
        &self,
        storage: &shader::Storage,
        target: &wgpu::TextureView,
        _target_size: Size<u32>,
        viewport: Rectangle<u32>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let pipeline = storage.get::<TilemapShaderPipeline>().unwrap();
        pipeline.render(target, encoder, viewport);
    }
}

#[derive(Default)]
pub enum MouseInteraction {
    #[default]
    Idle,
    Panning(Vec2),
}

pub struct TilemapWithControls {
    tilemap_program: TilemapProgram,
}
impl TilemapWithControls {
    pub fn new() -> Self {
        Self {
            tilemap_program: TilemapProgram::new(),
        }
    }
    /** Tell the tilemap to show these bytes. */
    pub fn show(&mut self, tilemap_bytes: Option<Arc<Vec<u8>>>) {
        if let Some(bytes) = &tilemap_bytes {
            println!(
                "Showing tilemap starting with these bytes: {:?}",
                (*bytes).iter().cloned().take(64).collect::<Vec<_>>()
            );
        }
        self.tilemap_program.tilemap_bytes = tilemap_bytes;
    }
    /** This is where tilemap handles its own messages. */
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::UpdateMaxIterations(max_iter) => {
                self.tilemap_program.controls.max_iter = max_iter;
                Command::none()
            }
            Message::UpdateZoom(zoom) => {
                self.tilemap_program.controls.zoom = zoom;
                Command::none()
            }
            Message::PanningDelta(delta) => {
                self.tilemap_program.controls.center -=
                    2.0 * delta * self.tilemap_program.controls.scale();
                Command::none()
            }
            Message::ZoomDelta(pos, bounds, delta) => {
                let delta = delta * ZOOM_WHEEL_SCALE;
                let prev_scale = self.tilemap_program.controls.scale();
                let prev_zoom = self.tilemap_program.controls.zoom;
                self.tilemap_program.controls.zoom = (prev_zoom + delta).clamp(ZOOM_MIN, ZOOM_MAX);

                let vec = pos - Vec2::new(bounds.width, bounds.height) * 0.5;
                let new_scale = self.tilemap_program.controls.scale();
                self.tilemap_program.controls.center += vec * (prev_scale - new_scale) * 2.0;
                Command::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let shader_controls = row![
            control(
                "Max iterations",
                slider(
                    ITERS_MIN..=ITERS_MAX,
                    self.tilemap_program.controls.max_iter,
                    move |iter| { Message::UpdateMaxIterations(iter) }
                )
                .width(Length::Fill)
            ),
            control(
                "Zoom",
                slider(
                    ZOOM_MIN..=ZOOM_MAX,
                    self.tilemap_program.controls.zoom,
                    move |zoom| { Message::UpdateZoom(zoom) }
                )
                .step(0.01)
                .width(Length::Fill)
            ),
        ];

        container(
            column![
                shader_element(&self.tilemap_program)
                    .width(512)
                    .height(iced::Length::Fill),
                //.width(512).height(512),
                shader_controls,
            ]
            .align_items(Alignment::Center),
        )
        .into()
    }
}
struct TilemapProgram {
    controls: Controls,
    tilemap_bytes: Option<Arc<Vec<u8>>>,
}

impl TilemapProgram {
    fn new() -> Self {
        Self {
            controls: Controls::default(),
            tilemap_bytes: None,
        }
    }
}

impl shader::Program<Message> for TilemapProgram {
    type State = MouseInteraction;
    type Primitive = TilemapPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        TilemapPrimitive::new(self.controls)
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
        _shell: &mut Shell<'_, Message>,
    ) -> (Status, Option<Message>) {
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            if let Some(pos) = cursor.position_in(bounds) {
                let pos = Vec2::new(pos.x, pos.y);
                let delta = match delta {
                    mouse::ScrollDelta::Lines { x: _, y } => y,
                    mouse::ScrollDelta::Pixels { x: _, y } => y,
                };
                return (
                    Status::Captured,
                    Some(Message::ZoomDelta(pos, bounds, delta)),
                );
            }
        }

        #[allow(clippy::single_match)]
        match state {
            MouseInteraction::Idle => match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if let Some(pos) = cursor.position_over(bounds) {
                        *state = MouseInteraction::Panning(Vec2::new(pos.x, pos.y));
                        return (Status::Captured, None);
                    }
                }
                _ => {}
            },
            MouseInteraction::Panning(prev_pos) => match event {
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    *state = MouseInteraction::Idle;
                }
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    let pos = Vec2::new(position.x, position.y);
                    let delta = pos - *prev_pos;
                    *state = MouseInteraction::Panning(pos);
                    return (Status::Captured, Some(Message::PanningDelta(delta)));
                }
                _ => {}
            },
        };

        (Status::Ignored, None)
    }
}

fn control<'a>(
    label: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![text(label), control.into()].spacing(10).into()
}

const ZOOM_DEFAULT: f32 = 2.0;
const ZOOM_MIN: f32 = 1.0;
const ZOOM_MAX: f32 = 17.0;
const ZOOM_WHEEL_SCALE: f32 = 0.2;
const ZOOM_PIXELS_FACTOR: f32 = 200.0;
const ITERS_DEFAULT: u32 = 20;
const ITERS_MIN: u32 = 20;
const ITERS_MAX: u32 = 200;

const CENTER_DEFAULT: Vec2 = Vec2::new(-1.5, 0.0);
