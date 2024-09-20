mod tilemap_program;

use std::{path::PathBuf, sync::Arc};

use tilemap_program::Tilemap;

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
    tilemap: Option<Tilemap>,
    loaded_tilemaps: Vec<(PathBuf, Arc<Vec<u8>>)>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
enum Message {
    TilemapMessage(tilemap_program::Message),
    TilemapLoaded(Option<(PathBuf, Arc<Vec<u8>>)>),
    SelectTilemap((PathBuf, Arc<Vec<u8>>)),
    MouseMovedInPalette(Point),
    MousePressedInPalette,
}
impl App {
    fn new() -> (Self, Task<Message>) {
        (
            App {
                tilemap: None,
                loaded_tilemaps: vec![],
            },
            Task::batch([
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/global.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::TilemapLoaded,
                ),
                Task::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/grass.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::TilemapLoaded,
                ),
            ]),
        )
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TilemapMessage(tilemap_program::Message::CursorMoved(_pos)) => Task::none(),
            Message::TilemapLoaded(Some((path, bytes))) => {
                println!("loaded {path:?}, {:?} bytes", bytes.len());
                self.loaded_tilemaps.push((path, bytes.clone()));

                // Choose the first loaded tilemap to display.
                if self.loaded_tilemaps.len() == 1 {
                    self.tilemap = Some(Tilemap::new(bytes));
                }

                Task::none()
            }
            Message::SelectTilemap((_path, bytes)) => {
                self.tilemap = Some(Tilemap::new(bytes));
                Task::none()
            }
            Message::MouseMovedInPalette(point) => {
                println!("Moved in palette {:?}", point);
                Task::none()
            }
            Message::MousePressedInPalette => {
                println!("Clicked palette");
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
                    mouse_area(
                        image(format!("{}/assets/palette.png", env!("CARGO_MANIFEST_DIR")))
                            .filter_method(image::FilterMethod::Nearest)
                            .width(100)
                            .height(100)
                    )
                    .on_move(Message::MouseMovedInPalette)
                    .on_press(Message::MousePressedInPalette),
                    Space::with_height(Length::FillPortion(1)),
                ]
                .align_x(Alignment::Center)
                .width(Length::FillPortion(1)),
                vertical_rule(2),
                column![
                    heading("Tile Library"),
                    Space::with_height(Length::FillPortion(1)),
                    self.tilemap.as_ref().map_or_else(
                        || container(column![]),
                        |tilemap| container(Element::map(tilemap.view(), Message::TilemapMessage))
                    ),
                    Space::with_height(Length::Fixed(10.)),
                    column(self.loaded_tilemaps.iter().map(|tilemap| {
                        button(tilemap.0.file_name().unwrap().to_str().unwrap())
                            .style(button::secondary)
                            .on_press(Message::SelectTilemap(tilemap.clone()))
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
