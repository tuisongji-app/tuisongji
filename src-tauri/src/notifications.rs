use std::time::Instant;

#[derive(Clone)]
pub(crate) struct NotifItem {
    pub room_id: u64,
    pub name: String,
    pub action: String,
    pub avatar_path: Option<String>,
    pub sub_type: String,
    pub expires_at: Instant,
}

pub(crate) fn tray_title(items: &[NotifItem], now: Instant) -> String {
    if let Some(item) = items.iter().find(|item| item.expires_at > now) {
        format!("{} {}", item.name, item.action)
    } else if items.is_empty() {
        String::new()
    } else {
        format!("({})", items.len())
    }
}

pub(crate) fn next_expiry(items: &[NotifItem], now: Instant) -> Option<Instant> {
    items
        .iter()
        .map(|item| item.expires_at)
        .filter(|deadline| *deadline > now)
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn notification(room_id: u64, name: &str, expires_at: Instant) -> NotifItem {
        NotifItem {
            room_id,
            name: name.to_string(),
            action: "开播".to_string(),
            avatar_path: None,
            sub_type: "bilibili".to_string(),
            expires_at,
        }
    }

    #[test]
    fn dismissing_expired_messages_only_updates_the_count() {
        let now = Instant::now();
        let mut items = vec![
            notification(1, "主播一", now),
            notification(2, "主播二", now),
        ];
        assert_eq!(tray_title(&items, now), "(2)");

        items.retain(|item| item.room_id != 1);
        assert_eq!(tray_title(&items, now), "(1)");
        assert_eq!(next_expiry(&items, now), None);

        items.clear();
        assert_eq!(tray_title(&items, now), "");
    }

    #[test]
    fn dismissing_a_new_message_does_not_redisplay_expired_messages() {
        let now = Instant::now();
        let mut items = vec![
            notification(1, "新主播", now + Duration::from_secs(60)),
            notification(2, "旧主播", now),
        ];
        assert_eq!(tray_title(&items, now), "新主播 开播");

        items.retain(|item| item.room_id != 1);
        assert_eq!(tray_title(&items, now), "(1)");
    }

    #[test]
    fn remaining_messages_keep_their_original_deadlines() {
        let now = Instant::now();
        let older_deadline = now + Duration::from_secs(30);
        let newer_deadline = now + Duration::from_secs(60);
        let mut items = vec![
            notification(1, "新主播", newer_deadline),
            notification(2, "旧主播", older_deadline),
        ];
        assert_eq!(next_expiry(&items, now), Some(older_deadline));
        assert_eq!(tray_title(&items, older_deadline), "新主播 开播");

        items.retain(|item| item.room_id != 1);
        assert_eq!(tray_title(&items, now), "旧主播 开播");
        assert_eq!(next_expiry(&items, now), Some(older_deadline));
        assert_eq!(tray_title(&items, older_deadline), "(1)");
        assert_eq!(next_expiry(&items, older_deadline), None);
    }

    #[test]
    fn new_messages_can_show_text_after_older_messages_have_expired() {
        let now = Instant::now();
        let deadline = now + Duration::from_secs(60);
        let mut items = vec![notification(1, "旧主播", now)];
        assert_eq!(tray_title(&items, now), "(1)");

        items.insert(0, notification(2, "新主播", deadline));
        assert_eq!(tray_title(&items, now), "新主播 开播");
        assert_eq!(tray_title(&items, deadline), "(2)");
    }
}
