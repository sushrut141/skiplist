use rand::Rng;
use std::cell::RefCell;
use std::clone::Clone;
use std::cmp::{Ord, Ordering};
use std::fmt::Display;
use std::option::Option;
use std::rc::{Rc, Weak};

type Link<K, V> = Option<Rc<RefCell<Node<K, V>>>>;
type WeakLink<K, V> = Option<Weak<RefCell<Node<K, V>>>>;

struct Node<K, V> {
    key: K,
    value: V,
    right: Link<K, V>,
    down: Link<K, V>,
    left: WeakLink<K, V>,
    up: WeakLink<K, V>,
}

impl<K, V> Node<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    fn new(key: K, value: V) -> Node<K, V> {
        Node {
            key,
            value,
            right: None,
            down: None,
            left: None,
            up: None,
        }
    }

    fn cmp(&self, value: &K) -> Ordering {
        self.key.cmp(value)
    }
}

struct Level<K, V> {
    size: usize,
    head: Link<K, V>,
}

impl<K, V> Level<K, V>
where
    K: Ord + Clone,
    V: Clone,
{
    fn new() -> Level<K, V> {
        Level {
            size: 0,
            head: None,
        }
    }

    fn iter(&self) -> Iter<K, V> {
        Iter {
            next: self.head.as_ref().map(Rc::clone),
        }
    }

    // Return the node after which a node with the supplied key can be inserted.
    // For example in this list:
    //     h -> 1 -> 2 -> 2 -> 3
    //                    ^
    //                    |
    //    bisection point for key `3`
    fn bisect(&mut self, key: &K) -> Link<K, V> {
        let maybe_marker = self.iter().find(|node_ref| {
            return match node_ref.borrow().cmp(key) {
                Ordering::Greater => true,
                Ordering::Less | Ordering::Equal => false,
            };
        });
        if maybe_marker.is_some() {
            let marker = maybe_marker.unwrap();
            return marker.borrow().left.as_ref().and_then(Weak::upgrade);
        }
        self.iter().last()
    }

    // Find the insertion point for the supplied key after the given node.
    // For example in the below list the bisection point is 5
    // h -> 1 -> 2 -> 5 -> 7
    //      |         |
    //      node      insertion point for key 6
    fn bisect_after(&self, node: &Rc<RefCell<Node<K, V>>>, target: &K) -> Link<K, V> {
        if node.borrow().key.cmp(target) == Ordering::Greater {
            return None;
        }
        let mut maybe_current = Some(Rc::clone(node));
        let mut prev: Link<K, V> = node.borrow().left.as_ref().and_then(Weak::upgrade);
        let mut output = None;
        while maybe_current.is_some() {
            let current = maybe_current.take().unwrap();
            prev = Some(Rc::clone(&current));
            match current.borrow().cmp(target) {
                Ordering::Less => {
                    maybe_current = current.borrow().right.as_ref().map(Rc::clone);
                }
                Ordering::Equal => {
                    maybe_current = current.borrow().right.as_ref().map(Rc::clone);
                }
                Ordering::Greater => {
                    output = current.borrow().left.as_ref().and_then(Weak::upgrade);
                }
            };
            if output.is_some() {
                break;
            }
        }
        // found insertion point
        if output.is_some() {
            return output;
        }
        return prev;
    }

    fn insert(&mut self, key: K, value: V) -> Rc<RefCell<Node<K, V>>> {
        let mut head: Link<K, V> = self.head.as_ref().map(Rc::clone);
        let mut maybe_prev_node = Option::None;
        while head.is_some() {
            let node = head.take().unwrap();
            match node.borrow().cmp(&key) {
                Ordering::Less | Ordering::Equal => {
                    maybe_prev_node = Some(Rc::clone(&node));
                    head = node.borrow().right.as_ref().map(Rc::clone);
                }
                Ordering::Greater => {
                    break;
                }
            };
        }
        return match maybe_prev_node {
            // insert at head
            None => {
                let maybe_prev_head_ref: Option<Rc<RefCell<Node<K, V>>>> =
                    self.head.as_ref().map(Rc::clone);
                if maybe_prev_head_ref.is_some() {
                    let prev_head_ref = maybe_prev_head_ref.unwrap();
                    let new_head = Rc::new(RefCell::new(Node::new(key, value)));
                    new_head.borrow_mut().right = self.head.take();
                    self.head = Some(new_head);
                    prev_head_ref.borrow_mut().left = self.head.as_ref().map(Rc::downgrade);
                } else {
                    self.head = Some(Rc::new(RefCell::new(Node::new(key, value))));
                }
                self.size += 1;
                Rc::clone(self.head.as_ref().unwrap())
            }
            Some(prev_node) => {
                let maybe_next_node: Option<Rc<RefCell<Node<K, V>>>> =
                    prev_node.borrow().right.as_ref().map(Rc::clone);
                let new_node = Rc::new(RefCell::new(Node::new(key, value)));
                if maybe_next_node.is_some() {
                    // handle insert in the middle
                    let next_node = maybe_next_node.unwrap();
                    next_node.borrow_mut().left = Some(Rc::downgrade(&new_node));
                    new_node.borrow_mut().right = prev_node.borrow_mut().right.take();
                    new_node.borrow_mut().left = Some(Rc::downgrade(&prev_node));
                    prev_node.borrow_mut().right = Some(new_node);
                    self.size += 1;
                } else {
                    // handle insert at tail
                    new_node.borrow_mut().left = Some(Rc::downgrade(&prev_node));
                    prev_node.borrow_mut().right = Some(new_node);
                    self.size += 1;
                }
                Rc::clone(prev_node.borrow().right.as_ref().unwrap())
            }
        };
    }

    // Insert after the supplied node.
    // This method just inserts after the supplied node.
    // It is up to the caller to ensure that the sorted order is maintained.
    fn insert_after(
        &mut self,
        key: K,
        value: V,
        after: Rc<RefCell<Node<K, V>>>,
    ) -> Rc<RefCell<Node<K, V>>> {
        let node = Rc::new(RefCell::new(Node::new(key, value)));
        let maybe_next_node = after.borrow_mut().right.take();
        node.borrow_mut().left = Some(Rc::downgrade(&after));
        if maybe_next_node.is_some() {
            let next_node = maybe_next_node.unwrap();
            next_node.borrow_mut().left = Some(Rc::downgrade(&node));
            node.borrow_mut().right = Some(next_node);
        }
        after.borrow_mut().right = Some(node);
        self.size += 1;
        Rc::clone(after.borrow().right.as_ref().unwrap())
    }

    fn delete(&mut self, key: &K) {
        let maybe_node = self.iter().find(|node_ref| {
            return match node_ref.borrow().cmp(key) {
                Ordering::Equal => true,
                Ordering::Less | Ordering::Greater => false,
            };
        });
        if maybe_node.is_some() {
            let to_delete = maybe_node.as_ref().unwrap();
            let maybe_prev_node = to_delete.borrow().left.as_ref().and_then(Weak::upgrade);
            if maybe_prev_node.is_some() {
                let prev_node: Rc<RefCell<Node<K, V>>> = maybe_prev_node.unwrap();
                let maybe_new_next: Option<Rc<RefCell<Node<K, V>>>> =
                    to_delete.borrow().right.as_ref().map(Rc::clone);
                if maybe_new_next.is_some() {
                    let new_next = maybe_new_next.unwrap();
                    new_next.borrow_mut().left = Some(Rc::downgrade(&prev_node));
                }
                prev_node.borrow_mut().right = to_delete.borrow_mut().right.take();
            } else {
                // handle deleting head
                self.head = to_delete.borrow_mut().right.take();
                to_delete.borrow_mut().left = None;
            }
            self.size -= 1;
        }
    }
}

struct Iter<K, V> {
    next: Link<K, V>,
}

impl<K, V> Iterator for Iter<K, V> {
    type Item = Rc<RefCell<Node<K, V>>>;

    fn next(&mut self) -> Option<Self::Item> {
        let maybe_current: Option<Rc<RefCell<Node<K, V>>>> = self.next.as_ref().map(Rc::clone);
        if maybe_current.is_some() {
            let current = maybe_current.unwrap();
            self.next = current.borrow_mut().right.as_ref().map(Rc::clone);
            return Some(current);
        }
        None
    }
}

/// Skip List is an alternative to self balancing sorted data structures like AVL Trees and
/// Red Black Trees.
///
/// It supports fast insertion and lookup times with logarithmic complexity.
///
/// Skip List is a probabilistic data structure that uses multiple stacked Linked Lists
/// to achieve fast read and writes.
/// For more information about how skip lists work
/// refer [here](https://en.wikipedia.org/wiki/Skip_list).
pub struct SkipList<K, V> {
    size: usize,
    levels: Vec<Level<K, V>>,
}

enum Insertion<K, V> {
    Before,
    // represents insertion point after supplied node
    After(Rc<RefCell<Node<K, V>>>),
}

impl<K, V> SkipList<K, V>
where
    K: Ord + Clone + Display,
    V: Clone,
{
    /// Create a empty skip list. This is the recommended way of creating a skip list.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let list: SkipList<i32, i32> = SkipList::new();
    /// ```
    pub fn new() -> SkipList<K, V> {
        let levels = vec![Level::new()];
        SkipList { size: 0, levels }
    }

    /// Insert the given key and value into the list.
    ///
    /// # Arguments
    /// * _key_ - The key by which the value is to be accessed.
    ///             This is also used as the sort key.
    /// * _value_ - The value to store associated with the key.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let mut cakes: SkipList<i32, String> = SkipList::new();
    /// cakes.insert(20, "Strawberry Topping".to_owned());
    /// cakes.insert(40, "Chocolate Glaze".to_owned());
    ///
    /// assert_eq!(cakes.len(), 2);
    /// ```
    ///
    pub fn insert(&mut self, key: K, value: V) {
        if self.levels.len() > 0 {
            let mut insertion_path = Vec::new();
            self.bisect_levels(&key, &mut insertion_path);
            let mut is_head = false;
            match insertion_path.get(0).unwrap() {
                Insertion::Before => {
                    is_head = true;
                }
                _ => (),
            }
            let mut prev_level_node = self.insert_at_position(0, &key, &value, &insertion_path[0]);
            let mut i = 1;
            while i < self.levels.len() {
                let current_level_node =
                    self.insert_at_position(i, &key, &value, &insertion_path[i]);
                prev_level_node.borrow_mut().up = Some(Rc::downgrade(&current_level_node));
                current_level_node.borrow_mut().down = Some(Rc::clone(&prev_level_node));
                prev_level_node = Rc::clone(&current_level_node);
                i += 1;
            }
            // create more levels probabilistically if more than one node present
            // and newly added node is not head
            // we expect more than one node to be present to avoid
            // let head_ref = self.levels[0].head.as_ref().unwrap();
            if self.levels[0].size > 1 {
                while self.flip_coin() && !is_head {
                    self.add_level();
                    let curr_size = self.levels.len();
                    let new_node = self.levels[curr_size - 1].insert(key.clone(), value.clone());
                    prev_level_node.borrow_mut().up = Some(Rc::downgrade(&new_node));
                    new_node.borrow_mut().down = Some(Rc::clone(&prev_level_node));
                    prev_level_node = Rc::clone(&new_node);
                }
            }
            self.size += 1;
        }
    }

    /// Get the value associated with a key if it exists.
    ///
    /// # Arguments
    /// * _key_ - The key whose value is to be read.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let mut cakes: SkipList<i32, String> = SkipList::new();
    /// cakes.insert(20, "Strawberry Topping".to_owned());
    /// cakes.insert(40, "Chocolate Glaze".to_owned());
    /// let maybe_chocolate = cakes.get(&20);
    /// assert_eq!(maybe_chocolate.is_some(), true);
    /// assert_eq!(maybe_chocolate.unwrap(), "Strawberry Topping");
    /// ```
    pub fn get(&mut self, key: &K) -> Option<V> {
        let size = self.levels.len();
        let mut i = 0;
        let mut maybe_prev = self.levels[size - i - 1].bisect(key);
        i += 1;
        while i < size && maybe_prev.is_some() {
            let prev = maybe_prev.take().unwrap();
            let after = prev.borrow().down.as_ref().map(Rc::clone).unwrap();
            maybe_prev = self.levels[size - i - 1].bisect_after(&after, key);
            i += 1;
        }
        if maybe_prev.is_some() {
            let found = maybe_prev.take().unwrap();
            return match found.borrow().cmp(key) {
                Ordering::Equal => Some(found.borrow().value.clone()),
                _ => None,
            };
        }
        None
    }

    /// Delete the value associated with the key.
    ///
    /// # Arguments
    /// * _key_ - The key associated with the value to delete.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let mut cakes: SkipList<i32, String> = SkipList::new();
    /// cakes.insert(20, "Strawberry Topping".to_owned());
    /// cakes.insert(40, "Chocolate Glaze".to_owned());
    /// cakes.insert(100, "lemon cream".to_owned());
    /// cakes.delete(&100);
    /// ```
    pub fn delete(&mut self, key: &K) {
        let size = self.levels.len();
        for i in 0..size {
            self.levels[i].delete(key);
        }
        self.size = self.levels[0].size;
    }

    /// Find the largest key after which the supplied key can be inserted.
    ///
    /// # Arguments
    /// * _key_ - The key whose insertion point is to be found.
    ///
    /// # Returns
    /// `Option` of key after which the given key can be inserted.
    /// `None` implies no key less that supplied key exists.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let mut list = SkipList::new();
    /// list.insert(1, "small");
    /// list.insert(4, "x-large");
    /// list.insert(3, "large");
    /// // 1 -> 3 -> 4
    /// // |
    /// // 2 insertion point
    /// let maybe_medium_insertion = list.bisect(&2);
    /// assert!(maybe_medium_insertion.is_some());
    /// assert_eq!(maybe_medium_insertion.unwrap(), 1);
    /// ```
    pub fn bisect(&mut self, key: &K) -> Option<K> {
        let size = self.levels.len();
        if size > 0 {
            let mut i = 0;
            let mut maybe_prev = self.levels[size - i - 1].bisect(key);
            let mut prev_node: Option<Rc<RefCell<Node<K, V>>>> = None;
            while i < size && maybe_prev.is_some() {
                prev_node = maybe_prev.as_ref().map(Rc::clone);
                let prev = maybe_prev.take().unwrap();
                let maybe_after = prev.borrow().down.as_ref().map(Rc::clone);
                if maybe_after.is_none() {
                    return Some(prev.borrow().key.clone());
                }
                let after = maybe_after.unwrap();
                i += 1;
                maybe_prev = self.levels[size - i - 1].bisect_after(&after, key);
            }
            return prev_node.map(|node_rc| node_rc.borrow().key.clone());
        }
        None
    }

    /// Collect the entries sorted by key into a collection.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let mut cakes: SkipList<i32, String> = SkipList::new();
    /// cakes.insert(20, "Strawberry Topping".to_owned());
    /// cakes.insert(40, "Chocolate Glaze".to_owned());
    /// cakes.insert(100, "Lemon Cream".to_owned());
    ///
    /// // add to cart
    /// let cart = cakes.collect();
    /// assert_eq!(cart.len(), 3);
    /// assert_eq!(cart, vec![(20, "Strawberry Topping".to_owned()), (40, "Chocolate Glaze".to_owned()), (100, "Lemon Cream".to_owned())])
    /// ```
    pub fn collect(&self) -> Vec<(K, V)> {
        let mut values = Vec::new();
        self.iter().for_each(|node_ref| {
            let key = node_ref.borrow().key.clone();
            let value = node_ref.borrow().value.clone();
            values.push((key, value));
        });
        values
    }

    /// Returns the size of the list.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let mut cakes: SkipList<i32, String> = SkipList::new();
    /// cakes.insert(20, "Strawberry Topping".to_owned());
    /// cakes.insert(40, "Chocolate Glaze".to_owned());
    /// cakes.insert(100, "Lemon Cream".to_owned());
    /// assert_eq!(cakes.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns whether list is empty.
    ///
    /// # Example
    /// ```rust
    /// use subway::skiplist::SkipList;
    ///
    /// let empty_list: SkipList<i32, i32> = SkipList::new();
    /// assert_eq!(empty_list.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    fn insert_at_position(
        &mut self,
        level: usize,
        key: &K,
        value: &V,
        insertion: &Insertion<K, V>,
    ) -> Rc<RefCell<Node<K, V>>> {
        return match insertion {
            Insertion::Before => {
                let new_head = self.levels[level].insert(key.clone(), value.clone());
                Rc::clone(&new_head)
            }
            Insertion::After(node) => {
                let new_node =
                    self.levels[level].insert_after(key.clone(), value.clone(), Rc::clone(node));
                Rc::clone(&new_node)
            }
        };
    }

    /// Find the points of insertion in each level to complete an insert to the list.
    fn bisect_levels(&self, key: &K, output: &mut Vec<Insertion<K, V>>) {
        let size = self.levels.len();
        let mut i = 0;
        while i < size {
            let idx = size - i - 1;
            let head = self.levels[idx].head.as_ref();
            if head.is_some() {
                let head_ref = head.unwrap();
                match head_ref.borrow().cmp(key) {
                    Ordering::Greater => {
                        // insert before head
                        output.push(Insertion::Before);
                    }
                    Ordering::Equal | Ordering::Less => {
                        let insertion_point = self.levels[idx].bisect_after(head_ref, key).unwrap();
                        output.push(Insertion::After(insertion_point));
                    }
                }
            } else {
                // insert new head into empty level
                output.push(Insertion::Before)
            }
            i += 1;
        }
        output.reverse()
    }

    fn iter(&self) -> Iter<K, V> {
        Iter {
            next: self.levels[0].head.as_ref().map(Rc::clone),
        }
    }

    fn add_level(&mut self) {
        let size = self.levels.len();
        let prev_head: Rc<RefCell<Node<K, V>>> =
            self.levels[size - 1].head.as_ref().map(Rc::clone).unwrap();
        let key: K = prev_head.borrow().key.clone();
        let value: V = prev_head.borrow().value.clone();
        let mut new_level = Level::new();
        let new_head = new_level.insert(key, value);
        prev_head.borrow_mut().up = Some(Rc::downgrade(&new_head));
        new_head.borrow_mut().down = Some(prev_head);
        self.levels.push(new_level);
    }

    fn flip_coin(&self) -> bool {
        let random = rand::thread_rng().gen_range(0.0, 1.0);
        return random > 0.50;
    }

    fn print(&self) {
        let size = self.levels.len();
        println!("number of levels is {0}", self.levels.len());
        let mut level = 0;
        while level < size {
            println!("printing level {0}", self.levels.len() - level - 1);
            self.levels[self.levels.len() - level - 1]
                .iter()
                .for_each(|node_ref| {
                    println!("{}", node_ref.borrow().key.clone());
                });
            level += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node() {
        let node_a = Node::new(1, "a_val".to_owned());
        let node_b = Node::new(2, "b_val".to_owned());
        let node_c = Node::new(1, "c_val".to_owned());
        assert_eq!(node_a.cmp(&node_b.key), Ordering::Less);
        assert_eq!(node_b.cmp(&node_a.key), Ordering::Greater);
        assert_eq!(node_c.cmp(&node_a.key), Ordering::Equal);
    }

    #[test]
    fn test_level() {
        let mut level = Level::new();
        assert_eq!(level.size, 0);
        level.insert(1, 1);
        assert_eq!(level.size, 1);
    }

    #[test]
    fn test_level_insert() {
        let mut level = Level::new();
        level.insert(1, "val_1".to_owned());
        level.insert(4, "val_4".to_owned());
        level.insert(3, "val_3".to_owned());
        let node = level.insert(0, "val_0".to_owned());
        assert_eq!(node.borrow().key, 0);
        assert_eq!(level.size, 4);
    }

    #[test]
    fn test_level_insert_after() {
        let mut level = Level::new();
        level.insert(3, 3);
        level.insert(0, 0);
        let after = level.insert(1, 1);
        let new_node = level.insert_after(2, 2, Rc::clone(&after));
        let prev_node = new_node.borrow().left.as_ref().and_then(Weak::upgrade);
        let next_node = new_node.borrow().right.as_ref().map(Rc::clone);
        assert!(prev_node.is_some());
        assert_eq!(prev_node.as_ref().unwrap().borrow().key, 1);
        assert!(next_node.is_some());
        assert_eq!(next_node.as_ref().unwrap().borrow().key, 3);
    }

    #[test]
    fn test_level_insert_after_tail() {
        let mut level = Level::new();
        level.insert(3, 3);
        level.insert(0, 0);
        let tail = level.insert(5, 5);
        let new_node = level.insert_after(6, 6, Rc::clone(&tail));
        let prev_node = new_node.borrow().left.as_ref().and_then(Weak::upgrade);
        let next_node = new_node.borrow().right.as_ref().map(Rc::clone);
        assert!(prev_node.is_some());
        assert_eq!(prev_node.as_ref().unwrap().borrow().key, 5);
        assert!(next_node.is_none());
    }

    #[test]
    fn test_bisect_after() {
        let mut level = Level::new();
        level.insert(5, 5);
        level.insert(2, 2);
        level.insert(4, 4);
        let node = level.insert(3, 3);
        let maybe_found = level.bisect_after(&node, &4);
        assert!(maybe_found.is_some());
        assert_eq!(maybe_found.unwrap().borrow().key, 4);
        let maybe_last = level.bisect_after(&node, &7);
        assert!(maybe_last.is_some());
        assert_eq!(maybe_last.unwrap().borrow().key, 5);
    }

    #[test]
    fn test_bisect_after_larger_node() {
        let mut level = Level::new();
        level.insert(4, 4);
        level.insert(2, 2);
        level.insert(3, 3);
        let node = level.insert(1, 1);
        let maybe_found = level.bisect_after(&node, &0);
        assert!(maybe_found.is_none());
    }

    #[test]
    fn test_bisect_after_when_node_does_not_exist() {
        let mut level = Level::new();
        level.insert(4, 4);
        level.insert(2, 2);
        level.insert(3, 3);
        let node = level.insert(1, 1);
        let maybe_found = level.bisect_after(&node, &5);
        assert!(maybe_found.is_some());
        assert!(maybe_found.as_ref().unwrap().borrow().right.is_none());
    }

    #[test]
    fn test_level_is_sorted() {
        let mut level = Level::new();
        level.insert(1, 1);
        level.insert(0, 0);
        level.insert(3, 3);
        level.insert(2, 2);
        level.insert(4, 4);
        let mut values = vec![];
        level.iter().for_each(|node_ref| {
            let val = node_ref.borrow().key;
            values.push(val);
        });
        assert_eq!(values, vec![0, 1, 2, 3, 4]);
        assert_eq!(level.iter().last().as_ref().unwrap().borrow().key, 4);
        level.iter().for_each(|node_ref| {
            let val = node_ref.borrow().key;
            values.push(val);
        });
        assert_eq!(values, vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_bisect_when_key_exists() {
        let mut level = Level::new();
        level.insert(1, 1);
        level.insert(0, 0);
        level.insert(3, 3);
        level.insert(2, 2);
        level.insert(2, 2);
        level.insert(4, 4);
        // test value exists in middle
        let maybe_marker = level.bisect(&2);
        assert!(maybe_marker.is_some());
        assert_eq!(maybe_marker.as_ref().unwrap().borrow().key, 2);
        let maybe_next_node: Option<Rc<RefCell<Node<i32, i32>>>> = maybe_marker
            .as_ref()
            .unwrap()
            .borrow()
            .right
            .as_ref()
            .map(Rc::clone);
        assert_eq!(maybe_next_node.unwrap().borrow().key, 3);
        // test value exists at end
        let maybe_marker = level.bisect(&4);
        assert!(maybe_marker.is_some());
        assert_eq!(maybe_marker.as_ref().unwrap().borrow().key, 4);
    }

    #[test]
    fn test_bisect_when_key_does_not_exist() {
        let mut level = Level::new();
        level.insert(1, 1);
        level.insert(0, 0);
        level.insert(3, 3);
        level.insert(2, 2);
        level.insert(2, 2);
        level.insert(5, 5);
        // test value doesn't exist
        let maybe_marker = level.bisect(&4);
        assert!(maybe_marker.is_some());
        assert_eq!(maybe_marker.as_ref().unwrap().borrow().key, 3);
        let maybe_end = level.bisect(&5);
        assert!(maybe_end.is_some());
        assert!(maybe_end.as_ref().unwrap().borrow().right.is_none());
    }

    #[test]
    fn test_bisect_after_with_last_node() {
        let mut level: Level<i32, i32> = Level::new();
        level.insert(1, 1);
        level.insert(0, 0);
        level.insert(3, 3);
        level.insert(2, 2);
        level.insert(2, 2);
        let last_node = level.insert(5, 5);
        assert!(last_node.borrow().right.is_none());
        let maybe_found = level.bisect_after(&last_node, &5);
        assert!(maybe_found.is_some());
        assert_eq!(
            maybe_found.as_ref().unwrap().borrow().key,
            last_node.borrow().key
        );
    }

    #[test]
    fn test_bisect_after_when_insertion_point_is_at_end() {
        let mut level: Level<i32, i32> = Level::new();
        level.insert(1, 1);
        level.insert(0, 0);
        level.insert(3, 3);
        level.insert(2, 2);
        let node = level.insert(2, 2);
        let maybe_insert = level.bisect_after(&node, &5);
        assert!(maybe_insert.is_some());
        assert_eq!(maybe_insert.as_ref().unwrap().borrow().key, 3);
    }

    #[test]
    fn test_delete_from_level() {
        let mut level = Level::new();
        level.insert(1, 1);
        level.insert(0, 0);
        level.insert(3, 3);
        level.insert(2, 2);
        level.insert(2, 2);
        level.insert(6, 6);
        level.insert(4, 4);
        level.insert(4, 4);
        // delete value from middle of list
        level.delete(&2);
        // delete from end of last
        level.delete(&6);
        // delete from start of list
        level.delete(&0);
        let mut values = vec![];
        level.iter().for_each(|node_ref| {
            let value = node_ref.borrow().key;
            values.push(value);
        });
        assert_eq!(level.size, 5);
        assert_eq!(values, vec![1, 2, 3, 4, 4]);
        let mut new_level = Level::new();
        new_level.insert(0, 0);
        new_level.delete(&0);
        assert_eq!(new_level.size, 0);
    }

    #[test]
    fn test_skiplist() {
        let list: SkipList<i32, i32> = SkipList::new();
        assert_eq!(list.size, 0);
    }

    #[test]
    fn test_skiplist_insert() {
        let mut list = SkipList::new();
        list.insert(2, 2);
        list.insert(3, 3);
        list.insert(1, 1);
        list.insert(4, 4);
        list.insert(5, 5);
        list.insert(7, 7);
        list.insert(8, 8);
        list.insert(6, 6);
        assert_eq!(list.size, 8);
    }

    #[test]
    fn test_skiplist_sorted() {
        let mut list = SkipList::new();
        list.insert(7, 7);
        list.insert(4, 4);
        list.insert(1, 1);
        list.insert(2, 2);
        list.insert(3, 3);
        list.insert(5, 5);
        list.insert(8, 8);
        list.insert(6, 6);
        let values: Vec<i32> = list.collect().iter().map(|tup| tup.1).collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_skiplist_get() {
        let mut list = SkipList::new();
        list.insert(7, 7);
        list.insert(4, 4);
        list.insert(1, 1);
        list.insert(2, 2);
        list.insert(3, 3);
        list.insert(5, 5);
        list.insert(8, 8);
        list.insert(6, 6);
        let maybe_1 = list.get(&1);
        assert!(maybe_1.is_some());
        assert_eq!(maybe_1.unwrap(), 1);
        let maybe_3 = list.get(&3);
        assert!(maybe_3.is_some());
        assert_eq!(maybe_3.unwrap(), 3);
    }

    #[test]
    fn test_skiplist_delete() {
        let mut list = SkipList::new();
        list.insert(7, 7);
        list.insert(4, 4);
        list.insert(1, 1);
        list.insert(2, 2);
        list.insert(3, 3);
        list.insert(5, 5);
        list.insert(8, 8);
        list.insert(6, 6);
        assert_eq!(list.size, 8);
        list.delete(&1);
        list.delete(&4);
        assert_eq!(list.size, 6);
        let values: Vec<i32> = list.collect().iter().map(|tup| tup.1).collect();
        assert_eq!(values, vec![2, 3, 5, 6, 7, 8]);
    }
}
