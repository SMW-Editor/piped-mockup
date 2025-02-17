use std::sync::Arc;
use std::sync::RwLock;

use glam::Vec2;

use iced::widget::canvas::Stroke;
use iced::Color;
use iced::Point;
use iced::Renderer;
use iced::Size;
use iced::{
    advanced::Shell,
    event::Status,
    mouse::{self, Cursor},
    widget::{
        canvas,
        shader::{self, wgpu, wgpu::util::DeviceExt, Event, Viewport},
    },
    Element, Rectangle,
};

// We have to alias the shader element because it has the same name as the iced::widget::shader
// module, and the `self` syntax only imports the module.
use iced::widget::shader as shader_element;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCoords(pub u32, pub u32);

pub struct Component {
    program: Program,
    canvas: CanvasOverlay,
    tile_hovered: Option<TileCoords>,
    tile_mouse_pressed_on: Option<TileCoords>,
}
/// These are messages that parent is expected to want to handle.
#[derive(Debug, Clone, Copy)]
pub enum PublicMessage {
    /// Raised when user presses then releases on the same tile.
    TileClicked(TileCoords),
}
/// Parent of this component should pass this PrivateMessage to the Component::update function, which may return a PublicMessage.
#[derive(Debug, Clone, Copy)]
pub struct PrivateMessage(Message);

#[derive(Debug, Clone, Copy)]
enum Message {
    CursorMoved(TileCoords),
    LeftButtonPressedInside,
    LeftButtonReleasedInside,
    CursorExited,
}

impl Component {
    pub fn new(
        graphics_bytes_arc: Arc<RwLock<Vec<u8>>>,
        tile_instances_arc: Arc<Vec<TileInstance>>,
    ) -> Self {
        Self {
            program: Program {
                graphics_bytes_arc,
                tile_instances_arc,
                lazy_pipeline_arc: Default::default(),
            },
            canvas: CanvasOverlay::new(),
            tile_hovered: None,
            tile_mouse_pressed_on: None,
        }
    }

    pub fn set_tile_instances(&mut self, tile_instances_arc: Arc<Vec<TileInstance>>) {
        self.program.tile_instances_arc = tile_instances_arc;
    }

    pub fn get_tile_instances(&self) -> Arc<Vec<TileInstance>> {
        self.program.tile_instances_arc.clone()
    }

    pub fn update(&mut self, message: PrivateMessage) -> Option<PublicMessage> {
        match message.0 {
            Message::CursorMoved(tile_hovered) => {
                self.tile_hovered = Some(tile_hovered);
                None
            }
            Message::LeftButtonPressedInside => {
                if let Some(tile_hovered) = self.tile_hovered {
                    self.tile_mouse_pressed_on = Some(tile_hovered)
                }
                None
            }
            Message::LeftButtonReleasedInside => {
                if let (Some(tile_mouse_pressed_on), Some(tile_hovered)) =
                    (self.tile_mouse_pressed_on, self.tile_hovered)
                {
                    if tile_mouse_pressed_on == tile_hovered {
                        Some(PublicMessage::TileClicked(tile_hovered))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Message::CursorExited => {
                self.tile_mouse_pressed_on = None;
                None
            }
        }
    }

    pub fn view(&self, dimens_in_tiles: Option<TileCoords>) -> Element<PrivateMessage> {
        use iced::widget::*;

        let instance_count = self.program.tile_instances_arc.len();
        let quad_count = instance_count.div_ceil(4);
        let (quad_columns, quad_rows) = if let Some(dimens_in_tiles) = dimens_in_tiles {
            (dimens_in_tiles.0 / 2, dimens_in_tiles.1 / 2)
        } else {
            (quad_count.min(8) as u32, quad_count.div_ceil(8) as u32)
        };
        let gfx_pixels_per_quad = 16;
        let screen_pixels_per_gfx_pixel = 2;
        let width = (quad_columns * gfx_pixels_per_quad * screen_pixels_per_gfx_pixel) as u16;
        let height = (quad_rows * gfx_pixels_per_quad * screen_pixels_per_gfx_pixel) as u16;

        mouse_area(stack!(
            shader_element(&self.program).width(width).height(height),
            canvas(&self.canvas).width(width).height(height)
        ))
        .on_press(PrivateMessage(Message::LeftButtonPressedInside))
        .on_release(PrivateMessage(Message::LeftButtonReleasedInside))
        .on_exit(PrivateMessage(Message::CursorExited))
        .on_move(|point| {
            PrivateMessage(Message::CursorMoved(TileCoords(
                (point.x / 16.) as u32,
                (point.y / 16.) as u32,
            )))
        })
        .into()
    }
}

struct Program {
    graphics_bytes_arc: Arc<RwLock<Vec<u8>>>,
    tile_instances_arc: Arc<Vec<TileInstance>>,
    lazy_pipeline_arc: LazyPipelineArc,
}
impl shader::Program<PrivateMessage> for Program {
    // This State type is what Iced puts in its widget tree, and passed to the update and draw
    // functions. We aren't using it, as it is initialized using Default, and for now we want to
    // manage our state ourselves in the app model.
    type State = ();
    type Primitive = FrameInfo;

    fn update(
        &self,
        _: &mut Self::State,
        _event: Event,
        _bounds: Rectangle,
        _cursor: Cursor,
        _: &mut Shell<'_, PrivateMessage>,
    ) -> (Status, Option<PrivateMessage>) {
        (Status::Ignored, None)
    }

    fn draw(&self, _: &Self::State, _: mouse::Cursor, _: Rectangle) -> Self::Primitive {
        FrameInfo {
            graphics_bytes_arc: self.graphics_bytes_arc.clone(),
            tile_instances_arc: self.tile_instances_arc.clone(),
            lazy_pipeline_arc: self.lazy_pipeline_arc.clone(),
        }
    }
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    resolution: Vec2,
    padding: u32,
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct TileInstance {
    // Position (in units of the pixelart's pixels)
    pub x: u32,
    pub y: u32,

    // Which graphic bytes to display
    pub id: u32,

    // Which palette row to use for colors
    pub pal: u8,

    // Settings for how to display the graphic
    pub scale: u8,
    pub flags: u16,
}
impl TileInstance {
    pub fn get_tile_coords(&self) -> TileCoords {
        TileCoords(self.x / 8, self.y / 8)
    }
    pub fn move_to_tile_coords(&mut self, tile_coords: TileCoords) {
        self.x = tile_coords.0 * 8;
        self.y = tile_coords.1 * 8;
    }
}

/// Created every frame, and has the ability to set stuff on the pipeline.
#[derive(Debug)]
pub struct FrameInfo {
    graphics_bytes_arc: Arc<RwLock<Vec<u8>>>,
    tile_instances_arc: Arc<Vec<TileInstance>>,
    lazy_pipeline_arc: LazyPipelineArc,
}
impl shader::Primitive for FrameInfo {
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
        // This is how the Iced examples memoize the pipeline. We don't need that as we just use
        // our own component state.
        if !storage.has::<TilemapShaderPipeline>() {
            storage.store(TilemapShaderPipeline::new(
                self.graphics_bytes_arc.clone(),
                device,
                format,
            ));
        }
        let pipeline = storage.get_mut::<TilemapShaderPipeline>().unwrap();
        */
        let mut pipeline_rw = self.lazy_pipeline_arc.write().unwrap();
        let pipeline = pipeline_rw.get_or_insert_with(|| {
            println!(
                "Creating pipeline, this many bytes total: {}",
                self.graphics_bytes_arc.read().unwrap().len()
            );
            TilemapShaderPipeline::new_and_create_wgpu_pipeline(
                device,
                format,
                self.graphics_bytes_arc.clone(),
                self.tile_instances_arc.clone(),
            )
        });
        pipeline.write_uniforms(
            queue,
            &Uniforms {
                resolution: Vec2::new(bounds.width, bounds.height),
                padding: 0,
            },
        );
        pipeline.replace_graphics_buffer_if_needed(device, &self.graphics_bytes_arc);
        pipeline.write_tile_instances_if_needed(device, queue, &self.tile_instances_arc);
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        _storage: &shader::Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        //let pipeline = storage.get::<TilemapShaderPipeline>().unwrap();
        self.lazy_pipeline_arc
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .render(target, encoder, *clip_bounds);
    }
}

type LazyPipelineArc = Arc<RwLock<Option<TilemapShaderPipeline>>>;

/// Created once then memoized. Creates the WGPU pipeline upon construction, and gives us
/// continuing access to the WGPU pipeline later on.
#[derive(Debug)]
struct TilemapShaderPipeline {
    tile_instances_arc: Arc<Vec<TileInstance>>,
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    palette_buffer: wgpu::Buffer,
    graphics_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}
impl TilemapShaderPipeline {
    fn new_and_create_wgpu_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        graphics_bytes_arc: Arc<RwLock<Vec<u8>>>,
        tile_instances_arc: Arc<Vec<TileInstance>>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tilemap shader module"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "tilemap_shader.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tilemap shader pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TileInstance>() as _,
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

        let mut palette = image::open("assets/palette.png").unwrap().to_rgba32f();
        palette
            .as_flat_samples_mut()
            .samples
            .iter_mut()
            .for_each(|c| *c = c.powf(2.2));
        let palette_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tilemap palette buffer"),
            contents: bytemuck::cast_slice(palette.as_flat_samples().samples),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let graphics_buffer = create_graphics_buffer(device, &graphics_bytes_arc.read().unwrap());
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tilemap uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as _,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = create_bind_group(
            &device,
            &pipeline,
            &palette_buffer,
            &graphics_buffer,
            &uniform_buffer,
        );
        let instance_buffer = create_instance_buffer(&device, &tile_instances_arc);

        Self {
            pipeline,
            tile_instances_arc,
            uniform_buffer,
            instance_buffer,
            palette_buffer,
            graphics_buffer,
            bind_group,
        }
    }

    fn write_uniforms(&mut self, queue: &wgpu::Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    fn replace_graphics_buffer_if_needed(
        &mut self,
        device: &wgpu::Device,
        graphics_bytes_rw: &RwLock<Vec<u8>>,
    ) {
        let graphics_bytes = graphics_bytes_rw.read().unwrap();
        // Only updating if size changed for now, since we don't expect the graphics bytes to be edited
        if self.graphics_buffer.size() != graphics_bytes.len() as _ {
            println!("Graphics buffer size changed, creating new one.");
            self.graphics_buffer = create_graphics_buffer(&device, &graphics_bytes);
            self.bind_group = create_bind_group(
                &device,
                &self.pipeline,
                &self.palette_buffer,
                &self.graphics_buffer,
                &self.uniform_buffer,
            );
        }
    }

    fn write_tile_instances_if_needed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tile_instances_arc: &Arc<Vec<TileInstance>>,
    ) {
        if !Arc::ptr_eq(&self.tile_instances_arc, tile_instances_arc) {
            if self.tile_instances_arc.len() != tile_instances_arc.len() {
                println!("Tile instances buffer size changed, creating new one.");

                self.instance_buffer = create_instance_buffer(&device, &tile_instances_arc);
                self.tile_instances_arc = tile_instances_arc.clone();
            } else {
                queue.write_buffer(
                    &self.instance_buffer,
                    0,
                    bytemuck::cast_slice(tile_instances_arc),
                );
            }
        }
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clip_bounds: Rectangle<u32>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("tilemap render pass"),
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
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));

        pass.draw(0..4, 0..self.tile_instances_arc.len() as u32);
    }
}

fn create_graphics_buffer(device: &wgpu::Device, graphics_bytes_arc: &Vec<u8>) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("tilemap graphics buffer"),
        contents: graphics_bytes_arc,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    })
}
fn create_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    palette_buffer: &wgpu::Buffer,
    graphics_buffer: &wgpu::Buffer,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let bind_group_layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("tilemap bind group"),
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
    })
}
fn create_instance_buffer(
    device: &wgpu::Device,
    tile_instances: &Vec<TileInstance>,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("tilemap instance buffer"),
        contents: bytemuck::cast_slice(tile_instances),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    })
}

struct CanvasOverlay {
    canvas_cache: canvas::Cache,
}

impl CanvasOverlay {
    pub fn new() -> Self {
        Self {
            canvas_cache: canvas::Cache::default(),
        }
    }
}

impl<Message> canvas::Program<Message> for CanvasOverlay {
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
            let stroke_width = 4.;
            let half_stroke_width = stroke_width / 2.;
            frame.stroke_rectangle(
                Point::new(32. - half_stroke_width, 32. - half_stroke_width),
                Size::new(16. + stroke_width, 16. + stroke_width),
                Stroke {
                    width: stroke_width,
                    style: Color::new(0.5, 0.5, 0.5, 1.).into(),
                    ..Default::default()
                },
            );
        })]
    }
}
