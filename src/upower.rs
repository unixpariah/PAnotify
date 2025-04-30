use crate::Event;
use futures_lite::StreamExt;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{fmt::Display, sync::mpsc};
use zbus::{proxy, zvariant::OwnedValue};

#[derive(
    PartialEq, Eq, OwnedValue, Deserialize_repr, Serialize_repr, Default, Hash, Clone, Copy,
)]
#[repr(u32)]
pub enum BatteryState {
    #[default]
    Unknown = 0,
    Charging = 1,
    Discharging = 2,
    Empty = 3,
    FullyCharged = 4,
    PendingCharge = 5,
    PendingDischarge = 6,
}

impl Display for BatteryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BatteryState::Unknown => "unknown",
            BatteryState::Charging => "charging",
            BatteryState::Discharging => "discharging",
            BatteryState::Empty => "empty",
            BatteryState::FullyCharged => "fullycharged",
            BatteryState::PendingCharge => "pendingcharge",
            BatteryState::PendingDischarge => "pendingdischarge",
        };
        write!(f, "{}", s)
    }
}

#[derive(
    PartialEq, Eq, OwnedValue, Deserialize_repr, Serialize_repr, Default, Hash, Clone, Copy,
)]
#[repr(u32)]
pub enum BatteryLevel {
    #[default]
    Unknown = 0,
    None = 1,
    Low = 3,
    Critical = 4,
    Normal = 6,
    High = 7,
    Full = 8,
}

impl Display for BatteryLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BatteryLevel::Unknown => "unknown",
            BatteryLevel::None => "none",
            BatteryLevel::Low => "low",
            BatteryLevel::Critical => "critical",
            BatteryLevel::Normal => "normal",
            BatteryLevel::High => "high",
            BatteryLevel::Full => "full",
        };

        write!(f, "{}", s)
    }
}

#[proxy(interface = "org.freedesktop.UPower", assume_defaults = true)]
trait UPower {
    #[zbus(property)]
    fn on_battery(&self) -> zbus::Result<bool>;

    #[zbus(object = "Device")]
    fn get_display_device(&self);
}

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower",
    assume_defaults = false
)]
trait Device {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn battery_level(&self) -> zbus::Result<BatteryLevel>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<BatteryState>;
}

fn handle_state(event_sender: &mpsc::Sender<Event>, value: BatteryState) {
    _ = event_sender.send(Event::BatteryState(value));
}

fn handle_battery_level(event_sender: &mpsc::Sender<Event>, value: BatteryLevel) {
    _ = event_sender.send(Event::BatteryLevel(value));
}

fn handle_on_battery(event_sender: &mpsc::Sender<Event>, value: bool) {
    _ = event_sender.send(Event::OnBattery(value));
}

fn handle_battery_percentage(event_sender: &mpsc::Sender<Event>, value: f64) {
    _ = event_sender.send(Event::BatteryPercentage(value as u64));
}

pub struct BatteryManager {
    connection: zbus::Connection,
    percentage: u64,
}

impl BatteryManager {
    pub async fn new() -> anyhow::Result<Self> {
        let connection = zbus::Connection::system().await?;

        Ok(Self {
            connection,
            percentage: 0,
        })
    }

    pub fn set_battery(&mut self, percentage: u64) {
        self.percentage = percentage;
    }

    pub fn get_battery(&self) -> u64 {
        self.percentage
    }

    pub async fn subscribe(&mut self, event_sender: mpsc::Sender<Event>) -> anyhow::Result<()> {
        let upower = UPowerProxy::new(&self.connection).await?;
        let device = upower.get_display_device().await?;

        self.percentage = device.percentage().await? as u64;

        {
            let percentage = device.percentage().await?;
            handle_battery_percentage(&event_sender, percentage);

            let mut percentage_stream = device.receive_percentage_changed().await;

            let event_sender = event_sender.clone();
            tokio::spawn(async move {
                while let Some(event) = percentage_stream.next().await {
                    if let Ok(percentage) = event.get().await {
                        handle_battery_percentage(&event_sender, percentage);
                    }
                }
            });
        }

        {
            let mut on_battery_stream = upower.receive_on_battery_changed().await;
            let event_sender = event_sender.clone();
            if let Ok(on_battery) = upower.on_battery().await {
                handle_on_battery(&event_sender, on_battery);
            }

            tokio::spawn(async move {
                while let Some(event) = on_battery_stream.next().await {
                    if let Ok(on_battery) = event.get().await {
                        handle_on_battery(&event_sender, on_battery);
                    }
                }
            });
        }

        {
            let state = device.state().await?;
            handle_state(&event_sender, state);

            let mut state_stream = device.receive_state_changed().await;

            let event_sender = event_sender.clone();
            tokio::spawn(async move {
                while let Some(event) = state_stream.next().await {
                    if let Ok(state) = event.get().await {
                        handle_state(&event_sender, state);
                    }
                }
            });
        }

        let level = device.battery_level().await?;
        handle_battery_level(&event_sender, level);

        let mut level_stream = device.receive_battery_level_changed().await;

        let event_sender = event_sender.clone();
        tokio::spawn(async move {
            while let Some(event) = level_stream.next().await {
                if let Ok(level) = event.get().await {
                    handle_battery_level(&event_sender, level);
                }
            }
        });

        Ok(())
    }
}
