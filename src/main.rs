mod tilemap_program;

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
}

#[derive(Debug, Clone, Copy)]
enum Message {
    ToggleLarge,
    TilemapMessage(tilemap_program::Message),
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
            },
            Command::none(),
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
        }
    }

    fn title(&self) -> String {
        "Piped Mockup".into()
    }
    fn theme(&self) -> Theme {
        Theme::Dark
    }
    fn view(&self) -> Element<Message> {
        let element: Element<Message> = container(
            column![
                "Palette:",
                Element::from(
                    image(format!("{}/assets/palette.png", env!("CARGO_MANIFEST_DIR")))
                        .filter_method(image::FilterMethod::Nearest)
                        .width(Length::Fill)
                        .height(if self.large { 200 } else { 100 })
                ),
                button("Toggle Large").on_press(Message::ToggleLarge),
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
        .into();
        element.explain(Color::BLACK)
    }
}
