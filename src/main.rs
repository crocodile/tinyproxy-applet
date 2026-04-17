use cosmic::app::{Core, Task};
use cosmic::iced::{Length, Subscription};
use cosmic::widget::icon;
use cosmic::{Application, Element};
use std::time::Duration;

const APP_ID: &str = "com.connectedtribe.tinyproxy-applet";
const PROXY_ON_ICON: &[u8] = include_bytes!("../assets/proxy-on-symbolic.svg");
const PROXY_OFF_ICON: &[u8] = include_bytes!("../assets/proxy-off-symbolic.svg");

#[derive(Default)]
struct TinyproxyApplet {
    core: Core,
    running: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Surface(cosmic::surface::Action),
    Tick,
}

impl Application for TinyproxyApplet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        let mut applet = Self { core, running: false };
        // Check immediately on startup
        applet.check_status();
        (applet, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Surface(action) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(action),
                ));
            }
            Message::Tick => self.check_status(),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let icon_bytes = if self.running {
            PROXY_ON_ICON
        } else {
            PROXY_OFF_ICON
        };

        let suggested_size = self.core.applet.suggested_size(false);
        let icon = icon::from_svg_bytes(icon_bytes)
            .icon()
            .width(Length::Fixed(suggested_size.0 as f32))
            .height(Length::Fixed(suggested_size.1 as f32));
        let tooltip = if self.running {
            "Tinyproxy is connected"
        } else {
            "Tinyproxy is not connected"
        };
        let button = self.core.applet.button_from_element(icon, false);

        self.core
            .applet
            .autosize_window(self.core.applet.applet_tooltip(
                button,
                tooltip,
                false,
                Message::Surface,
                None,
            ))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        cosmic::iced::time::every(Duration::from_secs(5)).map(|_| Message::Tick)
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl TinyproxyApplet {
    fn check_status(&mut self) {
        let output = std::process::Command::new("systemctl")
            .args(["is-active", "--quiet", "tinyproxy"])
            .status();
        self.running = matches!(output, Ok(s) if s.success());
    }
}


fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<TinyproxyApplet>(())
}
