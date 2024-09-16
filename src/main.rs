mod tilemap_program;

use std::{path::PathBuf, sync::Arc};

use tilemap_program::TilemapWithControls;

use iced::{
    executor,
    widget::{button, column, container, image},
    window, Alignment, Application, Color, Command, Element, Length, Settings, Theme,
};

fn main() -> iced::Result {
    App::run(Settings {
        antialiasing: true,
        window: window::Settings {
            position: window::Position::Centered,
            ..Default::default()
        },
        ..Default::default()
    })
}

struct App {
    large: bool,
    tilemap_with_controls: TilemapWithControls,
    loaded_tilemaps: Vec<(PathBuf, Arc<Vec<u8>>)>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
enum Message {
    ToggleLarge,
    TilemapMessage(tilemap_program::Message),
    TilemapLoaded(Option<(PathBuf, Arc<Vec<u8>>)>),
    SelectTilemap((PathBuf, Arc<Vec<u8>>)),
}

impl Application for App {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_: Self::Flags) -> (Self, Command<Message>) {
        (
            App {
                large: false,
                tilemap_with_controls: TilemapWithControls::new(),
                loaded_tilemaps: vec![],
            },
            Command::batch([
                Command::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/global.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::TilemapLoaded,
                ),
                Command::perform(
                    load_file(PathBuf::from(format!(
                        "{}/assets/grass.bin",
                        env!("CARGO_MANIFEST_DIR")
                    ))),
                    Message::TilemapLoaded,
                ),
            ]),
        )
    }
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleLarge => {
                self.large = !self.large;
                Command::none()
            }
            Message::TilemapMessage(m) => Command::map(
                self.tilemap_with_controls.update(m),
                Message::TilemapMessage,
            ),
            Message::TilemapLoaded(Some((path, bytes))) => {
                println!("loaded {path:?}, {:?} bytes", bytes.len());
                self.loaded_tilemaps.push((path, bytes.clone()));

                // Choose the first loaded tilemap to display.
                if self.loaded_tilemaps.len() == 1 {
                    self.tilemap_with_controls.show(Some(bytes));
                }

                Command::none()
            }
            Message::SelectTilemap(tilemap) => {
                self.tilemap_with_controls.show(Some(tilemap.1));
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn title(&self) -> String {
        "Piped Mockup".into()
    }
    fn theme(&self) -> Theme {
        Theme::Dark
    }
    fn view(&self) -> Element<Message> {
        container(
            column![
                "Palette:",
                Element::from(
                    image(format!("{}/assets/palette.png", env!("CARGO_MANIFEST_DIR")))
                        .filter_method(image::FilterMethod::Nearest)
                        .width(Length::Fill)
                        .height(if self.large { 200 } else { 100 })
                ),
                button("Toggle Large").on_press(Message::ToggleLarge),
                "Available Tilemaps:",
                column(self.loaded_tilemaps.iter().map(|tilemap| {
                    button(tilemap.0.file_name().unwrap().to_str().unwrap())
                        .on_press(Message::SelectTilemap(tilemap.clone()))
                        .into()
                }))
                .align_items(Alignment::Center),
                "Tilemap:",
                Element::map(self.tilemap_with_controls.view(), Message::TilemapMessage),
            ]
            .align_items(Alignment::Center)
            .spacing(20),
        )
        .center_x()
        .center_y()
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}

async fn load_file(path: PathBuf) -> Option<(PathBuf, Arc<Vec<u8>>)> {
    let contents = tokio::fs::read(&path).await.ok()?;
    Some((path, Arc::new(contents)))
}
