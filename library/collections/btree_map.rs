//! `BTreeMap<K, V>`: an ordered map in a B-tree.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::iter::{DoubleEndedIterator, ExactSizeIterator, Extend, FromIterator};
use std::ops::{Bound, RangeBounds};

/// Every node but the root holds between `B - 1` and `2B - 1` entries.
const B: usize = 6;
const CAPACITY: usize = 2 * B - 1;
const MIN_LEN: usize = B - 1;

/// A node of the tree. A leaf has no children; any other node has one more
/// child than it has entries, and its entries separate those of the
/// children: everything in `children[i]` sorts between `keys[i - 1]` and
/// `keys[i]`.
struct Node<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
    children: Vec<Node<K, V>>,
}

impl<K, V> Node<K, V> {
    fn new() -> Node<K, V> {
        Node { keys: Vec::new(), values: Vec::new(), children: Vec::new() }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Where `key` is among this node's keys: `Ok` with its position, or
    /// `Err` with the position of the child whose subtree would hold it.
    fn search<Q: Ord + ?Sized>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
    {
        let mut index = 0;
        while index < self.keys.len() {
            match key.cmp(<K as Borrow<Q>>::borrow(&self.keys[index])) {
                Ordering::Greater => index += 1,
                Ordering::Equal => return Ok(index),
                Ordering::Less => return Err(index),
            }
        }
        Err(index)
    }

    /// How many of this node's keys sort before `key`, counting a key equal
    /// to it when `equal_too` holds.
    fn count_before<Q: Ord + ?Sized>(&self, key: &Q, equal_too: bool) -> usize
    where
        K: Borrow<Q>,
    {
        match self.search(key) {
            Ok(index) => {
                if equal_too {
                    index + 1
                } else {
                    index
                }
            }
            Err(index) => index,
        }
    }

    /// Split the full child at `index` in two around its middle entry,
    /// which moves up into this node.
    fn split_child(&mut self, index: usize) {
        let child = &mut self.children[index];
        let keys = child.keys.split_off(B);
        let values = child.values.split_off(B);
        let children = if child.is_leaf() { Vec::new() } else { child.children.split_off(B) };
        let middle_key = child.keys.pop().unwrap();
        let middle_value = child.values.pop().unwrap();
        self.keys.insert(index, middle_key);
        self.values.insert(index, middle_value);
        self.children.insert(index + 1, Node { keys, values, children });
    }

    /// Merge the child after `index` and the entry between the two into the
    /// child at `index`.
    fn merge_children(&mut self, index: usize) {
        let mut right = self.children.remove(index + 1);
        let key = self.keys.remove(index);
        let value = self.values.remove(index);
        let left = &mut self.children[index];
        left.keys.push(key);
        left.values.push(value);
        left.keys.append(&mut right.keys);
        left.values.append(&mut right.values);
        left.children.append(&mut right.children);
    }

    /// Before descending into the child at `index`, make sure it has an
    /// entry to spare, taking one through this node from a sibling or
    /// merging it with one. Returns the child's position afterwards.
    fn fill_child(&mut self, index: usize) -> usize {
        if self.children[index].keys.len() > MIN_LEN {
            return index;
        }
        if index > 0 && self.children[index - 1].keys.len() > MIN_LEN {
            let left = &mut self.children[index - 1];
            let key = left.keys.pop().unwrap();
            let value = left.values.pop().unwrap();
            let grandchild = left.children.pop();
            let key = std::mem::replace(&mut self.keys[index - 1], key);
            let value = std::mem::replace(&mut self.values[index - 1], value);
            let child = &mut self.children[index];
            child.keys.insert(0, key);
            child.values.insert(0, value);
            if let Some(grandchild) = grandchild {
                child.children.insert(0, grandchild);
            }
            return index;
        }
        if index + 1 < self.children.len() && self.children[index + 1].keys.len() > MIN_LEN {
            let right = &mut self.children[index + 1];
            let key = right.keys.remove(0);
            let value = right.values.remove(0);
            let grandchild = if right.is_leaf() { None } else { Some(right.children.remove(0)) };
            let key = std::mem::replace(&mut self.keys[index], key);
            let value = std::mem::replace(&mut self.values[index], value);
            let child = &mut self.children[index];
            child.keys.push(key);
            child.values.push(value);
            if let Some(grandchild) = grandchild {
                child.children.push(grandchild);
            }
            return index;
        }
        if index + 1 < self.children.len() {
            self.merge_children(index);
            index
        } else {
            self.merge_children(index - 1);
            index - 1
        }
    }

    /// Remove the least entry of this subtree. Unless this is the root, it
    /// must have an entry to spare.
    fn pop_first(&mut self) -> Option<(K, V)> {
        if self.is_leaf() {
            if self.keys.is_empty() {
                return None;
            }
            return Some((self.keys.remove(0), self.values.remove(0)));
        }
        let index = self.fill_child(0);
        self.children[index].pop_first()
    }

    /// Remove the greatest entry of this subtree, under the same condition.
    fn pop_last(&mut self) -> Option<(K, V)> {
        if self.is_leaf() {
            let key = self.keys.pop()?;
            return Some((key, self.values.pop().unwrap()));
        }
        let index = self.fill_child(self.children.len() - 1);
        self.children[index].pop_last()
    }

    /// Remove the entry for `key` from this subtree, refilling each node on
    /// the way down so that none ends up short of entries.
    fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        match self.search(key) {
            Ok(index) => {
                if self.is_leaf() {
                    return Some((self.keys.remove(index), self.values.remove(index)));
                }
                // An entry inside the tree is replaced by its neighbour from
                // a leaf, taken from whichever side has one to spare.
                let neighbour = if self.children[index].keys.len() > MIN_LEN {
                    self.children[index].pop_last()
                } else if self.children[index + 1].keys.len() > MIN_LEN {
                    self.children[index + 1].pop_first()
                } else {
                    self.merge_children(index);
                    return self.children[index].remove(key);
                };
                let (neighbour_key, neighbour_value) = neighbour.unwrap();
                let key = std::mem::replace(&mut self.keys[index], neighbour_key);
                let value = std::mem::replace(&mut self.values[index], neighbour_value);
                Some((key, value))
            }
            Err(index) => {
                if self.is_leaf() {
                    return None;
                }
                let index = self.fill_child(index);
                self.children[index].remove(key)
            }
        }
    }
}

impl<K: Ord, V> Node<K, V> {
    /// Insert into the subtree of this node, which must not be full.
    /// Returns where the value is stored and the value it replaced.
    fn insert(&mut self, key: K, value: V) -> (*mut V, Option<V>) {
        let mut node = self;
        loop {
            let mut index = match node.search(&key) {
                Ok(index) => {
                    let old = std::mem::replace(&mut node.values[index], value);
                    return (&mut node.values[index] as *mut V, Some(old));
                }
                Err(index) => index,
            };
            if node.is_leaf() {
                node.keys.insert(index, key);
                node.values.insert(index, value);
                return (&mut node.values[index] as *mut V, None);
            }
            if node.children[index].keys.len() == CAPACITY {
                node.split_child(index);
                match key.cmp(&node.keys[index]) {
                    Ordering::Less => {}
                    Ordering::Equal => {
                        let old = std::mem::replace(&mut node.values[index], value);
                        return (&mut node.values[index] as *mut V, Some(old));
                    }
                    Ordering::Greater => index += 1,
                }
            }
            node = &mut node.children[index];
        }
    }
}

impl<K: Clone, V: Clone> Clone for Node<K, V> {
    fn clone(&self) -> Node<K, V> {
        Node { keys: self.keys.clone(), values: self.values.clone(), children: self.children.clone() }
    }
}

/// Move the entries of a subtree, in order, onto the end of `entries`.
fn flatten<K, V>(node: Node<K, V>, entries: &mut Vec<(K, V)>) {
    let Node { keys, values, children } = node;
    let mut children = children.into_iter();
    for (key, value) in keys.into_iter().zip(values.into_iter()) {
        if let Some(child) = children.next() {
            flatten(child, entries);
        }
        entries.push((key, value));
    }
    if let Some(child) = children.next() {
        flatten(child, entries);
    }
}

/// A path from the root down to a leaf: each node with the position taken
/// in it, a child's for an inner node and a gap between entries for the
/// leaf. The leaf's gap is a position between two entries of the map, and
/// every such position has exactly one path.
type Path<K, V> = Vec<(*const Node<K, V>, usize)>;

fn push_leftmost<K, V>(path: &mut Path<K, V>, node: &Node<K, V>) {
    let mut node = node;
    loop {
        path.push((node as *const Node<K, V>, 0));
        if node.is_leaf() {
            return;
        }
        node = &node.children[0];
    }
}

fn push_rightmost<K, V>(path: &mut Path<K, V>, node: &Node<K, V>) {
    let mut node = node;
    loop {
        let last = node.keys.len();
        path.push((node as *const Node<K, V>, last));
        if node.is_leaf() {
            return;
        }
        node = &node.children[last];
    }
}

/// The path to the position a range bound marks: before the first key the
/// range starts with, or after the last one it ends with.
fn seek<K: Borrow<Q>, V, Q: Ord + ?Sized>(root: &Node<K, V>, bound: Bound<&Q>, is_end: bool) -> Path<K, V> {
    let mut path = Vec::new();
    let mut node = root;
    loop {
        let index = match bound {
            Bound::Included(key) => node.count_before(key, is_end),
            Bound::Excluded(key) => node.count_before(key, !is_end),
            Bound::Unbounded => {
                if is_end {
                    node.keys.len()
                } else {
                    0
                }
            }
        };
        path.push((node as *const Node<K, V>, index));
        if node.is_leaf() {
            return path;
        }
        node = &node.children[index];
    }
}

/// The entries between two positions of a tree, which every iterator over
/// borrowed entries consumes from either end. An entry is handed out as
/// its node and its index there.
struct Edges<K, V> {
    front: Path<K, V>,
    back: Path<K, V>,
}

impl<K, V> Edges<K, V> {
    fn all(root: &Node<K, V>) -> Edges<K, V> {
        let mut front = Vec::new();
        let mut back = Vec::new();
        push_leftmost(&mut front, root);
        push_rightmost(&mut back, root);
        Edges { front, back }
    }

    fn between<Q: Ord + ?Sized>(root: &Node<K, V>, start: Bound<&Q>, end: Bound<&Q>) -> Edges<K, V>
    where
        K: Borrow<Q>,
    {
        let first = match start {
            Bound::Included(key) => Some((key, false)),
            Bound::Excluded(key) => Some((key, true)),
            Bound::Unbounded => None,
        };
        let last = match end {
            Bound::Included(key) => Some((key, false)),
            Bound::Excluded(key) => Some((key, true)),
            Bound::Unbounded => None,
        };
        if let (Some((first, first_excluded)), Some((last, last_excluded))) = (first, last) {
            match first.cmp(last) {
                Ordering::Greater => panic!("range start is greater than range end in BTreeMap"),
                Ordering::Equal => {
                    if first_excluded && last_excluded {
                        panic!("range start and end are equal and excluded in BTreeMap");
                    }
                }
                Ordering::Less => {}
            }
        }
        Edges { front: seek(root, start, false), back: seek(root, end, true) }
    }

    fn is_empty(&self) -> bool {
        let (front_leaf, front_gap) = self.front[self.front.len() - 1];
        let (back_leaf, back_gap) = self.back[self.back.len() - 1];
        front_leaf == back_leaf && front_gap == back_gap
    }

    /// The entry after the front position, which then moves past it.
    fn next(&mut self) -> Option<(*const Node<K, V>, usize)> {
        if self.is_empty() {
            return None;
        }
        loop {
            let top = self.front.len() - 1;
            let (node, index) = self.front[top];
            let node_ref = unsafe { &*node };
            if index < node_ref.keys.len() {
                self.front[top].1 = index + 1;
                if !node_ref.is_leaf() {
                    push_leftmost(&mut self.front, &node_ref.children[index + 1]);
                }
                return Some((node, index));
            }
            self.front.pop();
        }
    }

    /// The entry before the back position, which then moves before it.
    fn next_back(&mut self) -> Option<(*const Node<K, V>, usize)> {
        if self.is_empty() {
            return None;
        }
        loop {
            let top = self.back.len() - 1;
            let (node, index) = self.back[top];
            let node_ref = unsafe { &*node };
            if index > 0 {
                self.back[top].1 = index - 1;
                if !node_ref.is_leaf() {
                    push_rightmost(&mut self.back, &node_ref.children[index - 1]);
                }
                return Some((node, index - 1));
            }
            self.back.pop();
        }
    }
}

fn entry_at<'a, K, V>(node: *const Node<K, V>, index: usize) -> (&'a K, &'a V) {
    let node = unsafe { &*node };
    (&node.keys[index], &node.values[index])
}

fn entry_at_mut<'a, K, V>(node: *const Node<K, V>, index: usize) -> (&'a K, &'a mut V) {
    let node = unsafe { &mut *(node as *mut Node<K, V>) };
    (&node.keys[index], &mut node.values[index])
}

/// A map whose entries are kept sorted by key.
pub struct BTreeMap<K, V> {
    root: Node<K, V>,
    len: usize,
}

impl<K, V> BTreeMap<K, V> {
    pub fn new() -> BTreeMap<K, V> {
        BTreeMap { root: Node::new(), len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.root = Node::new();
        self.len = 0;
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { edges: Edges::all(&self.root), remaining: self.len }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut { edges: Edges::all(&self.root), remaining: self.len }
    }

    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { iter: self.iter() }
    }

    pub fn values(&self) -> Values<'_, K, V> {
        Values { iter: self.iter() }
    }

    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut { iter: self.iter_mut() }
    }

    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys { iter: self.into_iter() }
    }

    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues { iter: self.into_iter() }
    }

    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        let mut node = &self.root;
        while !node.is_leaf() {
            node = &node.children[0];
        }
        if node.keys.is_empty() {
            None
        } else {
            Some((&node.keys[0], &node.values[0]))
        }
    }

    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        let mut node = &self.root;
        while !node.is_leaf() {
            node = &node.children[node.keys.len()];
        }
        let last = node.keys.len();
        if last == 0 {
            None
        } else {
            Some((&node.keys[last - 1], &node.values[last - 1]))
        }
    }

    pub fn pop_first(&mut self) -> Option<(K, V)> {
        let entry = self.root.pop_first();
        self.after_removal(entry.is_some());
        entry
    }

    pub fn pop_last(&mut self) -> Option<(K, V)> {
        let entry = self.root.pop_last();
        self.after_removal(entry.is_some());
        entry
    }

    /// Account for a removal, dropping a root that merging left empty.
    fn after_removal(&mut self, removed: bool) {
        if removed {
            self.len -= 1;
        }
        if self.root.keys.is_empty() && !self.root.is_leaf() {
            self.root = self.root.children.pop().unwrap();
        }
    }

    /// Where the entry for `key` is: its node and its index there.
    fn find<Q: Ord + ?Sized>(&self, key: &Q) -> Option<(*const Node<K, V>, usize)>
    where
        K: Borrow<Q>,
    {
        let mut node = &self.root;
        loop {
            match node.search(key) {
                Ok(index) => return Some((node as *const Node<K, V>, index)),
                Err(index) => {
                    if node.is_leaf() {
                        return None;
                    }
                    node = &node.children[index];
                }
            }
        }
    }

    pub fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        match self.find(key) {
            Some((node, index)) => Some(entry_at(node, index).1),
            None => None,
        }
    }

    pub fn get_mut<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        match self.find(key) {
            Some((node, index)) => Some(entry_at_mut(node, index).1),
            None => None,
        }
    }

    pub fn get_key_value<Q: Ord + ?Sized>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        match self.find(key) {
            Some((node, index)) => Some(entry_at(node, index)),
            None => None,
        }
    }

    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.find(key).is_some()
    }

    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        match self.remove_entry(key) {
            Some((_, value)) => Some(value),
            None => None,
        }
    }

    pub fn remove_entry<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        let entry = self.root.remove(key);
        self.after_removal(entry.is_some());
        entry
    }

    /// The entries whose keys fall in `range`, in order.
    pub fn range<Q: Ord + ?Sized, R: RangeBounds<Q>>(&self, range: R) -> Range<'_, K, V>
    where
        K: Borrow<Q>,
    {
        Range { edges: Edges::between(&self.root, range.start_bound(), range.end_bound()) }
    }

    pub fn range_mut<Q: Ord + ?Sized, R: RangeBounds<Q>>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        K: Borrow<Q>,
    {
        RangeMut { edges: Edges::between(&self.root, range.start_bound(), range.end_bound()) }
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    /// Insert an entry, returning the value the key had before, if any.
    /// The key already in the map is kept.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert_entry(key, value).1
    }

    /// Insert an entry; returns where the value is stored and the value it
    /// replaced.
    fn insert_entry(&mut self, key: K, value: V) -> (*mut V, Option<V>) {
        if self.root.keys.len() == CAPACITY {
            let old_root = std::mem::replace(&mut self.root, Node::new());
            self.root.children.push(old_root);
            self.root.split_child(0);
        }
        let (slot, old) = self.root.insert(key, value);
        if old.is_none() {
            self.len += 1;
        }
        (slot, old)
    }

    /// The entry for `key`, to read, change, fill or remove in place.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        match self.find(&key) {
            Some((node, index)) => {
                let (stored, value) = entry_at_mut(node, index);
                Entry::Occupied(OccupiedEntry { key: stored as *const K, value: value as *mut V, probe: key, map: self })
            }
            None => Entry::Vacant(VacantEntry { map: self, key }),
        }
    }

    /// Keep only the entries for which `keep` returns `true`.
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut keep: F) {
        let entries = std::mem::replace(self, BTreeMap::new());
        for (key, mut value) in entries {
            if keep(&key, &mut value) {
                self.insert(key, value);
            }
        }
    }

    /// Move every entry of `other` into this map; its values win.
    pub fn append(&mut self, other: &mut BTreeMap<K, V>) {
        for (key, value) in std::mem::replace(other, BTreeMap::new()) {
            self.insert(key, value);
        }
    }

    /// The entries from `key` on, as a new map; this one keeps the rest.
    pub fn split_off<Q: Ord + ?Sized>(&mut self, key: &Q) -> BTreeMap<K, V>
    where
        K: Borrow<Q>,
    {
        let mut tail = BTreeMap::new();
        for (stored, value) in std::mem::replace(self, BTreeMap::new()) {
            if <K as Borrow<Q>>::borrow(&stored) < key {
                self.insert(stored, value);
            } else {
                tail.insert(stored, value);
            }
        }
        tail
    }
}

/// An entry of a map, found by [`BTreeMap::entry`]: occupied or vacant.
pub enum Entry<'a, K, V> {
    Vacant(VacantEntry<'a, K, V>),
    Occupied(OccupiedEntry<'a, K, V>),
}

pub struct VacantEntry<'a, K, V> {
    map: &'a mut BTreeMap<K, V>,
    key: K,
}

pub struct OccupiedEntry<'a, K, V> {
    map: &'a mut BTreeMap<K, V>,
    key: *const K,
    value: *mut V,
    /// The key the entry was looked up with, for finding it again to
    /// remove it: removal moves entries between nodes, the stored key too.
    probe: K,
}

impl<'a, K: Ord, V> Entry<'a, K, V> {
    /// The value, inserting `default` first if the entry is vacant.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, make: F) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(make()),
        }
    }

    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, make: F) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = make(&entry.key);
                entry.insert(value)
            }
        }
    }

    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(V::default()),
        }
    }

    /// Apply `change` to the value if the entry is occupied.
    pub fn and_modify<F: FnOnce(&mut V)>(self, change: F) -> Entry<'a, K, V> {
        match self {
            Entry::Occupied(mut entry) => {
                change(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }
}

impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn into_key(self) -> K {
        self.key
    }

    /// Fill the entry, returning the value just stored.
    pub fn insert(self, value: V) -> &'a mut V {
        let (slot, _) = self.map.insert_entry(self.key, value);
        unsafe { &mut *slot }
    }
}

impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        unsafe { &*self.key }
    }

    pub fn get(&self) -> &V {
        unsafe { &*self.value }
    }

    pub fn get_mut(&mut self) -> &mut V {
        unsafe { &mut *self.value }
    }

    /// The value, borrowed for as long as the map was.
    pub fn into_mut(self) -> &'a mut V {
        unsafe { &mut *self.value }
    }

    /// Replace the value, returning the old one.
    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }

    pub fn remove(self) -> V {
        self.remove_entry().1
    }

    pub fn remove_entry(self) -> (K, V) {
        self.map.remove_entry(&self.probe).unwrap()
    }
}

impl<K, V> Default for BTreeMap<K, V> {
    fn default() -> BTreeMap<K, V> {
        BTreeMap::new()
    }
}

impl<K: Clone, V: Clone> Clone for BTreeMap<K, V> {
    fn clone(&self) -> BTreeMap<K, V> {
        BTreeMap { root: self.root.clone(), len: self.len }
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for BTreeMap<K, V> {
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        self.len == other.len && self.iter().zip(other.iter()).all(|(a, b)| a.0 == b.0 && a.1 == b.1)
    }
}

impl<K: Eq, V: Eq> Eq for BTreeMap<K, V> {}

impl<K: Ord + Borrow<Q>, V, Q: Ord + ?Sized> std::ops::Index<&Q> for BTreeMap<K, V> {
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        match self.get(key) {
            Some(value) => value,
            None => panic!("key not found in BTreeMap"),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for BTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for BTreeMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> BTreeMap<K, V> {
        let mut map = BTreeMap::new();
        map.extend(entries);
        map
    }
}

impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, entries: I) {
        for (key, value) in entries {
            self.insert(key, value);
        }
    }

    fn extend_one(&mut self, entry: (K, V)) {
        self.insert(entry.0, entry.1);
    }
}

impl<K: Ord + Copy, V: Copy> Extend<(&K, &V)> for BTreeMap<K, V> {
    fn extend<I: IntoIterator<Item = (&K, &V)>>(&mut self, entries: I) {
        for (key, value) in entries {
            self.insert(*key, *value);
        }
    }

    fn extend_one(&mut self, entry: (&K, &V)) {
        self.insert(*entry.0, *entry.1);
    }
}

/// Iterator over the entries of a map, in key order.
pub struct Iter<'a, K, V> {
    edges: Edges<K, V>,
    remaining: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        let (node, index) = self.edges.next()?;
        self.remaining -= 1;
        Some(entry_at(node, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        let (node, index) = self.edges.next_back()?;
        self.remaining -= 1;
        Some(entry_at(node, index))
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}

pub struct IterMut<'a, K, V> {
    edges: Edges<K, V>,
    remaining: usize,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (node, index) = self.edges.next()?;
        self.remaining -= 1;
        Some(entry_at_mut(node, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (node, index) = self.edges.next_back()?;
        self.remaining -= 1;
        Some(entry_at_mut(node, index))
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}

/// Iterator over the entries of a map within a range of keys.
pub struct Range<'a, K, V> {
    edges: Edges<K, V>,
}

impl<'a, K, V> Iterator for Range<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        let (node, index) = self.edges.next()?;
        Some(entry_at(node, index))
    }
}

impl<'a, K, V> DoubleEndedIterator for Range<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        let (node, index) = self.edges.next_back()?;
        Some(entry_at(node, index))
    }
}

pub struct RangeMut<'a, K, V> {
    edges: Edges<K, V>,
}

impl<'a, K, V> Iterator for RangeMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (node, index) = self.edges.next()?;
        Some(entry_at_mut(node, index))
    }
}

impl<'a, K, V> DoubleEndedIterator for RangeMut<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (node, index) = self.edges.next_back()?;
        Some(entry_at_mut(node, index))
    }
}

/// Iterator that takes the entries of a map by value, in key order.
pub struct IntoIter<K, V> {
    entries: std::vec::IntoIter<(K, V)>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.entries.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entries.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    fn next_back(&mut self) -> Option<(K, V)> {
        self.entries.next_back()
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Defines an iterator that yields one half of each entry of another.
macro_rules! projection {
    ($name:ident [$($life:lifetime)?] over $inner:ident yields $item:ty, |$entry:pat_param| $part:expr) => {
        pub struct $name<$($life,)? K, V> {
            iter: $inner<$($life,)? K, V>,
        }

        impl<$($life,)? K, V> Iterator for $name<$($life,)? K, V> {
            type Item = $item;

            fn next(&mut self) -> Option<$item> {
                match self.iter.next() {
                    Some($entry) => Some($part),
                    None => None,
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<$($life,)? K, V> DoubleEndedIterator for $name<$($life,)? K, V> {
            fn next_back(&mut self) -> Option<$item> {
                match self.iter.next_back() {
                    Some($entry) => Some($part),
                    None => None,
                }
            }
        }

        impl<$($life,)? K, V> ExactSizeIterator for $name<$($life,)? K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
    };
}

projection!(Keys ['a] over Iter yields &'a K, |(key, _)| key);
projection!(Values ['a] over Iter yields &'a V, |(_, value)| value);
projection!(ValuesMut ['a] over IterMut yields &'a mut V, |(_, value)| value);
projection!(IntoKeys [] over IntoIter yields K, |(key, _)| key);
projection!(IntoValues [] over IntoIter yields V, |(_, value)| value);

impl<K, V> IntoIterator for BTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> IntoIter<K, V> {
        let mut entries = Vec::with_capacity(self.len);
        let BTreeMap { root, len: _ } = self;
        flatten(root, &mut entries);
        IntoIter { entries: entries.into_iter() }
    }
}

impl<'a, K, V> IntoIterator for &'a BTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut BTreeMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<K: Ord, V, const N: usize> From<[(K, V); N]> for BTreeMap<K, V> {
    fn from(entries: [(K, V); N]) -> BTreeMap<K, V> {
        entries.into_iter().collect()
    }
}
