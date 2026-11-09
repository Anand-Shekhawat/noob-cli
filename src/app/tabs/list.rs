//! The list of open tabs, and which one you're looking at.
//!
//! It's generic (`T` can be anything), so the rules are tested with plain
//! numbers instead of real shells.

/// Tabs in order, left to right, with one of them active. Never empty.
#[derive(Debug)]
pub struct TabList<T> {
    tabs: Vec<T>,
    active: usize,
}

impl<T> TabList<T> {
    /// A list with one tab, which is active.
    pub fn new(first: T) -> TabList<T> {
        TabList {
            tabs: vec![first],
            active: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Where the active tab is (0 is the leftmost).
    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &T {
        &self.tabs[self.active]
    }

    pub fn active_mut(&mut self) -> &mut T {
        &mut self.tabs[self.active]
    }

    /// Adds a tab at the right-hand end and makes it active.
    pub fn add(&mut self, tab: T) {
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    /// Closes the tab at `index`. Returns `false` (and keeps it) when it's the
    /// last one: closing that means closing the window, which the caller does.
    /// After closing the active tab, the one to its right becomes active (or,
    /// at the end, the one to its left), like in web browsers.
    pub fn close(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || self.tabs.len() == 1 {
            return false;
        }
        self.tabs.remove(index);
        // A tab on the left went away: the active one moved one place left.
        // The active one itself went away: its right neighbour slid into its
        // place, unless it was the last, then the new last one is active.
        if index < self.active {
            self.active -= 1;
        }
        self.active = self.active.min(self.tabs.len() - 1);
        true
    }

    /// Makes the tab at `index` active (ignored if there's no such tab).
    pub fn select(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    /// The next tab to the right, going round to the first after the last.
    pub fn next(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
    }

    /// The tab to the left, going round to the last before the first.
    pub fn previous(&mut self) {
        self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
    }

    /// Every tab, left to right, with its place in the list.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        self.tabs.iter_mut().enumerate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(count: usize) -> TabList<usize> {
        let mut list = TabList::new(0);
        for tab in 1..count {
            list.add(tab);
        }
        list
    }

    #[test]
    fn a_new_list_has_one_active_tab() {
        let list = TabList::new("first");
        assert_eq!(list.len(), 1);
        assert_eq!(*list.active(), "first");
    }

    #[test]
    fn a_new_tab_goes_at_the_end_and_becomes_active() {
        let mut list = list(3);
        list.select(0);
        list.add(9);
        assert_eq!(list.len(), 4);
        assert_eq!(list.active_index(), 3);
        assert_eq!(*list.active(), 9);
    }

    #[test]
    fn closing_the_active_tab_activates_its_right_neighbour() {
        let mut list = list(3);
        list.select(1);
        assert!(list.close(1));
        assert_eq!(*list.active(), 2);
    }

    #[test]
    fn closing_the_last_tab_on_the_right_activates_its_left_neighbour() {
        let mut list = list(3);
        assert!(list.close(2));
        assert_eq!(*list.active(), 1);
    }

    #[test]
    fn closing_a_tab_left_of_the_active_one_keeps_the_same_tab_active() {
        let mut list = list(3);
        list.select(2);
        assert!(list.close(0));
        assert_eq!(*list.active(), 2);
        assert_eq!(list.active_index(), 1);
    }

    #[test]
    fn the_only_tab_is_never_closed_here() {
        let mut list = list(1);
        assert!(!list.close(0), "closing it means closing the window");
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn closing_or_selecting_a_tab_that_does_not_exist_changes_nothing() {
        let mut list = list(2);
        assert!(!list.close(7));
        list.select(7);
        assert_eq!(list.len(), 2);
        assert_eq!(list.active_index(), 1);
    }

    #[test]
    fn next_and_previous_go_round_the_ends() {
        let mut list = list(3);
        list.next();
        assert_eq!(list.active_index(), 0, "after the last comes the first");
        list.previous();
        assert_eq!(list.active_index(), 2, "before the first comes the last");
        list.previous();
        assert_eq!(list.active_index(), 1);
    }
}
