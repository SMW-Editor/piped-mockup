mod palette_program;
mod tilemap;

use std::{path::PathBuf, sync::Arc};

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
    graphics_files: Vec<(PathBuf, Arc<Vec<u8>>)>,
}

#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Message {
    FromDisplayedGraphicsFile(tilemap::Message),
    FromPaletteSelector(palette_program::Message),
    GraphicsFileLoaded(Option<(PathBuf, Arc<Vec<u8>>)>),
    DisplayGraphicsFile((PathBuf, Arc<Vec<u8>>)),
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
                self.graphics_files.push((path, graphics_bytes.clone()));

                // Choose the first loaded graphics file to display.
                if self.graphics_files.len() == 1 {
                    self.displayed_graphics_file_component = Some(tilemap::Component::new(
                        graphics_bytes.clone(),
                        get_tile_instances_for_graphics_file(graphics_bytes),
                    ));
                }

                Task::none()
            }
            Message::DisplayGraphicsFile((_path, graphics_bytes)) => {
                self.displayed_graphics_file_component = Some(tilemap::Component::new(
                    graphics_bytes.clone(),
                    get_tile_instances_for_graphics_file(graphics_bytes),
                ));
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
                    column(self.graphics_files.iter().map(|file| {
                        button(file.0.file_name().unwrap().to_str().unwrap())
                            .style(button::secondary)
                            .on_press(Message::DisplayGraphicsFile(file.clone()))
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
    graphics_bytes: Arc<Vec<u8>>,
) -> Arc<Vec<tilemap::TileInstance>> {
    let mut tile_instances = vec![];
    for i in 0..(graphics_bytes.len() / 64) as u32 {
        let tx = i % 8 * 16;
        let ty = i / 8 * 16;
        tile_instances.push(tilemap::TileInstance {
            x: tx,
            y: ty,
            id: i * 4,
            pal: 3,
            scale: 1,
            flags: 0,
        });
        tile_instances.push(tilemap::TileInstance {
            x: tx + 8,
            y: ty,
            id: i * 4 + 1,
            pal: 3,
            scale: 1,
            flags: 0,
        });
        tile_instances.push(tilemap::TileInstance {
            x: tx,
            y: ty + 8,
            id: i * 4 + 2,
            pal: 3,
            scale: 1,
            flags: 0,
        });
        tile_instances.push(tilemap::TileInstance {
            x: tx + 8,
            y: ty + 8,
            id: i * 4 + 3,
            pal: 3,
            scale: 1,
            flags: 0,
        });
    }
    Arc::new(tile_instances)
}
