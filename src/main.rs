mod notify;
mod pulse;

use libpulse_binding::context::subscribe::{Facility, InterestMaskSet};
use notify::notify;
use std::collections::HashMap;
use std::sync::mpsc;

struct PAnotify<'a> {
    pulse: pulse::PulseManager,
    notifier: Notifier<'a>,
    event_channel: mpsc::Receiver<Event>,
}

impl PAnotify<'_> {
    async fn new() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();

        let mut pulse = pulse::PulseManager::new()?;
        pulse.set_subscription_callback(move |facility, _, _| {
            let event = match facility {
                Some(Facility::Sink) => Some(Event::VolumeChanged),
                Some(Facility::Card) => Some(Event::DefaultDeviceChanged),
                _ => None,
            };

            if let Some(event) = event {
                let _ = tx.send(event);
            }
        });

        pulse.subscribe(InterestMaskSet::SINK | InterestMaskSet::CARD);

        Ok(Self {
            event_channel: rx,
            pulse,
            notifier: Notifier::new().await?,
        })
    }

    async fn run(mut self) -> anyhow::Result<()> {
        loop {
            match self.event_channel.recv() {
                Ok(Event::VolumeChanged) => {
                    let volume = self.pulse.get_default_sink_volume()?.max().print();
                    let volume_value = volume.trim_end_matches('%').trim().parse()?;
                    self.notifier.send_volume_notification(volume_value).await?;
                }
                Ok(Event::DefaultDeviceChanged) => {
                    self.notifier.send_device_change_notification().await?;
                }
                Err(_) => break,
            }
        }
        Ok(())
    }
}

struct Notifier<'a> {
    builder: notify::NotificationBuilder<'a>,
    active_notifications: HashMap<Event, u32>,
}

impl<'a> Notifier<'a> {
    async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            builder: notify().await?,
            active_notifications: HashMap::new(),
        })
    }

    async fn send_volume_notification(&mut self, volume: u32) -> anyhow::Result<()> {
        let id = self
            .active_notifications
            .get(&Event::VolumeChanged)
            .unwrap_or(&0);
        let new_id = self
            .builder
            .clone()
            .with_id(*id)
            .with_summary(&format!("Volume [ {}% ]", volume))
            .with_progress(volume as i32)
            .send()
            .await?;

        self.active_notifications
            .insert(Event::VolumeChanged, new_id);
        Ok(())
    }

    async fn send_device_change_notification(&mut self) -> anyhow::Result<()> {
        let id = self
            .active_notifications
            .get(&Event::DefaultDeviceChanged)
            .unwrap_or(&0);
        let new_id = self
            .builder
            .clone()
            .with_urgency(notify::Urgency::Normal)
            .with_summary("Device changed")
            .with_id(*id)
            .send()
            .await?;

        self.active_notifications
            .insert(Event::DefaultDeviceChanged, new_id);
        Ok(())
    }
}

#[derive(PartialEq, Eq, Hash)]
enum Event {
    VolumeChanged,
    DefaultDeviceChanged,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let panotify = PAnotify::new().await?;
    panotify.run().await?;

    Ok(())
}
