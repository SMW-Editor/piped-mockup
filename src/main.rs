mod palette_program;
mod tilemap;

use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use palette_program::Palette;

use iced::{
    application,
    widget::{
        button, column, container, horizontal_rule, mouse_area, row, text, vertical_rule, Space,
    },
    window, Alignment, Element, Length, Point, Settings, Task, Theme,
};

fn main() -> iced::Result {
    application("Piped Mockup", App::update, App::view)
        .theme(|_| Theme::Dark)
        .settings(Settings {
            antialiasing: true,
            ..Default::default()
        })
        .window(window::Settings {
            position: window::Position::Centered,
            ..Default::default()
        })
        .run_with(App::new)
}

struct App {
    displayed_graphics_file_component: Option<tilemap::Component>,
    palette_selector: Palette,
    graphics_files: Vec<GraphicsFile>,
    all_graphics_bytes: Arc<RwLock<Vec<u8>>>,
    displayed_block: Option<tilemap::Component>,
    current_quadrant: usize,
}

#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Message {
    FromDisplayedGraphicsFile(tilemap::PrivateMessage),
    FromDisplayedBlock(tilemap::PrivateMessage),
    FromPaletteSelector(palette_program::Message),
    GraphicsFileLoaded(Option<(PathBuf, Arc<Vec<u8>>)>),
    DisplayGraphicsFile(usize),
    LoadMoreGraphicsFiles,
    MouseMovedOverPalette(Point),
    MousePressedOverPalette,
}
impl App {
    fn new() -> (Self, Task<Message>) {
        (
            App {
                displayed_graphics_file_component: None,
                palette_selector: Palette::new(),
                graphics_files: vec![],
                all_graphics_bytes: Arc::new(RwLock::new(vec![])),
                displayed_block: None,
                current_quadrant: 0,
            },
            Task::batch([
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/global.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::GraphicsFileLoaded,
                ),
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/grass.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::GraphicsFileLoaded,
                ),
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/onoff.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::GraphicsFileLoaded,
                ),
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/pswitch.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::GraphicsFileLoaded,
                ),
            ]),
        )
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GraphicsFileLoaded(Some((path, bytes))) => {
                println!("loaded {path:?}, {:?} bytes", bytes.len());
                let file = GraphicsFile {
                    path,
                    bytes: bytes.clone(),
                    offset_in_all_bytes: self.all_graphics_bytes.read().unwrap().len(),
                };

                if self.displayed_graphics_file_component.is_none() {
                    self.displayed_graphics_file_component = Some(tilemap::Component::new(
                        self.all_graphics_bytes.clone(),
                        file.get_tile_instances(),
                    ));
                    self.displayed_block = Some(tilemap::Component::new(
                        self.all_graphics_bytes.clone(),
                        Arc::new(file.get_tile_instances().iter().take(4).cloned().collect()),
                    ));
                }

                self.all_graphics_bytes
                    .write()
                    .unwrap()
                    .extend(bytes.iter().cloned());
                self.graphics_files.push(file);

                Task::none()
            }
            Message::DisplayGraphicsFile(file_index) => {
                let file = self.graphics_files.get(file_index).unwrap();
                if let Some(tilemap_component) = self.displayed_graphics_file_component.as_mut() {
                    tilemap_component.set_tile_instances(file.get_tile_instances());
                } else {
                    self.displayed_graphics_file_component = Some(tilemap::Component::new(
                        self.all_graphics_bytes.clone(),
                        file.get_tile_instances(),
                    ));
                }
                Task::none()
            }
            Message::LoadMoreGraphicsFiles => Task::batch([Task::perform(
                load_file(PathBuf::from(format!(
                    "{}/assets/anim.bin",
                    env!("CARGO_MANIFEST_DIR")
                ))),
                Message::GraphicsFileLoaded,
            )]),
            Message::FromDisplayedGraphicsFile(m) => {
                if let Some(tilemap_component) = self.displayed_graphics_file_component.as_mut() {
                    match tilemap_component.update(m) {
                        Some(tilemap::PublicMessage::TileClicked(tile_coords)) => {
                            println!("Selected {tile_coords:?}");
                            if let Some(displayed_block) = self.displayed_block.as_mut() {
                                displayed_block.set_tile_instances(Arc::new(
                                    displayed_block
                                        .get_tile_instances()
                                        .iter()
                                        .cloned()
                                        .enumerate()
                                        .map(|(i, instance)| {
                                            if i == self.current_quadrant {
                                                let mut instance = tilemap_component
                                                    .get_tile_instances()
                                                    .iter()
                                                    .find(|tile| {
                                                        tile.x / 8 == tile_coords.0
                                                            && tile.y / 8 == tile_coords.1
                                                    })
                                                    .unwrap()
                                                    .clone();
                                                instance.x = ((i as u32) % 2) * 8;
                                                instance.y = ((i as u32) / 2) * 8;
                                                instance
                                            } else {
                                                instance
                                            }
                                        })
                                        .collect(),
                                ));
                                self.current_quadrant = (self.current_quadrant + 1) % 4;
                            }
                        }
                        None => {}
                    }
                }
                Task::none()
            }
            Message::FromPaletteSelector(_) => Task::none(),
            Message::MouseMovedOverPalette(point) => {
                println!("Moved in palette {:?}", point);
                Task::none()
            }
            Message::MousePressedOverPalette => {
                println!("Clicked palette row");
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let heading = |label| container(label).padding(10);
        container(
            row![
                column![heading("Block Library")]
                    .align_x(Alignment::Center)
                    .width(Length::FillPortion(1)),
                vertical_rule(2),
                column![
                    heading("Block"),
                    Space::with_height(Length::FillPortion(1)),
                    self.displayed_block.as_ref().map_or_else(
                        || container(column![]),
                        |displayed_block| container(Element::map(
                            displayed_block.view(),
                            Message::FromDisplayedBlock
                        ))
                    ),
                    Space::with_height(Length::FillPortion(1)),
                    container(text(self.current_quadrant)).padding(10),
                    Space::with_height(Length::FillPortion(1)),
                    horizontal_rule(2),
                    heading("Palette"),
                    Space::with_height(Length::FillPortion(1)),
                    mouse_area(Element::map(
                        self.palette_selector.view(),
                        Message::FromPaletteSelector
                    ),)
                    .on_move(Message::MouseMovedOverPalette)
                    .on_press(Message::MousePressedOverPalette),
                    Space::with_height(Length::FillPortion(1)),
                ]
                .align_x(Alignment::Center)
                .width(Length::FillPortion(1)),
                vertical_rule(2),
                column![
                    heading("Graphics File Library"),
                    Space::with_height(Length::FillPortion(1)),
                    self.displayed_graphics_file_component.as_ref().map_or_else(
                        || container(column![]),
                        |displayed_graphics_file_component| container(Element::map(
                            displayed_graphics_file_component.view(),
                            Message::FromDisplayedGraphicsFile
                        ))
                    ),
                    Space::with_height(Length::Fixed(10.)),
                    column(self.graphics_files.iter().enumerate().map(|(index, file)| {
                        button(file.path.file_name().unwrap().to_str().unwrap())
                            .style(button::secondary)
                            .on_press(Message::DisplayGraphicsFile(index))
                            .into()
                    }))
                    .spacing(10)
                    .align_x(Alignment::Center),
                    Space::with_height(Length::FillPortion(1)),
                    if self.graphics_files.len() < 5 {
                        container(button("Load more").on_press(Message::LoadMoreGraphicsFiles))
                    } else {
                        container(column![])
                    }
                ]
                .align_x(Alignment::Center)
                .width(Length::FillPortion(1)),
            ]
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

async fn load_file(path: PathBuf) -> Option<(PathBuf, Arc<Vec<u8>>)> {
    tokio::fs::read(&path)
        .await
        .ok()
        .map(|contents| (path, Arc::new(contents)))
}

struct GraphicsFile {
    path: PathBuf,
    bytes: Arc<Vec<u8>>,
    offset_in_all_bytes: usize,
}
impl GraphicsFile {
    fn get_tile_instances(&self) -> Arc<Vec<tilemap::TileInstance>> {
        let tile_offset = (self.offset_in_all_bytes as u32) / 32;
        let mut tile_instances = vec![];

        // Each iteration of the below for-loop is a 2x2 grid of 4 tiles which here we will call a
        // quad.
        let bits_per_pixel = 4;
        let bits_per_tile = bits_per_pixel * 8 * 8;
        let bytes_per_tile = bits_per_tile / 8;
        let bytes_per_quad = bytes_per_tile * 4;
        let number_of_quads = self.bytes.len() / bytes_per_quad;

        for i in 0..(number_of_quads) as u32 {
            let tx = i % 8 * 16;
            let ty = i / 8 * 16;
            tile_instances.push(tilemap::TileInstance {
                x: tx,
                y: ty,
                id: tile_offset + i * 4,
                pal: 3,
                scale: 1,
                flags: 0,
            });
            tile_instances.push(tilemap::TileInstance {
                x: tx + 8,
                y: ty,
                id: tile_offset + i * 4 + 1,
                pal: 3,
                scale: 1,
                flags: 0,
            });
            tile_instances.push(tilemap::TileInstance {
                x: tx,
                y: ty + 8,
                id: tile_offset + i * 4 + 2,
                pal: 3,
                scale: 1,
                flags: 0,
            });
            tile_instances.push(tilemap::TileInstance {
                x: tx + 8,
                y: ty + 8,
                id: tile_offset + i * 4 + 3,
                pal: 3,
                scale: 1,
                flags: 0,
            });
        }
        Arc::new(tile_instances)
    }
}
