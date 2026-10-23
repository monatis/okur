//! Windows' own list of the notifications it keeps (UserNotificationListener), those Focus
//! Assist kept from showing included. JAWS's Notification History listed a toast whose banner
//! Focus Assist hid, and the banner's window, which the tracer reads, never opened [obs, 533];
//! the list is where such a toast can be found.

/// One toast Windows keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedToast {
    /// Windows' id for it, unique among the notifications it keeps.
    pub id: u32,
    /// The sending application's name, as Windows shows it.
    pub app: String,
    /// The toast's text elements, its title first.
    pub texts: Vec<String>,
}

/// Whether this program may read the list, as Windows answers without asking the user:
/// "allowed", "denied" or "unspecified".
#[cfg(windows)]
pub fn access() -> Result<&'static str, String> {
    use windows::UI::Notifications::Management::{
        UserNotificationListener, UserNotificationListenerAccessStatus,
    };
    let status = UserNotificationListener::Current()
        .and_then(|listener| listener.GetAccessStatus())
        .map_err(|e| format!("the notification listener: {e}"))?;
    Ok(match status {
        UserNotificationListenerAccessStatus::Allowed => "allowed",
        UserNotificationListenerAccessStatus::Denied => "denied",
        _ => "unspecified",
    })
}

/// The toasts Windows keeps, in its order.
#[cfg(windows)]
pub fn toasts() -> Result<Vec<ListedToast>, String> {
    use windows::UI::Notifications::Management::UserNotificationListener;
    use windows::UI::Notifications::{
        KnownNotificationBindings, NotificationKinds, UserNotification,
    };

    fn listed(notification: &UserNotification) -> windows::core::Result<ListedToast> {
        let app = notification
            .AppInfo()
            .and_then(|info| info.DisplayInfo())
            .and_then(|display| display.DisplayName())
            .map(|name| name.to_string())
            .unwrap_or_default();
        let binding = notification
            .Notification()?
            .Visual()?
            .GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
        let texts = binding
            .GetTextElements()?
            .into_iter()
            .filter_map(|text| text.Text().ok().map(|t| t.to_string()))
            .collect();
        Ok(ListedToast {
            id: notification.Id()?,
            app,
            texts,
        })
    }

    let list = UserNotificationListener::Current()
        .and_then(|listener| listener.GetNotificationsAsync(NotificationKinds::Toast))
        .and_then(|operation| operation.join())
        .map_err(|e| format!("listing the notifications: {e}"))?;
    Ok(list
        .into_iter()
        .filter_map(|notification| listed(&notification).ok())
        .collect())
}
