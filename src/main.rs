mod palette_program;
mod tilemap;

use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use palette_program::Palette;

use iced::{
    application,
    widget::{button, column, container, horizontal_rule, mouse_area, row, vertical_rule, Space},
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
    graphics_files: Vec<(PathBuf, Arc<Vec<u8>>, usize)>,
    all_graphics_bytes: Arc<RwLock<Vec<u8>>>,
}

#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Message {
    FromDisplayedGraphicsFile(tilemap::Message),
    FromPaletteSelector(palette_program::Message),
    GraphicsFileLoaded(Option<(PathBuf, Arc<Vec<u8>>)>),
    DisplayGraphicsFile(usize),
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
            },
            Task::batch([
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/anim.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::GraphicsFileLoaded,
                ),
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
            Message::GraphicsFileLoaded(Some((path, graphics_bytes))) => {
                println!("loaded {path:?}, {:?} bytes", graphics_bytes.len());
                let offset = self.all_graphics_bytes.read().unwrap().len();
                self.graphics_files
                    .push((path, graphics_bytes.clone(), offset));
                self.all_graphics_bytes
                    .write()
                    .unwrap()
                    .extend(graphics_bytes.iter().cloned());

                // (temporary) Wait for all 5 files to load before creating pipeline etc so that we
                // have the full graphics bytes vec.
                if self.graphics_files.len() == 5 {
                    println!("All files loaded");
                    self.displayed_graphics_file_component = Some(tilemap::Component::new(
                        self.all_graphics_bytes.clone(),
                        get_tile_instances_for_graphics_file(&graphics_bytes, offset),
                    ));
                }

                Task::none()
            }
            Message::DisplayGraphicsFile(file_index) => {
                let (_path, graphics_bytes, offset) =
                    self.graphics_files.get(file_index).unwrap().clone();
                if let Some(tilemap_component) = self.displayed_graphics_file_component.as_mut() {
                    tilemap_component.set_tile_instances(get_tile_instances_for_graphics_file(
                        &graphics_bytes,
                        offset,
                    ));
                } else {
                    self.displayed_graphics_file_component = Some(tilemap::Component::new(
                        self.all_graphics_bytes.clone(),
                        get_tile_instances_for_graphics_file(&graphics_bytes, offset),
                    ));
                }
                Task::none()
            }
            Message::FromDisplayedGraphicsFile(tilemap::Message::CursorMoved(_pos)) => Task::none(),
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
                    container(Space::new(Length::Fixed(100.), Length::Fixed(100.)))
                        .style(|theme: &Theme| container::background(theme.palette().primary)),
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
                        button(file.0.file_name().unwrap().to_str().unwrap())
                            .style(button::secondary)
                            .on_press(Message::DisplayGraphicsFile(index))
                            .into()
                    }))
                    .spacing(10)
                    .align_x(Alignment::Center),
                    Space::with_height(Length::FillPortion(1)),
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
    let contents = tokio::fs::read(&path).await.ok()?;
    Some((path, Arc::new(contents)))
}

fn get_tile_instances_for_graphics_file(
    graphics_bytes_in_file: &Vec<u8>,
    byte_offset_in_all_graphics: usize,
) -> Arc<Vec<tilemap::TileInstance>> {
    let tile_offset = (byte_offset_in_all_graphics as u32) / 32;
    let mut tile_instances = vec![];
    for i in 0..(graphics_bytes_in_file.len() / 64) as u32 {
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
