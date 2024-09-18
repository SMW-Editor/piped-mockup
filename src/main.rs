mod tilemap_program;

use std::{path::PathBuf, sync::Arc};

use tilemap_program::TilemapWithControls;

use iced::{
    application,
    widget::{button, column, container, image},
    window, Alignment, Element, Length, Settings, Task, Theme,
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
impl App {
    fn new() -> (Self, Task<Message>) {
        (
            App {
                large: false,
                tilemap_with_controls: TilemapWithControls::new(),
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
            Message::ToggleLarge => {
                self.large = !self.large;
                Task::none()
            }
            Message::TilemapMessage(m) => Task::map(
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

                Task::none()
            }
            Message::SelectTilemap(tilemap) => {
                self.tilemap_with_controls.show(Some(tilemap.1));
                Task::none()
            }
            _ => Task::none(),
        }
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
                .align_x(Alignment::Center),
                "Tilemap:",
                Element::map(self.tilemap_with_controls.view(), Message::TilemapMessage),
            ]
            .align_x(Alignment::Center)
            .spacing(20),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}

async fn load_file(path: PathBuf) -> Option<(PathBuf, Arc<Vec<u8>>)> {
    let contents = tokio::fs::read(&path).await.ok()?;
    Some((path, Arc::new(contents)))
}
