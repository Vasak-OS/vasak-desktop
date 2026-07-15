use serde::Serialize;
use std::collections::HashMap;

use super::WindowInfo;

/// Represents the differences between two window list snapshots.
/// Used to emit incremental updates instead of full list replacements.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct WindowDelta {
    /// Windows present in `current` but not in `prev`
    pub added: Vec<WindowInfo>,
    /// IDs of windows present in `prev` but not in `current`
    pub removed: Vec<String>,
    /// Windows present in both but with changed title, is_minimized, icon, or demands_attention
    pub modified: Vec<WindowInfo>,
}

impl WindowDelta {
    /// Computes the delta between a previous and current window list.
    /// Returns `None` if there are no changes (added, removed, or modified are all empty).
    ///
    /// Identity is determined by `window.id`. A window is considered modified if its
    /// `title`, `is_minimized`, `icon`, or `demands_attention` fields differ.
    pub fn compute(prev: &[WindowInfo], current: &[WindowInfo]) -> Option<Self> {
        let prev_map: HashMap<&str, &WindowInfo> =
            prev.iter().map(|w| (w.id.as_str(), w)).collect();

        let current_map: HashMap<&str, &WindowInfo> =
            current.iter().map(|w| (w.id.as_str(), w)).collect();

        let mut added = Vec::new();
        let mut modified = Vec::new();

        for window in current {
            match prev_map.get(window.id.as_str()) {
                None => {
                    added.push(window.clone());
                }
                Some(prev_window) => {
                    if Self::has_changed(prev_window, window) {
                        modified.push(window.clone());
                    }
                }
            }
        }

        let removed: Vec<String> = prev
            .iter()
            .filter(|w| !current_map.contains_key(w.id.as_str()))
            .map(|w| w.id.clone())
            .collect();

        if added.is_empty() && removed.is_empty() && modified.is_empty() {
            None
        } else {
            Some(WindowDelta {
                added,
                removed,
                modified,
            })
        }
    }

    /// Checks whether any of the mutable fields (title, is_minimized, icon, demands_attention)
    /// have changed between two window entries with the same id.
    fn has_changed(prev: &WindowInfo, current: &WindowInfo) -> bool {
        prev.title != current.title
            || prev.is_minimized != current.is_minimized
            || prev.icon != current.icon
            || prev.demands_attention != current.demands_attention
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_window(id: &str, title: &str, minimized: bool) -> WindowInfo {
        WindowInfo {
            id: id.to_string(),
            title: title.to_string(),
            is_minimized: minimized,
            icon: "app-icon".to_string(),
            demands_attention: None,
        }
    }

    #[test]
    fn test_no_changes_returns_none() {
        let windows = vec![
            make_window("1", "Terminal", false),
            make_window("2", "Browser", false),
        ];
        assert_eq!(WindowDelta::compute(&windows, &windows), None);
    }

    #[test]
    fn test_added_window() {
        let prev = vec![make_window("1", "Terminal", false)];
        let current = vec![
            make_window("1", "Terminal", false),
            make_window("2", "Browser", false),
        ];

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.added[0].id, "2");
        assert!(delta.removed.is_empty());
        assert!(delta.modified.is_empty());
    }

    #[test]
    fn test_removed_window() {
        let prev = vec![
            make_window("1", "Terminal", false),
            make_window("2", "Browser", false),
        ];
        let current = vec![make_window("1", "Terminal", false)];

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert!(delta.added.is_empty());
        assert_eq!(delta.removed, vec!["2".to_string()]);
        assert!(delta.modified.is_empty());
    }

    #[test]
    fn test_modified_title() {
        let prev = vec![make_window("1", "Terminal", false)];
        let current = vec![make_window("1", "Terminal - vim", false)];

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert!(delta.added.is_empty());
        assert!(delta.removed.is_empty());
        assert_eq!(delta.modified.len(), 1);
        assert_eq!(delta.modified[0].title, "Terminal - vim");
    }

    #[test]
    fn test_modified_minimized() {
        let prev = vec![make_window("1", "Terminal", false)];
        let current = vec![make_window("1", "Terminal", true)];

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert!(delta.added.is_empty());
        assert!(delta.removed.is_empty());
        assert_eq!(delta.modified.len(), 1);
        assert!(delta.modified[0].is_minimized);
    }

    #[test]
    fn test_modified_icon() {
        let prev = vec![make_window("1", "Terminal", false)];
        let mut current = vec![make_window("1", "Terminal", false)];
        current[0].icon = "new-icon".to_string();

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert_eq!(delta.modified.len(), 1);
        assert_eq!(delta.modified[0].icon, "new-icon");
    }

    #[test]
    fn test_modified_demands_attention() {
        let prev = vec![make_window("1", "Terminal", false)];
        let mut current = vec![make_window("1", "Terminal", false)];
        current[0].demands_attention = Some(true);

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert_eq!(delta.modified.len(), 1);
        assert_eq!(delta.modified[0].demands_attention, Some(true));
    }

    #[test]
    fn test_combined_added_removed_modified() {
        let prev = vec![
            make_window("1", "Terminal", false),
            make_window("2", "Browser", false),
            make_window("3", "Editor", false),
        ];
        let current = vec![
            make_window("1", "Terminal - vim", false), // modified title
            make_window("3", "Editor", false),         // unchanged
            make_window("4", "Files", false),          // added
        ];

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.added[0].id, "4");
        assert_eq!(delta.removed, vec!["2".to_string()]);
        assert_eq!(delta.modified.len(), 1);
        assert_eq!(delta.modified[0].id, "1");
    }

    #[test]
    fn test_empty_lists_returns_none() {
        let empty: Vec<WindowInfo> = vec![];
        assert_eq!(WindowDelta::compute(&empty, &empty), None);
    }

    #[test]
    fn test_all_new_windows() {
        let prev: Vec<WindowInfo> = vec![];
        let current = vec![
            make_window("1", "Terminal", false),
            make_window("2", "Browser", false),
        ];

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert_eq!(delta.added.len(), 2);
        assert!(delta.removed.is_empty());
        assert!(delta.modified.is_empty());
    }

    #[test]
    fn test_all_windows_removed() {
        let prev = vec![
            make_window("1", "Terminal", false),
            make_window("2", "Browser", false),
        ];
        let current: Vec<WindowInfo> = vec![];

        let delta = WindowDelta::compute(&prev, &current).unwrap();
        assert!(delta.added.is_empty());
        assert_eq!(delta.removed.len(), 2);
        assert!(delta.modified.is_empty());
    }
}
