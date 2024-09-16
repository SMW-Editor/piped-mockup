mod tilemap_program;

use glam::Vec2;
use tilemap_program::TilemapProgram;

use iced::{
    executor,
    widget::{button, column, container, image, row, shader, slider, text},
    window, Alignment, Application, Color, Command, Element, Length, Rectangle, Settings, Theme,
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
    tilemap_program: TilemapProgram,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    ToggleLarge,
    UpdateMaxIterations(u32),
    UpdateZoom(f32),
    PanningDelta(Vec2),
    ZoomDelta(Vec2, Rectangle, f32),
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
                tilemap_program: TilemapProgram::new(),
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

            Message::UpdateMaxIterations(max_iter) => {
                self.tilemap_program.controls.max_iter = max_iter;
                Command::none()
            }
            Message::UpdateZoom(zoom) => {
                self.tilemap_program.controls.zoom = zoom;
                Command::none()
            }
            Message::PanningDelta(delta) => {
                self.tilemap_program.controls.center -=
                    2.0 * delta * self.tilemap_program.controls.scale();
                Command::none()
            }
            Message::ZoomDelta(pos, bounds, delta) => {
                let delta = delta * ZOOM_WHEEL_SCALE;
                let prev_scale = self.tilemap_program.controls.scale();
                let prev_zoom = self.tilemap_program.controls.zoom;
                self.tilemap_program.controls.zoom =
                    (prev_zoom + delta).max(ZOOM_MIN).min(ZOOM_MAX);

                let vec = pos - Vec2::new(bounds.width, bounds.height) * 0.5;
                let new_scale = self.tilemap_program.controls.scale();
                self.tilemap_program.controls.center += vec * (prev_scale - new_scale) * 2.0;
                Command::none()
            }
        }
    }

    fn title(&self) -> String {
        "Piped Mockup".into()
    }
    fn theme(&self) -> Theme {
        Theme::Dark
    }
    fn view(&self) -> Element<Message> {
        let shader_controls = row![
            control(
                "Max iterations",
                slider(
                    ITERS_MIN..=ITERS_MAX,
                    self.tilemap_program.controls.max_iter,
                    move |iter| { Message::UpdateMaxIterations(iter) }
                )
                .width(Length::Fill)
            ),
            control(
                "Zoom",
                slider(
                    ZOOM_MIN..=ZOOM_MAX,
                    self.tilemap_program.controls.zoom,
                    move |zoom| { Message::UpdateZoom(zoom) }
                )
                .step(0.01)
                .width(Length::Fill)
            ),
        ];
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
                shader(&self.tilemap_program).width(400).height(400),
                shader_controls,
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

fn control<'a>(
    label: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![text(label), control.into()].spacing(10).into()
}

const ZOOM_MIN: f32 = 1.0;
const ZOOM_MAX: f32 = 17.0;
const ZOOM_WHEEL_SCALE: f32 = 0.2;
const ITERS_MIN: u32 = 20;
const ITERS_MAX: u32 = 200;
