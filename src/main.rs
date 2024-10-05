mod palette_program;
mod tilemap;

use std::{path::PathBuf, sync::Arc};

use palette_program::Palette;

use iced::{
    application,
    widget::{
        button, column, container, horizontal_rule, image, mouse_area, row, vertical_rule, Space,
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
    displayed_tilemap: Option<tilemap::Component>,
    palette: Palette,
    tilemap_files: Vec<(PathBuf, Arc<Vec<u8>>)>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
enum Message {
    FromDisplayedTilemap(tilemap::Message),
    FromPalette(palette_program::Message),
    TilemapFileLoaded(Option<(PathBuf, Arc<Vec<u8>>)>),
    DisplayTilemapFile((PathBuf, Arc<Vec<u8>>)),
    MouseMovedOverPalette(Point),
    MousePressedOverPalette,
}
impl App {
    fn new() -> (Self, Task<Message>) {
        (
            App {
                displayed_tilemap: None,
                palette: Palette::new(),
                tilemap_files: vec![],
            },
            Task::batch([
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/global.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::TilemapFileLoaded,
                ),
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/grass.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::TilemapFileLoaded,
                ),
            ]),
        )
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FromDisplayedTilemap(tilemap::Message::CursorMoved(_pos)) => Task::none(),
            Message::TilemapFileLoaded(Some((path, bytes))) => {
                println!("loaded {path:?}, {:?} bytes", bytes.len());
                self.tilemap_files.push((path, bytes.clone()));

                // Choose the first loaded tilemap to display.
                if self.tilemap_files.len() == 1 {
                    self.displayed_tilemap = Some(tilemap::Component::new(bytes));
                }

                Task::none()
            }
            Message::DisplayTilemapFile((_path, bytes)) => {
                self.displayed_tilemap = Some(tilemap::Component::new(bytes));
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
                    mouse_area(Element::map(self.palette.view(), Message::FromPalette),)
                        .on_move(Message::MouseMovedOverPalette)
                        .on_press(Message::MousePressedOverPalette),
                    Space::with_height(Length::FillPortion(1)),
                ]
                .align_x(Alignment::Center)
                .width(Length::FillPortion(1)),
                vertical_rule(2),
                column![
                    heading("Tile Library"),
                    Space::with_height(Length::FillPortion(1)),
                    self.displayed_tilemap.as_ref().map_or_else(
                        || container(column![]),
                        |displayed_tilemap| container(Element::map(
                            displayed_tilemap.view(),
                            Message::FromDisplayedTilemap
                        ))
                    ),
                    Space::with_height(Length::Fixed(10.)),
                    column(self.tilemap_files.iter().map(|tilemap| {
                        button(tilemap.0.file_name().unwrap().to_str().unwrap())
                            .style(button::secondary)
                            .on_press(Message::DisplayTilemapFile(tilemap.clone()))
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
