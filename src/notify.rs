use std::collections::HashMap;

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, zbus::zvariant::Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

#[derive(Clone, Copy)]
pub enum Urgency {
    Low = 0,
    Normal = 1,
    Critical = 2,
}

#[derive(Clone)]
pub struct NotificationBuilder<'a> {
    proxy: NotificationsProxy<'a>,
    summary: &'a str,
    body: &'a str,
    progress: Option<i32>,
    icon: &'a str,
    urgency: Urgency,
    id: u32,
}

pub async fn notify<'a>() -> zbus::Result<NotificationBuilder<'a>> {
    let conn = zbus::Connection::session().await?;
    Ok(NotificationBuilder {
        summary: "",
        body: "",
        progress: None,
        icon: "",
        urgency: Urgency::Low,
        proxy: NotificationsProxy::new(&conn).await?,
        id: 0,
    })
}

impl<'a> NotificationBuilder<'a> {
    pub fn with_progress(mut self, value: i32) -> Self {
        self.progress = Some(value);
        self
    }

    pub fn with_summary(mut self, summary: &'a str) -> Self {
        self.summary = summary;
        self
    }

    pub fn with_body(mut self, body: &'a str) -> Self {
        self.body = body;
        self
    }

    pub fn with_urgency(mut self, urgency: Urgency) -> Self {
        self.urgency = urgency;
        self
    }

    pub fn with_id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    pub async fn send(&self) -> zbus::Result<u32> {
        let mut hints = HashMap::new();
        hints.insert("urgency", zbus::zvariant::Value::U8(self.urgency as u8));
        if let Some(value) = self.progress {
            hints.insert("value", zbus::zvariant::Value::I32(value));
        }

        self.proxy
            .notify(
                "SysNotifier",
                self.id,
                self.icon,
                self.summary,
                self.body,
                &[],
                hints,
                -1,
            )
            .await
    }
}
