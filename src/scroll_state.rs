use ratatui::widgets::TableState;

/// Generic scrollable list state that manages selection, bounds, and ratatui scroll offset.
/// Replaces the scattered `*_selected: usize` fields and adds proper scrolling.
#[derive(Debug, Clone)]
pub struct ScrollState {
    selected: usize,
    len: usize,
    table_state: TableState,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollState {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            selected: 0,
            len: 0,
            table_state,
        }
    }

    /// Update the total number of items. Clamps selection if needed.
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len.saturating_sub(1);
        }
        self.sync_table_state();
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn move_up(&mut self) {
        if self.len == 0 {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.sync_table_state();
    }

    pub fn move_down(&mut self) {
        if self.len == 0 {
            return;
        }
        self.selected = (self.selected + 1).min(self.len.saturating_sub(1));
        self.sync_table_state();
    }

    pub fn move_top(&mut self) {
        self.selected = 0;
        self.sync_table_state();
    }

    pub fn move_bottom(&mut self) {
        if self.len == 0 {
            return;
        }
        self.selected = self.len.saturating_sub(1);
        self.sync_table_state();
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
        self.sync_table_state();
    }

    pub fn page_down(&mut self, page_size: usize) {
        if self.len == 0 {
            return;
        }
        self.selected = (self.selected + page_size).min(self.len.saturating_sub(1));
        self.sync_table_state();
    }

    /// Get mutable reference to ratatui TableState for stateful rendering.
    pub fn table_state_mut(&mut self) -> &mut TableState {
        &mut self.table_state
    }

    /// Reset selection to 0.
    pub fn reset(&mut self) {
        self.selected = 0;
        self.sync_table_state();
    }

    fn sync_table_state(&mut self) {
        self.table_state.select(Some(self.selected));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_list() {
        let mut s = ScrollState::new();
        s.set_len(0);
        assert_eq!(s.selected(), 0);
        s.move_down();
        assert_eq!(s.selected(), 0);
        s.move_up();
        assert_eq!(s.selected(), 0);
    }

    #[test]
    fn test_navigation() {
        let mut s = ScrollState::new();
        s.set_len(10);
        assert_eq!(s.selected(), 0);

        s.move_down();
        assert_eq!(s.selected(), 1);

        s.move_bottom();
        assert_eq!(s.selected(), 9);

        s.move_down(); // can't go past end
        assert_eq!(s.selected(), 9);

        s.move_top();
        assert_eq!(s.selected(), 0);

        s.move_up(); // can't go past start
        assert_eq!(s.selected(), 0);
    }

    #[test]
    fn test_page_navigation() {
        let mut s = ScrollState::new();
        s.set_len(100);

        s.page_down(20);
        assert_eq!(s.selected(), 20);

        s.page_down(20);
        assert_eq!(s.selected(), 40);

        s.page_up(20);
        assert_eq!(s.selected(), 20);

        s.page_up(30); // clamps to 0
        assert_eq!(s.selected(), 0);
    }

    #[test]
    fn test_clamp_on_shrink() {
        let mut s = ScrollState::new();
        s.set_len(10);
        s.move_bottom();
        assert_eq!(s.selected(), 9);

        s.set_len(5); // shrink - should clamp
        assert_eq!(s.selected(), 4);

        s.set_len(0);
        assert_eq!(s.selected(), 0);
    }

    #[test]
    fn test_table_state_sync() {
        let mut s = ScrollState::new();
        s.set_len(5);
        s.move_down();
        s.move_down();
        assert_eq!(s.table_state_mut().selected(), Some(2));
    }
}
