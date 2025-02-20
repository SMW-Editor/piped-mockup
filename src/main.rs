mod palette;
mod tilemap;

use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use iced::{application, window, Alignment, Element, Length, Point, Settings, Size, Task, Theme};
use tilemap::{TileCoords, TileInstance};

fn main() -> iced::Result {
    application("Piped Mockup", App::update, App::view)
        .theme(|_| Theme::Dark)
        .settings(Settings {
            antialiasing: true,
            ..Default::default()
        })
        .window(window::Settings {
            position: window::Position::Centered,
            size: Size::new(1000., 1000.),
            ..Default::default()
        })
        .run_with(App::new)
}

struct App {
    displayed_graphics_file_component: Option<tilemap::Component>,
    palette_selector: palette::Component,
    graphics_files: Vec<GraphicsFile>,
    all_graphics_bytes: Arc<RwLock<Vec<u8>>>,
    displayed_block_library: Option<tilemap::Component>,
}

#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Message {
    FromDisplayedGraphicsFile(tilemap::Envelope),
    FromDisplayedBlockLibrary(tilemap::Envelope),
    FromPaletteSelector(palette::Envelope),
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
                palette_selector: palette::Component::new(),
                graphics_files: vec![],
                all_graphics_bytes: Arc::new(RwLock::new(vec![])),
                displayed_block_library: None,
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
                        file.layout_all_tile_instances_from_file(
                            self.palette_selector.selected_line,
                        ),
                    ));
                    // Show single block
                    // self.displayed_block_library = Some(tilemap::Component::new(
                    //     self.all_graphics_bytes.clone(),
                    //     Arc::new(file.get_tile_instances().iter().take(4).cloned().collect()),
                    // ));
                    // For now start out the displayed block library with the current size
                    self.displayed_block_library = Some(tilemap::Component::new(
                        self.all_graphics_bytes.clone(),
                        Arc::new(Vec::new()),
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
                if let Some(displayed_graphics_file_component) =
                    self.displayed_graphics_file_component.as_mut()
                {
                    displayed_graphics_file_component.set_tile_instances(
                        file.layout_all_tile_instances_from_file(
                            self.palette_selector.selected_line,
                        ),
                    );
                } else {
                    self.displayed_graphics_file_component = Some(tilemap::Component::new(
                        self.all_graphics_bytes.clone(),
                        file.layout_all_tile_instances_from_file(
                            self.palette_selector.selected_line,
                        ),
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
            Message::FromDisplayedGraphicsFile(envelope) => {
                if let Some(displayed_graphics_file_component) =
                    self.displayed_graphics_file_component.as_mut()
                {
                    match displayed_graphics_file_component.update(envelope) {
                        Some(tilemap::PublicMessage::TileClicked(tile_coords)) => {
                            println!("Selected {tile_coords:?}");
                            displayed_graphics_file_component.set_brush(Some(tile_coords));
                        }
                        None => {}
                    }
                }
                Task::none()
            }
            Message::FromDisplayedBlockLibrary(envelope) => {
                if let Some(displayed_block_library) = self.displayed_block_library.as_mut() {
                    match displayed_block_library.update(envelope) {
                        Some(tilemap::PublicMessage::TileClicked(clicked_tile_coords)) => {
                            if let Some(displayed_graphics_file_component) =
                                self.displayed_graphics_file_component.as_mut()
                            {
                                if let Some(brush) = displayed_graphics_file_component.get_brush() {
                                    println!("Painting {clicked_tile_coords:?} with {brush:?}");
                                    let mut copy_of_tile_from_graphics_file =
                                        displayed_graphics_file_component
                                            .get_tile_instances()
                                            .iter()
                                            .find(|tile| tile.get_tile_coords() == brush)
                                            .unwrap()
                                            .clone();
                                    copy_of_tile_from_graphics_file
                                        .move_to_tile_coords(clicked_tile_coords);
                                    displayed_block_library.set_tile_instances(Arc::new({
                                        let mut found = false;
                                        let mut new_tile_instances_for_block_library =
                                            displayed_block_library
                                                .get_tile_instances()
                                                .iter()
                                                .cloned()
                                                .map(|tile_in_block_library| {
                                                    if tile_in_block_library.get_tile_coords()
                                                        == clicked_tile_coords
                                                    {
                                                        found = true;
                                                        copy_of_tile_from_graphics_file
                                                    } else {
                                                        tile_in_block_library
                                                    }
                                                })
                                                .collect::<Vec<TileInstance>>();
                                        if !found {
                                            new_tile_instances_for_block_library
                                                .push(copy_of_tile_from_graphics_file);
                                        }
                                        new_tile_instances_for_block_library
                                    }));
                                }
                            }
                        }
                        None => {}
                    }
                }
                Task::none()
            }
            Message::FromPaletteSelector(envelope) => {
                match self.palette_selector.update(envelope) {
                    Some(palette::PublicMessage::PaletteLineClicked(line)) => {
                        println!("PaletteLineClicked({line:?}");
                        self.palette_selector.selected_line = line;

                        if let Some(displayed_graphics_file_component) =
                            self.displayed_graphics_file_component.as_mut()
                        {
                            displayed_graphics_file_component.set_tile_instances(Arc::new(
                                displayed_graphics_file_component
                                    .get_tile_instances()
                                    .iter()
                                    .cloned()
                                    .map(|tile| {
                                        let mut new_tile = tile.clone();
                                        new_tile.pal = line as u8;
                                        new_tile
                                    })
                                    .collect::<Vec<TileInstance>>(),
                            ));
                        }
                    }
                    None => {}
                }
                Task::none()
            }
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
        use iced::widget::{column, row, *};
        let heading = |label| container(label).padding(10);
        container(
            row![
                column![
                    heading("Block Library"),
                    Space::with_height(Length::FillPortion(1)),
                    self.displayed_block_library.as_ref().map_or_else(
                        || container(column![]),
                        |displayed_block_library| container(Element::map(
                            displayed_block_library.view(Some(TileCoords(32, 32))),
                            Message::FromDisplayedBlockLibrary
                        ))
                    ),
                    Space::with_height(Length::FillPortion(1)),
                    horizontal_rule(2),
                    heading("Palette"),
                    Space::with_height(Length::FillPortion(1)),
                    Element::map(self.palette_selector.view(), Message::FromPaletteSelector),
                    Space::with_height(Length::FillPortion(1)),
                ]
                .align_x(Alignment::Center)
                .width(Length::FillPortion(1)),
                vertical_rule(2),
                column![
                    heading("Graphics File"),
                    Space::with_height(Length::FillPortion(1)),
                    self.displayed_graphics_file_component.as_ref().map_or_else(
                        || container(column![]),
                        |displayed_graphics_file_component| container(Element::map(
                            displayed_graphics_file_component.view(None),
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
    fn layout_all_tile_instances_from_file(
        &self,
        palette_line: usize,
    ) -> Arc<Vec<tilemap::TileInstance>> {
        let pal = palette_line as u8;
        let mut tile_instances = vec![];

        // Each iteration of the below for-loop is a 2x2 grid of 4 tiles which here we will call a
        // quad.
        let bits_per_pixel = 4;
        let bits_per_tile = bits_per_pixel * 8 * 8;
        let bytes_per_tile = bits_per_tile / 8; // it's 32
        let bytes_per_quad = bytes_per_tile * 4;
        let number_of_quads_in_this_file = self.bytes.len() / bytes_per_quad;

        // If the all-bytes array was an all-tiles array, the following number would be the index
        // of the first tile in this file.
        let first_tile_id_of_file = (self.offset_in_all_bytes / bytes_per_tile) as u32;

        let quads_per_row = 8;

        for quad_index in 0..(number_of_quads_in_this_file) as u32 {
            // These are in units of the visible pixels in the tile
            let quad_left_x = quad_index % quads_per_row * 16;
            let quad_top_y = quad_index / quads_per_row * 16;

            let first_tile_id_of_quad = first_tile_id_of_file + quad_index * 4;

            tile_instances.push(tilemap::TileInstance {
                x: quad_left_x,
                y: quad_top_y,
                id: first_tile_id_of_quad,
                pal,
                scale: 1,
                flags: 0,
            });
            tile_instances.push(tilemap::TileInstance {
                x: quad_left_x + 8,
                y: quad_top_y,
                id: first_tile_id_of_quad + 1,
                pal,
                scale: 1,
                flags: 0,
            });
            tile_instances.push(tilemap::TileInstance {
                x: quad_left_x,
                y: quad_top_y + 8,
                id: first_tile_id_of_quad + 2,
                pal,
                scale: 1,
                flags: 0,
            });
            tile_instances.push(tilemap::TileInstance {
                x: quad_left_x + 8,
                y: quad_top_y + 8,
                id: first_tile_id_of_quad + 3,
                pal,
                scale: 1,
                flags: 0,
            });
        }
        Arc::new(tile_instances)
    }
}
