use cosmic::app::{Core, Task};
use cosmic::iced::futures::{SinkExt, StreamExt};
use cosmic::iced::{Length, Subscription, stream};
use cosmic::widget::icon;
use cosmic::{Application, Element};
use std::any::TypeId;
use std::future::pending;
use std::time::Duration;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;

const APP_ID: &str = "com.connectedtribe.tinyproxy-applet";
const SERVICE_UNIT: &str = "tinyproxy.service";
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
    StatusChanged(bool),
}

struct SystemdStatusWatcher;

struct WatchedUnit {
    path: OwnedObjectPath,
    proxy: SystemdUnitProxy<'static>,
    active_state_changes: zbus::proxy::PropertyStream<'static, String>,
}

#[zbus::proxy(
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1",
    interface = "org.freedesktop.systemd1.Manager"
)]
trait SystemdManager {
    fn subscribe(&self) -> zbus::Result<()>;
    fn load_unit(&self, name: &str) -> zbus::Result<OwnedObjectPath>;

    #[zbus(signal)]
    fn unit_new(&self, id: &str, unit: OwnedObjectPath) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unit_removed(&self, id: &str, unit: OwnedObjectPath) -> zbus::Result<()>;
}

#[zbus::proxy(
    default_service = "org.freedesktop.systemd1",
    interface = "org.freedesktop.systemd1.Unit"
)]
trait SystemdUnit {
    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;
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
        (Self { core, running: false }, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Surface(action) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(action),
                ));
            }
            Message::StatusChanged(running) => self.running = running,
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
        Subscription::run_with(TypeId::of::<SystemdStatusWatcher>(), |_| {
            stream::channel(10, move |mut output| async move {
                watch_service_status(&mut output).await;
            })
        })
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn is_running_state(active_state: &str) -> bool {
    matches!(active_state, "active" | "reloading" | "refreshing")
}

async fn watch_service_status(
    output: &mut cosmic::iced::futures::channel::mpsc::Sender<Message>,
) {
    loop {
        if let Err(err) = watch_service_status_once(output).await {
            eprintln!("tinyproxy-applet: failed to watch systemd over D-Bus: {err}");
        }

        let _ = output.send(Message::StatusChanged(false)).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn watch_service_status_once(
    output: &mut cosmic::iced::futures::channel::mpsc::Sender<Message>,
) -> zbus::Result<()> {
    let connection = zbus::Connection::system().await?;
    let manager = SystemdManagerProxy::new(&connection).await?;
    let mut last_running = None;

    manager.subscribe().await?;

    let mut unit_new_stream = manager.receive_unit_new().await?;
    let mut unit_removed_stream = manager.receive_unit_removed().await?;
    let mut owner_changed_stream = manager.inner().receive_owner_changed().await?;

    let mut watched_unit = match manager.load_unit(SERVICE_UNIT).await {
        Ok(path) => Some(watch_unit(&connection, path).await?),
        Err(_) => {
            send_status_if_changed(output, &mut last_running, false).await;
            None
        }
    };

    if let Some(unit) = &watched_unit {
        let active_state = unit.proxy.active_state().await?;
        send_status_if_changed(output, &mut last_running, is_running_state(&active_state)).await;
    }

    loop {
        tokio::select! {
            owner_changed = owner_changed_stream.next() => {
                match owner_changed {
                    Some(Some(_)) => {}
                    Some(None) | None => return Ok(()),
                }
            }
            unit_new = unit_new_stream.next() => {
                let Some(unit_new) = unit_new else {
                    return Ok(());
                };

                let args = unit_new.args()?;
                if args.id == SERVICE_UNIT {
                    let unit = watch_unit(&connection, args.unit.clone()).await?;
                    let active_state = unit.proxy.active_state().await?;
                    send_status_if_changed(output, &mut last_running, is_running_state(&active_state)).await;
                    watched_unit = Some(unit);
                }
            }
            unit_removed = unit_removed_stream.next() => {
                let Some(unit_removed) = unit_removed else {
                    return Ok(());
                };

                let args = unit_removed.args()?;
                if args.id == SERVICE_UNIT {
                    let removed_current_unit = watched_unit
                        .as_ref()
                        .map(|unit| unit.path == args.unit)
                        .unwrap_or(true);

                    if removed_current_unit {
                        watched_unit = None;
                        send_status_if_changed(output, &mut last_running, false).await;
                    }
                }
            }
            active_state_change = async {
                match watched_unit.as_mut() {
                    Some(unit) => unit.active_state_changes.next().await,
                    None => pending::<Option<zbus::proxy::PropertyChanged<'static, String>>>().await,
                }
            } => {
                let Some(active_state_change) = active_state_change else {
                    watched_unit = None;
                    send_status_if_changed(output, &mut last_running, false).await;
                    continue;
                };

                let active_state = active_state_change.get().await?;
                send_status_if_changed(output, &mut last_running, is_running_state(&active_state)).await;
            }
        }
    }
}

async fn watch_unit(
    connection: &zbus::Connection,
    path: OwnedObjectPath,
) -> zbus::Result<WatchedUnit> {
    let proxy = SystemdUnitProxy::builder(connection)
        .path(path.clone())?
        .cache_properties(CacheProperties::Yes)
        .build()
        .await?;
    let active_state_changes = proxy.receive_active_state_changed().await;

    Ok(WatchedUnit {
        path,
        proxy,
        active_state_changes,
    })
}

async fn send_status_if_changed(
    output: &mut cosmic::iced::futures::channel::mpsc::Sender<Message>,
    last_running: &mut Option<bool>,
    running: bool,
) {
    if *last_running == Some(running) {
        return;
    }

    *last_running = Some(running);
    let _ = output.send(Message::StatusChanged(running)).await;
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<TinyproxyApplet>(())
}
