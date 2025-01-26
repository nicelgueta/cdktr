use std::collections::HashMap;
use std::hash::Hash;

use ratatui::layout::{Constraint, Flex, Layout, Rect};

pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

pub fn vec_to_hashmap<T, V>(v: Vec<(T, V)>) -> HashMap<T, V>
where
    T: Hash + Eq,
    V: Eq,
{
    let mut hashmap = HashMap::new();
    for (k, v) in v {
        hashmap.insert(k, v);
    }
    hashmap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center() {
        let area = Rect::new(0, 0, 100, 100);
        let centered = center(area, Constraint::Percentage(50), Constraint::Percentage(50));
        assert_eq!(centered, Rect::new(25, 25, 50, 50));
    }

    #[test]
    fn test_center_small_area() {
        let area = Rect::new(0, 0, 10, 10);
        let centered = center(area, Constraint::Percentage(50), Constraint::Percentage(50));
        assert_eq!(centered, Rect::new(3, 3, 5, 5)); // rounds up not down
    }

    #[test]
    fn test_vec_to_hashmap() {
        let v = vec![(1, "a"), (2, "b"), (3, "c")];
        let h = vec_to_hashmap(v);
        assert_eq!(h.get(&1), Some(&"a"));
        assert_eq!(h.get(&2), Some(&"b"));
        assert_eq!(h.get(&3), Some(&"c"));
    }

    #[test]
    fn test_vec_to_hashmap_empty_vec() {
        let v: Vec<(i32, &str)> = vec![];
        let h = vec_to_hashmap(v);
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn test_vec_to_hashmap_duplicate_keys() {
        let v = vec![(1, "a"), (1, "b"), (1, "c")];
        let h = vec_to_hashmap(v);
        assert_eq!(h.get(&1), Some(&"c"));
    }
}
