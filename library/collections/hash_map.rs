//! `HashMap<K, V>`: a hash table with open addressing.
//!
//! The table has a power-of-two number of slots, each with a control
//! byte: empty, deleted, or the top seven bits of the hash of the entry it
//! holds. A key's probe starts at the slot its hash picks and walks on to
//! the next slots until it finds the key or an empty slot; the control
//! bytes, a small array on their own, let it pass most other keys without
//! touching their entries. Once full and deleted slots together reach
//! seven eighths of the table, it is rebuilt: at twice the size if more
//! than half the slots hold entries, at the same size otherwise, which
//! clears the deleted slots.

use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::{ExactSizeIterator, Extend, FromIterator};
use std::marker::PhantomData;
pub use std::hash::{DefaultHasher, RandomState};

/// The control byte of a slot that has never held an entry.
const EMPTY: u8 = 0x80;
/// The control byte of a slot whose entry was removed: probes go on past it.
const DELETED: u8 = 0xfe;

/// The control byte of a slot holding an entry with this hash.
fn tag(hash: u64) -> u8 {
    (hash >> 57) as u8
}

fn is_full(control: u8) -> bool {
    control & 0x80 == 0
}

fn hash_of<Q: Hash + ?Sized>(key: &Q) -> u64 {
    let mut hasher = TableHasher { state: 0 };
    key.hash(&mut hasher);
    hasher.finish()
}

/// The hasher the table uses for its keys: a word at a time, then mixed
/// so that the low bits, which pick the slot, depend on every input bit.
/// Where a key lands is not observable, so this need not be rustc's.
struct TableHasher {
    state: u64,
}

impl TableHasher {
    fn add(&mut self, word: u64) {
        self.state = (self.state.rotate_left(5) ^ word).wrapping_mul(0x517cc1b727220a95);
    }
}

impl Hasher for TableHasher {
    fn write(&mut self, bytes: &[u8]) {
        let whole = bytes.len() / 8 * 8;
        let mut at = 0;
        while at < whole {
            self.add(unsafe { std::ptr::read_unaligned(bytes.as_ptr().add(at) as *const u64) });
            at += 8;
        }
        let mut last = 0u64;
        for (index, &byte) in bytes[whole..].iter().enumerate() {
            last |= (byte as u64) << (8 * index);
        }
        self.add(last ^ ((bytes.len() as u64) << 56));
    }

    fn write_u8(&mut self, value: u8) {
        self.add(value as u64);
    }

    fn write_u16(&mut self, value: u16) {
        self.add(value as u64);
    }

    fn write_u32(&mut self, value: u32) {
        self.add(value as u64);
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn write_str(&mut self, text: &str) {
        self.write(text.as_bytes());
    }

    fn finish(&self) -> u64 {
        let mut hash = self.state;
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xff51afd7ed558ccd);
        hash ^= hash >> 33;
        hash
    }
}

/// The number of slots a table needs to hold `entries` below its load limit.
fn table_size(entries: usize) -> usize {
    let mut size = 8;
    while size / 8 * 7 < entries {
        size *= 2;
    }
    size
}

/// The slots of a map: their control bytes, and the memory for their
/// entries, which holds one wherever the control byte says full.
struct Table<K, V> {
    control: Vec<u8>,
    entries: *mut (K, V),
    len: usize,
    /// Full and deleted slots together: what the load factor counts.
    used: usize,
}

impl<K, V> Table<K, V> {
    fn with_slots(slots: usize) -> Table<K, V> {
        let mut control = Vec::new();
        control.resize(slots, EMPTY);
        let bytes = slots * std::mem::size_of::<(K, V)>();
        let entries = if slots == 0 { std::ptr::null_mut() } else { unsafe { std::libc::malloc(bytes) as *mut (K, V) } };
        Table { control, entries, len: 0, used: 0 }
    }

    fn slots(&self) -> usize {
        self.control.len()
    }

    fn control_at(&self, position: usize) -> u8 {
        unsafe { *self.control.as_ptr().add(position) }
    }

    fn set_control(&mut self, position: usize, control: u8) {
        unsafe { *self.control.as_mut_ptr().add(position) = control };
    }

    /// The entry at a full slot.
    fn entry(&self, position: usize) -> &(K, V) {
        unsafe { &*self.entries.add(position) }
    }

    fn entry_mut(&mut self, position: usize) -> &mut (K, V) {
        unsafe { &mut *self.entries.add(position) }
    }

    /// The slot holding the entry for `key`, whose hash is `hash`.
    fn find<Q: Eq + ?Sized>(&self, hash: u64, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
    {
        if self.len == 0 {
            return None;
        }
        let mask = self.slots() - 1;
        let wanted = tag(hash);
        let mut position = hash as usize & mask;
        loop {
            let control = self.control_at(position);
            if control == wanted && key.eq(<K as Borrow<Q>>::borrow(&self.entry(position).0)) {
                return Some(position);
            }
            if control == EMPTY {
                return None;
            }
            position = (position + 1) & mask;
        }
    }

    /// The first slot without an entry along the probe for `hash`. The
    /// table must have one.
    fn free_slot(&self, hash: u64) -> usize {
        let mask = self.slots() - 1;
        let mut position = hash as usize & mask;
        while is_full(self.control_at(position)) {
            position = (position + 1) & mask;
        }
        position
    }

    /// Put an entry in a slot `free_slot` gave.
    fn fill(&mut self, position: usize, tag: u8, key: K, value: V) {
        if self.control_at(position) == EMPTY {
            self.used += 1;
        }
        self.set_control(position, tag);
        unsafe { std::ptr::write(self.entries.add(position), (key, value)) };
        self.len += 1;
    }

    /// Take the entry out of a full slot.
    fn take(&mut self, position: usize) -> (K, V) {
        // A probe that reaches this slot goes on to the next. If that one
        // is empty, every probe stops there anyway, so this one can be
        // empty too.
        let next = (position + 1) & (self.slots() - 1);
        if self.control_at(next) == EMPTY {
            self.set_control(position, EMPTY);
            self.used -= 1;
        } else {
            self.set_control(position, DELETED);
        }
        self.len -= 1;
        unsafe { std::ptr::read(self.entries.add(position)) }
    }

    /// Drop every entry, keeping the slots.
    fn clear(&mut self) {
        for position in 0..self.slots() {
            if is_full(self.control_at(position)) {
                unsafe { std::ptr::drop_in_place(self.entries.add(position)) };
            }
            self.set_control(position, EMPTY);
        }
        self.len = 0;
        self.used = 0;
    }
}

impl<K, V> Drop for Table<K, V> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<(K, V)>() && self.len > 0 {
            for position in 0..self.slots() {
                if is_full(self.control_at(position)) {
                    unsafe { std::ptr::drop_in_place(self.entries.add(position)) };
                }
            }
        }
        if !self.entries.is_null() {
            unsafe { std::libc::free(self.entries as *mut u8) };
        }
    }
}

/// A hash table with open addressing and linear probing.
pub struct HashMap<K, V> {
    table: Table<K, V>,
}

impl<K, V> HashMap<K, V> {
    pub fn new() -> HashMap<K, V> {
        HashMap { table: Table::with_slots(0) }
    }

    pub fn with_capacity(capacity: usize) -> HashMap<K, V> {
        if capacity == 0 {
            return HashMap::new();
        }
        HashMap { table: Table::with_slots(table_size(capacity)) }
    }

    pub fn len(&self) -> usize {
        self.table.len
    }

    pub fn is_empty(&self) -> bool {
        self.table.len == 0
    }

    /// How many entries fit before the table is rebuilt.
    pub fn capacity(&self) -> usize {
        self.table.slots() / 8 * 7
    }

    pub fn clear(&mut self) {
        self.table.clear();
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            control: self.table.control.as_ptr(),
            entries: self.table.entries,
            position: 0,
            remaining: self.table.len,
            _map: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            control: self.table.control.as_ptr(),
            entries: self.table.entries,
            position: 0,
            remaining: self.table.len,
            _map: PhantomData,
        }
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

    /// Remove every entry, handing them out by value.
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        let table = std::mem::replace(&mut self.table, Table::with_slots(0));
        Drain { iter: IntoIter { position: 0, table }, _map: PhantomData }
    }

    /// Keep only the entries for which `keep` returns `true`.
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut keep: F) {
        for position in 0..self.table.slots() {
            if is_full(self.table.control_at(position)) {
                let (key, value) = self.table.entry_mut(position);
                if !keep(key, value) {
                    drop(self.table.take(position));
                }
            }
        }
    }
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    /// Make room for `additional` more entries.
    pub fn reserve(&mut self, additional: usize) {
        let size = table_size(self.table.len + additional);
        if size > self.table.slots() {
            self.rebuild(size);
        }
    }

    pub fn shrink_to_fit(&mut self) {
        let size = table_size(self.table.len);
        if size < self.table.slots() {
            self.rebuild(size);
        }
    }

    /// Move every entry to a new table of `size` slots.
    fn rebuild(&mut self, size: usize) {
        let mut old = std::mem::replace(&mut self.table, Table::with_slots(size));
        for position in 0..old.slots() {
            if is_full(old.control_at(position)) {
                let (key, value) = unsafe { std::ptr::read(old.entries.add(position)) };
                let hash = hash_of(&key);
                let free = self.table.free_slot(hash);
                self.table.fill(free, tag(hash), key, value);
            }
        }
        // The entries have moved; only the memory is left to free.
        old.control.clear();
        old.len = 0;
    }

    /// Rebuild the table if one more entry would pass the load limit.
    fn make_room(&mut self) {
        let slots = self.table.slots();
        if (self.table.used + 1) * 8 <= slots * 7 {
            return;
        }
        let size = if slots == 0 {
            8
        } else if (self.table.len + 1) * 2 > slots {
            slots * 2
        } else {
            slots
        };
        self.rebuild(size);
    }

    /// Insert an entry, returning the value the key had before, if any.
    /// The key already in the map is kept.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = hash_of(&key);
        if let Some(position) = self.table.find(hash, &key) {
            return Some(std::mem::replace(&mut self.table.entry_mut(position).1, value));
        }
        self.make_room();
        let position = self.table.free_slot(hash);
        self.table.fill(position, tag(hash), key, value);
        None
    }

    /// The entry for `key`, to read, change, fill or remove in place.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let hash = hash_of(&key);
        if let Some(position) = self.table.find(hash, &key) {
            return Entry::Occupied(OccupiedEntry { map: self, position });
        }
        self.make_room();
        let position = self.table.free_slot(hash);
        Entry::Vacant(VacantEntry { map: self, position, tag: tag(hash), key })
    }

    pub fn get<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        match self.table.find(hash_of(key), key) {
            Some(position) => Some(&self.table.entry(position).1),
            None => None,
        }
    }

    pub fn get_mut<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        match self.table.find(hash_of(key), key) {
            Some(position) => Some(&mut self.table.entry_mut(position).1),
            None => None,
        }
    }

    pub fn get_key_value<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        match self.table.find(hash_of(key), key) {
            Some(position) => {
                let (key, value) = self.table.entry(position);
                Some((key, value))
            }
            None => None,
        }
    }

    pub fn contains_key<Q: Hash + Eq + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.table.find(hash_of(key), key).is_some()
    }

    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        match self.table.find(hash_of(key), key) {
            Some(position) => Some(self.table.take(position).1),
            None => None,
        }
    }

    pub fn remove_entry<Q: Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        match self.table.find(hash_of(key), key) {
            Some(position) => Some(self.table.take(position)),
            None => None,
        }
    }
}

/// An entry of a map, found by [`HashMap::entry`]: occupied or vacant.
pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

pub struct OccupiedEntry<'a, K, V> {
    map: &'a mut HashMap<K, V>,
    position: usize,
}

/// A key not in the map, and the slot its entry will go in: the table
/// has room for it already.
pub struct VacantEntry<'a, K, V> {
    map: &'a mut HashMap<K, V>,
    position: usize,
    tag: u8,
    key: K,
}

impl<'a, K, V> Entry<'a, K, V> {
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

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.map.table.entry(self.position).0
    }

    pub fn get(&self) -> &V {
        &self.map.table.entry(self.position).1
    }

    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.table.entry_mut(self.position).1
    }

    /// The value, borrowed for as long as the map was.
    pub fn into_mut(self) -> &'a mut V {
        &mut self.map.table.entry_mut(self.position).1
    }

    /// Replace the value, returning the old one.
    pub fn insert(&mut self, value: V) -> V {
        std::mem::replace(self.get_mut(), value)
    }

    pub fn remove(self) -> V {
        self.map.table.take(self.position).1
    }

    pub fn remove_entry(self) -> (K, V) {
        self.map.table.take(self.position)
    }
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn into_key(self) -> K {
        self.key
    }

    /// Fill the entry, returning the value just stored.
    pub fn insert(self, value: V) -> &'a mut V {
        self.map.table.fill(self.position, self.tag, self.key, value);
        &mut self.map.table.entry_mut(self.position).1
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> HashMap<K, V> {
        HashMap::new()
    }
}

impl<K: Clone, V: Clone> Clone for HashMap<K, V> {
    fn clone(&self) -> HashMap<K, V> {
        let mut table = Table::with_slots(self.table.slots());
        for position in 0..self.table.slots() {
            let control = self.table.control_at(position);
            if is_full(control) {
                let (key, value) = self.table.entry(position);
                table.fill(position, control, key.clone(), value.clone());
            } else if control == DELETED {
                table.set_control(position, DELETED);
                table.used += 1;
            }
        }
        HashMap { table }
    }
}

impl<K: Hash + Eq, V: PartialEq> PartialEq for HashMap<K, V> {
    fn eq(&self, other: &HashMap<K, V>) -> bool {
        if self.len != other.len {
            return false;
        }
        for (key, value) in self.iter() {
            match other.get(key) {
                Some(theirs) => {
                    if !value.eq(theirs) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}

impl<K: Hash + Eq, V: Eq> Eq for HashMap<K, V> {}

impl<K: Hash + Eq + Borrow<Q>, V, Q: Hash + Eq + ?Sized> std::ops::Index<&Q> for HashMap<K, V> {
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        match self.get(key) {
            Some(value) => value,
            None => panic!("key not found in HashMap"),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for HashMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();
        for (key, value) in self.iter() {
            map.entry(key, value);
        }
        map.finish()
    }
}

impl<K: Hash + Eq, V> FromIterator<(K, V)> for HashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(items: I) -> HashMap<K, V> {
        let mut map = HashMap::new();
        map.extend(items);
        map
    }
}

impl<K: Hash + Eq, V> Extend<(K, V)> for HashMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, items: I) {
        for (key, value) in items {
            self.insert(key, value);
        }
    }

    fn extend_one(&mut self, item: (K, V)) {
        self.insert(item.0, item.1);
    }
}

impl<K: Hash + Eq + Copy, V: Copy> Extend<(&K, &V)> for HashMap<K, V> {
    fn extend<I: IntoIterator<Item = (&K, &V)>>(&mut self, items: I) {
        for (key, value) in items {
            self.insert(*key, *value);
        }
    }

    fn extend_one(&mut self, item: (&K, &V)) {
        self.insert(*item.0, *item.1);
    }
}

/// Iterator over the entries of a map, in no particular order.
pub struct Iter<'a, K, V> {
    control: *const u8,
    entries: *mut (K, V),
    /// The next slot to look at.
    position: usize,
    /// The entries not yet handed out: none left means the end.
    remaining: usize,
    _map: PhantomData<&'a HashMap<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.remaining == 0 {
            return None;
        }
        unsafe {
            while !is_full(*self.control.add(self.position)) {
                self.position += 1;
            }
            let entry = &*self.entries.add(self.position);
            self.position += 1;
            self.remaining -= 1;
            Some((&entry.0, &entry.1))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<'a, K, V> Clone for Iter<'a, K, V> {
    fn clone(&self) -> Iter<'a, K, V> {
        Iter {
            control: self.control,
            entries: self.entries,
            position: self.position,
            remaining: self.remaining,
            _map: PhantomData,
        }
    }
}

pub struct IterMut<'a, K, V> {
    control: *const u8,
    entries: *mut (K, V),
    position: usize,
    remaining: usize,
    _map: PhantomData<&'a mut HashMap<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.remaining == 0 {
            return None;
        }
        unsafe {
            while !is_full(*self.control.add(self.position)) {
                self.position += 1;
            }
            let entry = &mut *self.entries.add(self.position);
            self.position += 1;
            self.remaining -= 1;
            Some((&entry.0, &mut entry.1))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}

/// Iterator that takes the entries of a map by value. Those it has not
/// handed out are dropped with it.
pub struct IntoIter<K, V> {
    table: Table<K, V>,
    position: usize,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        if self.table.len == 0 {
            return None;
        }
        while !is_full(self.table.control_at(self.position)) {
            self.position += 1;
        }
        self.table.set_control(self.position, EMPTY);
        self.table.len -= 1;
        let entry = unsafe { std::ptr::read(self.table.entries.add(self.position)) };
        self.position += 1;
        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.table.len, Some(self.table.len))
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.table.len
    }
}

pub struct Drain<'a, K, V> {
    iter: IntoIter<K, V>,
    _map: PhantomData<&'a mut HashMap<K, V>>,
}

impl<'a, K, V> Iterator for Drain<'a, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
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

impl<K, V> IntoIterator for HashMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter { table: self.table, position: 0 }
    }
}

impl<'a, K, V> IntoIterator for &'a HashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut HashMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<K: Hash + Eq, V, const N: usize> From<[(K, V); N]> for HashMap<K, V> {
    fn from(entries: [(K, V); N]) -> HashMap<K, V> {
        let mut map = HashMap::with_capacity(N);
        map.extend(entries);
        map
    }
}
