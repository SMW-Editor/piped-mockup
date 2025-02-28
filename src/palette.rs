use std::sync::Arc;
use std::sync::RwLock;

use glam::Vec2;

use iced::widget::canvas;
use iced::widget::canvas::Path;
use iced::widget::stack;
use iced::Color;
use iced::Point;
use iced::Renderer;
use iced::Size;
use iced::{
    advanced::Shell,
    event::Status,
    mouse::{self, Cursor},
    widget::shader::{self, wgpu, wgpu::util::DeviceExt, Event, Viewport},
    Element, Rectangle,
};

// We have to alias the shader element because it has the same name as the iced::widget::shader module, and the `self` syntax only imports the module.
use iced::widget::shader as shader_element;

const PALETTE_ROWS: usize = 16;

#[derive(Debug, Clone, Copy)]
pub enum PublicMessage {
    /// Raised when user presses then releases on the same palette line
    PaletteLineClicked(usize),
}

/// Parent of this component should pass this Envelope to the Component::update function, which may return a PublicMessage.
#[derive(Debug, Clone, Copy)]
pub struct Envelope(PrivateMessage);

#[derive(Debug, Clone, Copy)]
enum PrivateMessage {
    CursorMovedOverLine(usize),
    LeftButtonPressedInside,
    LeftButtonReleasedInside,
    CursorExited,
}

pub struct Component {
    pub selected_line: usize,
    palette_program: PaletteProgram,
    overlay: PaletteCanvasOverlay,
    line_hovered: Option<usize>,
    line_mouse_pressed_on: Option<usize>,
}
impl Component {
    pub fn new() -> Self {
        Self {
            selected_line: 3,
            palette_program: PaletteProgram::new(),
            overlay: PaletteCanvasOverlay::new(),
            line_hovered: None,
            line_mouse_pressed_on: None,
        }
    }

    pub fn update(&mut self, envelope: Envelope) -> Option<PublicMessage> {
        match envelope.0 {
            PrivateMessage::CursorMovedOverLine(line) => {
                self.line_hovered = Some(line);
                None
            }
            PrivateMessage::LeftButtonPressedInside => {
                self.line_mouse_pressed_on = self.line_hovered;
                None
            }
            PrivateMessage::LeftButtonReleasedInside => {
                if let (Some(line_mouse_pressed_on), Some(line_hovered)) =
                    (self.line_mouse_pressed_on, self.line_hovered)
                {
                    if line_mouse_pressed_on == line_hovered {
                        Some(PublicMessage::PaletteLineClicked(line_hovered))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            PrivateMessage::CursorExited => {
                self.line_hovered = None;
                self.line_mouse_pressed_on = None;
                None
            }
        }
    }

    pub fn view(&self) -> Element<Envelope> {
        use iced::widget::*;

        let dim = 256;
        mouse_area(stack!(
            shader_element(&self.palette_program).width(dim).height(dim),
            canvas(&self.overlay).width(dim).height(dim)
        ))
        .on_press(Envelope(PrivateMessage::LeftButtonPressedInside))
        .on_release(Envelope(PrivateMessage::LeftButtonReleasedInside))
        .on_exit(Envelope(PrivateMessage::CursorExited))
        .on_move(move |point| {
            println!("point: {point:?}");
            Envelope(PrivateMessage::CursorMovedOverLine(
                ((point.y / dim as f32) * PALETTE_ROWS as f32) as _,
            ))
        })
        .into()
    }
}

type LazyPipelineArc = Arc<RwLock<Option<PaletteShaderPipeline>>>;

struct PaletteProgram {
    pipeline: LazyPipelineArc,
}
impl PaletteProgram {
    fn new() -> Self {
        Self {
            pipeline: Default::default(),
        }
    }
}
impl shader::Program<Envelope> for PaletteProgram {
    type State = ();
    type Primitive = PaletteFrameInfo;

    fn update(
        &self,
        _state: &mut Self::State,
        _event: Event,
        _bounds: Rectangle,
        _cursor: Cursor,
        _shell: &mut Shell<'_, Envelope>,
    ) -> (Status, Option<Envelope>) {
        (Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        PaletteFrameInfo {
            pipeline: self.pipeline.clone(),
        }
    }
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    resolution: Vec2,
    padding: u32,
}

/// Created every frame, and has the ability to set stuff on the pipeline.
#[derive(Debug)]
pub struct PaletteFrameInfo {
    pipeline: LazyPipelineArc,
}
impl shader::Primitive for PaletteFrameInfo {
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
        let mut pipeline = self.pipeline.write().unwrap();
        let pipeline = pipeline.get_or_insert_with(|| PaletteShaderPipeline::new(device, format));
        pipeline.write_uniforms(
            queue,
            &Uniforms {
                resolution: Vec2::new(bounds.width, bounds.height),
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
        self.pipeline
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .render(target, encoder, *clip_bounds);
    }
}

#[derive(Debug)]
struct PaletteShaderPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl PaletteShaderPipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("palette shader module"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "palette_shader.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("palette render pipeline"),
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

        let mut palette_image = image::open("assets/palette.png").unwrap().to_rgba32f();
        palette_image
            .as_flat_samples_mut()
            .samples
            .iter_mut()
            .for_each(|c| *c = c.powf(2.2));
        let palette_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("palette palette buffer"),
            contents: bytemuck::cast_slice(palette_image.as_flat_samples().samples),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("palette bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: palette_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            bind_group,
        }
    }

    fn write_uniforms(&mut self, _queue: &wgpu::Queue, _uniforms: &Uniforms) {}

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

struct PaletteCanvasOverlay {
    pub canvas_cache: canvas::Cache,
}
impl PaletteCanvasOverlay {
    pub fn new() -> Self {
        Self {
            canvas_cache: canvas::Cache::default(),
        }
    }

    fn get_hatched_path(top_left: Point, size: Size) -> Path {
        let hatch_width = 8f32;
        let hatch_count_horizontal = (size.width / hatch_width / 2.).ceil() as usize;
        let hatch_count_vertical = (size.height / hatch_width / 2.).ceil() as usize;

        let top = top_left.y;
        let left = top_left.x;
        let right = left + size.width;
        let bottom = top + size.height;

        Path::new(|b| {
            for i in 0..hatch_count_horizontal {
                let i = i as f32;
                let hatch_start_x = left + i * 2. * hatch_width;
                b.move_to(Point::new(hatch_start_x, top));
                b.line_to(Point::new(hatch_start_x + hatch_width, top));
                b.line_to(Point::new(
                    hatch_start_x + size.height + hatch_width,
                    bottom,
                ));
                b.line_to(Point::new(hatch_start_x + size.height, bottom));
                b.close();
            }
            for i in 0..hatch_count_vertical {
                let i = i as f32;
                let hatch_start_y = top + (1. + i * 2.) * hatch_width;
                b.move_to(Point::new(left, hatch_start_y));
                b.line_to(Point::new(left, hatch_start_y + hatch_width));
                b.line_to(Point::new(right, hatch_start_y + size.width + hatch_width));
                b.line_to(Point::new(right, hatch_start_y + size.width));
                b.close();
            }
        })
    }
}
impl<Message> canvas::Program<Message> for PaletteCanvasOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        vec![self.canvas_cache.draw(renderer, bounds.size(), |frame| {
            frame.fill(
                // Subtract 2 in order to get the hatched paths to more accurately position
                // themselves over the pixels they're supposed to be covering, since the canvas can
                // shift relative to the shader element depending on final calculated layout
                // position.
                &Self::get_hatched_path(
                    Point::new(-2., bounds.height / 2. - 2.),
                    Size::new(bounds.width + 2., bounds.height / 2. + 2.),
                ),
                Color::new(0.1, 0.1, 0.1, 1.0),
            );
        })]
    }
}
