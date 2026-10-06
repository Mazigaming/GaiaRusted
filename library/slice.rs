//! Slices: views into a run of elements.

/// The slice of `len` elements starting at `data`.
pub unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T] {
    std::intrinsics::slice_from_raw_parts(data, len)
}

pub unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T] {
    std::intrinsics::slice_from_raw_parts_mut(data, len)
}

/// A slice of one element: the value `value` points to.
pub fn from_ref<T>(value: &T) -> &[T] {
    std::intrinsics::slice_from_raw_parts(value as *const T, 1)
}

impl<T> [T] {
    pub fn len(&self) -> usize {
        std::intrinsics::slice_len(self)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ptr(&self) -> *const T {
        std::intrinsics::slice_as_ptr(self)
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        std::intrinsics::slice_as_ptr(self) as *mut T
    }

    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }

    pub fn last(&self) -> Option<&T> {
        if self.len() == 0 { None } else { self.get(self.len() - 1) }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len() { Some(&self[index]) } else { None }
    }

    /// The element at `index`, which the caller guarantees is in bounds.
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        &*self.as_ptr().add(index)
    }

    /// The element at `index`, which the caller guarantees is in bounds.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        &mut *self.as_mut_ptr().add(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len() { Some(&mut self[index]) } else { None }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter { pointer: self.as_ptr(), next: 0, len: self.len() }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut { pointer: self.as_mut_ptr(), next: 0, len: self.len() }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a >= self.len() || b >= self.len() {
            panic!("swap index out of bounds: the len is {} but the indices are {} and {}", self.len(), a, b);
        }
        let pointer = self.as_mut_ptr();
        let saved = pointer.add(a).read();
        pointer.add(a).write(pointer.add(b).read());
        pointer.add(b).write(saved);
    }

    pub fn reverse(&mut self) {
        let len = self.len();
        let mut index = 0;
        while index < len / 2 {
            self.swap(index, len - 1 - index);
            index += 1;
        }
    }

    pub fn split_at(&self, middle: usize) -> (&[T], &[T]) {
        (self.subslice(0, middle), self.subslice(middle, self.len()))
    }

    pub fn split_at_mut(&mut self, middle: usize) -> (&mut [T], &mut [T]) {
        if middle > self.len() {
            panic!("split index {} is out of bounds of a slice of length {}", middle, self.len());
        }
        let pointer = self.as_mut_ptr();
        let rest = self.len() - middle;
        (
            std::intrinsics::slice_from_raw_parts_mut(pointer, middle),
            std::intrinsics::slice_from_raw_parts_mut(pointer.add(middle), rest),
        )
    }

    /// The elements from `start` up to (not including) `end`.
    fn subslice(&self, start: usize, end: usize) -> &[T] {
        if start > end || end > self.len() {
            panic!("range {}..{} is out of bounds of a slice of length {}", start, end, self.len());
        }
        std::intrinsics::slice_from_raw_parts(self.as_ptr().add(start), end - start)
    }
}

impl [&str] {
    pub fn concat(&self) -> String {
        self.join("")
    }

    /// The strings joined into one, with `separator` between them.
    pub fn join(&self, separator: &str) -> String {
        let mut joined = String::new();
        let mut index = 0;
        while index < self.len() {
            if index > 0 {
                joined.push_str(separator);
            }
            joined.push_str(self[index]);
            index += 1;
        }
        joined
    }
}

impl [String] {
    pub fn concat(&self) -> String {
        self.join("")
    }

    pub fn join(&self, separator: &str) -> String {
        let mut joined = String::new();
        let mut index = 0;
        while index < self.len() {
            if index > 0 {
                joined.push_str(separator);
            }
            joined.push_str(self[index].as_str());
            index += 1;
        }
        joined
    }
}

impl<T: PartialEq> [T] {
    pub fn contains(&self, wanted: &T) -> bool {
        let mut index = 0;
        while index < self.len() {
            if self[index] == *wanted {
                return true;
            }
            index += 1;
        }
        false
    }
}

impl<T: Clone> [T] {
    pub fn to_vec(&self) -> Vec<T> {
        let mut vec = Vec::with_capacity(self.len());
        vec.extend_from_slice(self);
        vec
    }
}

impl<T> [T] {
    /// Sort with a comparison, keeping equal elements in their order.
    /// Insertion sort for short runs, merge sort above that: O(n log n)
    /// in the worst case.
    /// Sort, keeping equal elements in their order: a merge sort.
    pub fn sort_by<F: FnMut(&T, &T) -> std::cmp::Ordering>(&mut self, compare: F) {
        let mut compare = compare;
        let len = self.len();
        let size = std::intrinsics::size_of::<T>();
        if len < 2 || size == 0 {
            return;
        }
        if len <= INSERTION_SORT_LEN {
            insertion_sort(self, &mut compare);
            return;
        }
        let scratch = unsafe { std::libc::malloc(len / 2 * size) as *mut T };
        merge_sort(self, scratch, &mut compare);
        unsafe { std::libc::free(scratch as *mut u8) }
    }

    pub fn sort_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, key: F) {
        let mut key = key;
        self.sort_by(|a, b| key(a).cmp(&key(b)));
    }

    pub fn sort_unstable_by<F: FnMut(&T, &T) -> std::cmp::Ordering>(&mut self, compare: F) {
        self.sort_by(compare);
    }

    pub fn sort_unstable_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, key: F) {
        self.sort_by_key(key);
    }

    /// Where `compare` puts the probe: `Ok` with the position of an equal
    /// element, or `Err` with where it would be inserted.
    pub fn binary_search_by<F: FnMut(&T) -> std::cmp::Ordering>(&self, compare: F) -> Result<usize, usize> {
        let mut compare = compare;
        let (mut low, mut high) = (0, self.len());
        while low < high {
            let middle = low + (high - low) / 2;
            match compare(&self[middle]) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Ok(middle),
            }
        }
        Err(low)
    }

    pub fn binary_search_by_key<K: Ord, F: FnMut(&T) -> K>(&self, wanted: &K, key: F) -> Result<usize, usize> {
        let mut key = key;
        self.binary_search_by(|element| key(element).cmp(wanted))
    }

    /// The index of the first element for which `predicate` is false, in a
    /// slice where it is true for a prefix.
    pub fn partition_point<P: FnMut(&T) -> bool>(&self, predicate: P) -> usize {
        let mut predicate = predicate;
        let (mut low, mut high) = (0, self.len());
        while low < high {
            let middle = low + (high - low) / 2;
            if predicate(&self[middle]) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        let len = self.len();
        if len == 0 { None } else { self.get_mut(len - 1) }
    }

    pub fn split_first(&self) -> Option<(&T, &[T])> {
        if self.is_empty() { None } else { Some((&self[0], self.subslice(1, self.len()))) }
    }

    pub fn split_last(&self) -> Option<(&T, &[T])> {
        let len = self.len();
        if len == 0 { None } else { Some((&self[len - 1], self.subslice(0, len - 1))) }
    }

    pub fn chunks(&self, size: usize) -> Chunks<T> {
        if size == 0 {
            panic!("chunk size must be non-zero");
        }
        Chunks { rest: self, size }
    }

    /// Chunks of exactly `size`; what does not fill one is left out.
    pub fn chunks_exact(&self, size: usize) -> Chunks<T> {
        if size == 0 {
            panic!("chunk size must be non-zero");
        }
        let whole = self.len() - self.len() % size;
        Chunks { rest: self.subslice(0, whole), size }
    }

    /// Chunks counted from the end.
    pub fn rchunks(&self, size: usize) -> RChunks<T> {
        if size == 0 {
            panic!("chunk size must be non-zero");
        }
        RChunks { rest: self, size }
    }

    pub fn chunks_mut(&mut self, size: usize) -> ChunksMut<T> {
        if size == 0 {
            panic!("chunk size must be non-zero");
        }
        ChunksMut { pointer: self.as_mut_ptr(), len: self.len(), size }
    }

    /// Every run of `size` consecutive elements.
    /// The parts between the elements `separator` accepts.
    pub fn split<P: FnMut(&T) -> bool>(&self, separator: P) -> Split<T, P> {
        Split { rest: Some(self), separator, parts_left: usize::MAX }
    }

    /// Like [`split`](Self::split), into at most `count` parts: the last
    /// one is all that remains.
    pub fn splitn<P: FnMut(&T) -> bool>(&self, count: usize, separator: P) -> Split<T, P> {
        Split { rest: if count == 0 { None } else { Some(self) }, separator, parts_left: count }
    }

    /// Like [`split`](Self::split), from the end.
    pub fn rsplit<P: FnMut(&T) -> bool>(&self, separator: P) -> RSplit<T, P> {
        RSplit { rest: Some(self), separator }
    }

    /// The parts after each element `separator` accepts, each ending with
    /// that element.
    pub fn split_inclusive<P: FnMut(&T) -> bool>(&self, separator: P) -> SplitInclusive<T, P> {
        SplitInclusive { rest: self, separator }
    }

    pub fn windows(&self, size: usize) -> Windows<T> {
        if size == 0 {
            panic!("window size must be non-zero");
        }
        Windows { slice: self, start: 0, size }
    }

    pub fn rotate_left(&mut self, by: usize) {
        let len = self.len();
        if len == 0 {
            return;
        }
        let by = by % len;
        self[..by].reverse();
        self[by..].reverse();
        self.reverse();
    }

    pub fn rotate_right(&mut self, by: usize) {
        let len = self.len();
        if len == 0 {
            return;
        }
        self.rotate_left(len - by % len);
    }

    pub fn swap_with_slice(&mut self, other: &mut [T]) {
        if self.len() != other.len() {
            panic!("destination and source slices have different lengths");
        }
        let mut index = 0;
        while index < self.len() {
            let a = self.as_mut_ptr().add(index);
            let b = other.as_mut_ptr().add(index);
            let saved = a.read();
            a.write(b.read());
            b.write(saved);
            index += 1;
        }
    }
}

impl<T: Ord> [T] {
    pub fn sort(&mut self) {
        self.sort_by(|a, b| a.cmp(b));
    }

    pub fn sort_unstable(&mut self) {
        self.sort_by(|a, b| a.cmp(b));
    }

    pub fn binary_search(&self, wanted: &T) -> Result<usize, usize> {
        self.binary_search_by(|element| element.cmp(wanted))
    }

    pub fn is_sorted(&self) -> bool {
        self.windows(2).all(|pair| pair[0] <= pair[1])
    }
}

impl<T: PartialEq> [T] {
    pub fn starts_with(&self, prefix: &[T]) -> bool {
        prefix.len() <= self.len() && self.subslice(0, prefix.len()) == *prefix
    }

    pub fn ends_with(&self, suffix: &[T]) -> bool {
        suffix.len() <= self.len() && self.subslice(self.len() - suffix.len(), self.len()) == *suffix
    }
}

impl<T: Clone> [T] {
    pub fn fill(&mut self, value: T) {
        for slot in self.iter_mut() {
            *slot = value.clone();
        }
    }

    pub fn clone_from_slice(&mut self, source: &[T]) {
        if self.len() != source.len() {
            panic!("destination and source slices have different lengths");
        }
        let mut index = 0;
        while index < self.len() {
            self[index] = source[index].clone();
            index += 1;
        }
    }

    pub fn copy_from_slice(&mut self, source: &[T]) {
        self.clone_from_slice(source);
    }

    pub fn repeat(&self, count: usize) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len() * count);
        let mut done = 0;
        while done < count {
            result.extend_from_slice(self);
            done += 1;
        }
        result
    }
}

impl<T: Clone> [Vec<T>] {
    pub fn concat(&self) -> Vec<T> {
        let mut result = Vec::new();
        for part in self.iter() {
            result.extend_from_slice(part.as_slice());
        }
        result
    }

    pub fn join(&self, separator: &T) -> Vec<T> {
        let mut result = Vec::new();
        for (index, part) in self.iter().enumerate() {
            if index > 0 {
                result.push(separator.clone());
            }
            result.extend_from_slice(part.as_slice());
        }
        result
    }
}

impl<T: Clone> [&[T]] {
    pub fn concat(&self) -> Vec<T> {
        let mut result = Vec::new();
        for part in self.iter() {
            result.extend_from_slice(*part);
        }
        result
    }

    pub fn join(&self, separator: &T) -> Vec<T> {
        let mut result = Vec::new();
        for (index, part) in self.iter().enumerate() {
            if index > 0 {
                result.push(separator.clone());
            }
            result.extend_from_slice(*part);
        }
        result
    }
}

impl<T: Clone, const N: usize> [[T; N]] {
    pub fn concat(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len() * N);
        for part in self.iter() {
            result.extend_from_slice(part);
        }
        result
    }

    pub fn join(&self, separator: &T) -> Vec<T> {
        let mut result = Vec::new();
        for (index, part) in self.iter().enumerate() {
            if index > 0 {
                result.push(separator.clone());
            }
            result.extend_from_slice(part);
        }
        result
    }
}

impl<T: PartialOrd> PartialOrd for [T] {
    /// Element by element; a prefix comes first.
    fn partial_cmp(&self, other: &[T]) -> Option<std::cmp::Ordering> {
        let shorter = self.len().min(other.len());
        let mut index = 0;
        while index < shorter {
            match self[index].partial_cmp(&other[index]) {
                Some(std::cmp::Ordering::Equal) => {}
                decided => return decided,
            }
            index += 1;
        }
        self.len().partial_cmp(&other.len())
    }
}

impl<T: Ord> Ord for [T] {
    fn cmp(&self, other: &[T]) -> std::cmp::Ordering {
        let shorter = self.len().min(other.len());
        let mut index = 0;
        while index < shorter {
            match self[index].cmp(&other[index]) {
                std::cmp::Ordering::Equal => {}
                decided => return decided,
            }
            index += 1;
        }
        self.len().cmp(&other.len())
    }
}

impl<T: Eq> Eq for [T] {}

impl<T: PartialEq> PartialEq for [T] {
    fn eq(&self, other: &[T]) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut index = 0;
        while index < self.len() {
            if self[index] != other[index] {
                return false;
            }
            index += 1;
        }
        true
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for [T] {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut list = f.debug_list();
        for item in self.iter() {
            list.entry(item);
        }
        list.finish()
    }
}

impl<T> std::ops::Index<std::ops::Range<usize>> for [T] {
    type Output = [T];
    fn index(&self, range: std::ops::Range<usize>) -> &[T] {
        self.subslice(range.start, range.end)
    }
}

impl<T> std::ops::Index<std::ops::RangeFrom<usize>> for [T] {
    type Output = [T];
    fn index(&self, range: std::ops::RangeFrom<usize>) -> &[T] {
        self.subslice(range.start, self.len())
    }
}

impl<T> std::ops::Index<std::ops::RangeTo<usize>> for [T] {
    type Output = [T];
    fn index(&self, range: std::ops::RangeTo<usize>) -> &[T] {
        self.subslice(0, range.end)
    }
}

impl<T> std::ops::Index<std::ops::RangeFull> for [T] {
    type Output = [T];
    fn index(&self, range: std::ops::RangeFull) -> &[T] {
        self
    }
}

impl<T> std::ops::Index<std::ops::RangeInclusive<usize>> for [T] {
    type Output = [T];
    fn index(&self, range: std::ops::RangeInclusive<usize>) -> &[T] {
        self.subslice(range.start, range.end + 1)
    }
}

impl<T> std::ops::Index<std::ops::RangeToInclusive<usize>> for [T] {
    type Output = [T];
    fn index(&self, range: std::ops::RangeToInclusive<usize>) -> &[T] {
        self.subslice(0, range.end + 1)
    }
}

impl<T> [T] {
    /// The elements from `start` to `end`, to change.
    fn subslice_mut(&mut self, start: usize, end: usize) -> &mut [T] {
        if start > end || end > self.len() {
            panic!("range {}..{} is out of bounds of a slice of length {}", start, end, self.len());
        }
        std::intrinsics::slice_from_raw_parts_mut(self.as_mut_ptr().add(start), end - start)
    }
}

impl<T> std::ops::IndexMut<std::ops::Range<usize>> for [T] {
    fn index_mut(&mut self, range: std::ops::Range<usize>) -> &mut [T] {
        self.subslice_mut(range.start, range.end)
    }
}

impl<T> std::ops::IndexMut<std::ops::RangeFrom<usize>> for [T] {
    fn index_mut(&mut self, range: std::ops::RangeFrom<usize>) -> &mut [T] {
        let len = self.len();
        self.subslice_mut(range.start, len)
    }
}

impl<T> std::ops::IndexMut<std::ops::RangeTo<usize>> for [T] {
    fn index_mut(&mut self, range: std::ops::RangeTo<usize>) -> &mut [T] {
        self.subslice_mut(0, range.end)
    }
}

impl<T> std::ops::IndexMut<std::ops::RangeInclusive<usize>> for [T] {
    fn index_mut(&mut self, range: std::ops::RangeInclusive<usize>) -> &mut [T] {
        self.subslice_mut(range.start, range.end + 1)
    }
}

impl<T> std::ops::IndexMut<std::ops::RangeFull> for [T] {
    fn index_mut(&mut self, range: std::ops::RangeFull) -> &mut [T] {
        self
    }
}

/// Iterator over references to the elements of a slice.
pub struct Iter<T> {
    pointer: *const T,
    next: usize,
    len: usize,
}

impl<T> Iter<T> {
    /// The elements not yet iterated over.
    pub fn as_slice(&self) -> &[T] {
        std::intrinsics::slice_from_raw_parts(self.pointer.add(self.next), self.len - self.next)
    }
}

impl<T> Clone for Iter<T> {
    fn clone(&self) -> Iter<T> {
        Iter { pointer: self.pointer, next: self.next, len: self.len }
    }
}

impl<T> Iterator for Iter<T> {
    type Item = &T;

    fn next(&mut self) -> Option<&T> {
        if self.next >= self.len {
            return None;
        }
        let item = unsafe { &*self.pointer.add(self.next) };
        self.next += 1;
        Some(item)
    }

    fn nth(&mut self, index: usize) -> Option<&T> {
        if index >= self.len - self.next {
            self.next = self.len;
            return None;
        }
        self.next += index;
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len - self.next, Some(self.len - self.next))
    }
}

impl<T> std::iter::DoubleEndedIterator for Iter<T> {
    fn next_back(&mut self) -> Option<&T> {
        if self.next >= self.len {
            return None;
        }
        self.len -= 1;
        Some(unsafe { &*self.pointer.add(self.len) })
    }
}

impl<T> std::iter::ExactSizeIterator for Iter<T> {
    fn len(&self) -> usize {
        self.len - self.next
    }
}

/// Iterator over mutable references to the elements of a slice.
pub struct IterMut<T> {
    pointer: *mut T,
    next: usize,
    len: usize,
}

impl<T> Iterator for IterMut<T> {
    type Item = &mut T;

    fn next(&mut self) -> Option<&mut T> {
        if self.next >= self.len {
            return None;
        }
        let item = unsafe { &mut *self.pointer.add(self.next) };
        self.next += 1;
        Some(item)
    }
}

impl<T> std::iter::DoubleEndedIterator for IterMut<T> {
    fn next_back(&mut self) -> Option<&mut T> {
        if self.next >= self.len {
            return None;
        }
        self.len -= 1;
        Some(unsafe { &mut *self.pointer.add(self.len) })
    }
}

impl<T> std::iter::ExactSizeIterator for IterMut<T> {
    fn len(&self) -> usize {
        self.len - self.next
    }
}

/// Consecutive pieces of a slice, the last possibly shorter.
pub struct Chunks<T> {
    rest: &[T],
    size: usize,
}

impl<T> Iterator for Chunks<T> {
    type Item = &[T];

    fn next(&mut self) -> Option<&[T]> {
        if self.rest.is_empty() {
            return None;
        }
        let end = self.size.min(self.rest.len());
        let (chunk, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some(chunk)
    }
}

impl<T> std::iter::DoubleEndedIterator for Chunks<T> {
    fn next_back(&mut self) -> Option<&[T]> {
        if self.rest.is_empty() {
            return None;
        }
        let last = match self.rest.len() % self.size { 0 => self.size, partial => partial };
        let (rest, chunk) = self.rest.split_at(self.rest.len() - last);
        self.rest = rest;
        Some(chunk)
    }
}

impl<T> std::iter::ExactSizeIterator for Chunks<T> {
    fn len(&self) -> usize {
        (self.rest.len() + self.size - 1) / self.size
    }
}

/// Pieces of a slice counted from its end, the first possibly shorter.
pub struct RChunks<T> {
    rest: &[T],
    size: usize,
}

impl<T> Iterator for RChunks<T> {
    type Item = &[T];

    fn next(&mut self) -> Option<&[T]> {
        if self.rest.is_empty() {
            return None;
        }
        let start = self.rest.len() - self.size.min(self.rest.len());
        let (rest, chunk) = self.rest.split_at(start);
        self.rest = rest;
        Some(chunk)
    }
}

/// Consecutive pieces of a slice to change.
pub struct ChunksMut<T> {
    pointer: *mut T,
    len: usize,
    size: usize,
}

impl<T> Iterator for ChunksMut<T> {
    type Item = &mut [T];

    fn next(&mut self) -> Option<&mut [T]> {
        if self.len == 0 {
            return None;
        }
        let length = self.size.min(self.len);
        let chunk = std::intrinsics::slice_from_raw_parts_mut(self.pointer, length);
        self.pointer = self.pointer.add(length);
        self.len -= length;
        Some(chunk)
    }
}

/// Every run of a given number of consecutive elements.
pub struct Windows<T> {
    slice: &[T],
    start: usize,
    size: usize,
}

impl<T> Iterator for Windows<T> {
    type Item = &[T];

    fn next(&mut self) -> Option<&[T]> {
        if self.start + self.size > self.slice.len() {
            return None;
        }
        let window = self.slice.subslice(self.start, self.start + self.size);
        self.start += 1;
        Some(window)
    }
}

impl<T> std::iter::ExactSizeIterator for Windows<T> {
    fn len(&self) -> usize {
        (self.slice.len() + 1).saturating_sub(self.start + self.size)
    }
}

impl<T> IntoIterator for &[T] {
    type Item = &T;
    type IntoIter = Iter<T>;

    fn into_iter(self) -> Iter<T> {
        self.iter()
    }
}

impl<T> IntoIterator for &mut [T] {
    type Item = &mut T;
    type IntoIter = IterMut<T>;

    fn into_iter(self) -> IterMut<T> {
        self.iter_mut()
    }
}

/// Slices up to this long are sorted by insertion.
const INSERTION_SORT_LEN: usize = 20;

/// Sort `items` by inserting each element into the sorted ones before it.
fn insertion_sort<T, F: FnMut(&T, &T) -> std::cmp::Ordering>(items: &mut [T], compare: &mut F) {
    let pointer = items.as_mut_ptr();
    for end in 1..items.len() {
        unsafe {
            if compare(&*pointer.add(end), &*pointer.add(end - 1)) != std::cmp::Ordering::Less {
                continue;
            }
            // Lift the element out and move the larger ones up behind it.
            let item = std::ptr::read(pointer.add(end));
            let mut hole = end;
            while hole > 0 && compare(&item, &*pointer.add(hole - 1)) == std::cmp::Ordering::Less {
                std::ptr::write(pointer.add(hole), std::ptr::read(pointer.add(hole - 1)));
                hole -= 1;
            }
            std::ptr::write(pointer.add(hole), item);
        }
    }
}

/// Sort `items` stably, with room for half of them in `scratch`.
///
/// Elements are moved bit for bit between the slice and the scratch
/// space; at every moment each lives in exactly one place, so none is
/// dropped or duplicated.
fn merge_sort<T, F: FnMut(&T, &T) -> std::cmp::Ordering>(items: &mut [T], scratch: *mut T, compare: &mut F) {
    let len = items.len();
    if len <= INSERTION_SORT_LEN {
        insertion_sort(items, compare);
        return;
    }
    let middle = len / 2;
    merge_sort(&mut items[..middle], scratch, compare);
    merge_sort(&mut items[middle..], scratch, compare);

    let pointer = items.as_mut_ptr();
    unsafe {
        // The halves are already in order.
        if compare(&*pointer.add(middle), &*pointer.add(middle - 1)) != std::cmp::Ordering::Less {
            return;
        }
        // The left half moves to the scratch space; merging writes the
        // output from the front, never past the right half's next element.
        std::ptr::copy_nonoverlapping(pointer, scratch, middle);
        let (mut left, mut right, mut out) = (0, middle, 0);
        while left < middle && right < len {
            if compare(&*pointer.add(right), &*scratch.add(left)) == std::cmp::Ordering::Less {
                std::ptr::write(pointer.add(out), std::ptr::read(pointer.add(right)));
                right += 1;
            } else {
                std::ptr::write(pointer.add(out), std::ptr::read(scratch.add(left)));
                left += 1;
            }
            out += 1;
        }
        // What is left of the right half is in place already.
        std::ptr::copy_nonoverlapping(scratch.add(left), pointer.add(out), middle - left);
    }
}

pub struct Split<T, P> {
    /// What is left to split; `None` once the last part is out.
    rest: Option<&[T]>,
    separator: P,
    /// How many more parts `splitn` may make.
    parts_left: usize,
}

impl<T, P: FnMut(&T) -> bool> Iterator for Split<T, P> {
    type Item = &[T];

    fn next(&mut self) -> Option<&[T]> {
        let rest = self.rest?;
        if self.parts_left == 1 {
            self.rest = None;
            return Some(rest);
        }
        self.parts_left -= 1;
        let mut index = 0;
        while index < rest.len() {
            if (self.separator)(&rest[index]) {
                self.rest = Some(rest.subslice(index + 1, rest.len()));
                return Some(rest.subslice(0, index));
            }
            index += 1;
        }
        self.rest = None;
        Some(rest)
    }
}

pub struct RSplit<T, P> {
    rest: Option<&[T]>,
    separator: P,
}

impl<T, P: FnMut(&T) -> bool> Iterator for RSplit<T, P> {
    type Item = &[T];

    fn next(&mut self) -> Option<&[T]> {
        let rest = self.rest?;
        let mut index = rest.len();
        while index > 0 {
            if (self.separator)(&rest[index - 1]) {
                self.rest = Some(rest.subslice(0, index - 1));
                return Some(rest.subslice(index, rest.len()));
            }
            index -= 1;
        }
        self.rest = None;
        Some(rest)
    }
}

pub struct SplitInclusive<T, P> {
    rest: &[T],
    separator: P,
}

impl<T, P: FnMut(&T) -> bool> Iterator for SplitInclusive<T, P> {
    type Item = &[T];

    fn next(&mut self) -> Option<&[T]> {
        if self.rest.is_empty() {
            return None;
        }
        let rest = self.rest;
        let mut end = 0;
        while end < rest.len() {
            end += 1;
            if (self.separator)(&rest[end - 1]) {
                break;
            }
        }
        self.rest = rest.subslice(end, rest.len());
        Some(rest.subslice(0, end))
    }
}

/// Bytes as ASCII text.
impl [u8] {
    pub fn is_ascii(&self) -> bool {
        self.iter().all(|byte| byte.is_ascii())
    }

    pub fn eq_ignore_ascii_case(&self, other: &[u8]) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a.eq_ignore_ascii_case(b))
    }

    pub fn to_ascii_uppercase(&self) -> Vec<u8> {
        self.iter().map(|byte| byte.to_ascii_uppercase()).collect()
    }

    pub fn to_ascii_lowercase(&self) -> Vec<u8> {
        self.iter().map(|byte| byte.to_ascii_lowercase()).collect()
    }

    pub fn make_ascii_uppercase(&mut self) {
        for byte in self.iter_mut() {
            byte.make_ascii_uppercase();
        }
    }

    pub fn make_ascii_lowercase(&mut self) {
        for byte in self.iter_mut() {
            byte.make_ascii_lowercase();
        }
    }

    /// The bytes without ASCII whitespace at either end.
    pub fn trim_ascii(&self) -> &[u8] {
        self.trim_ascii_start().trim_ascii_end()
    }

    pub fn trim_ascii_start(&self) -> &[u8] {
        let start = self.iter().position(|byte| !byte.is_ascii_whitespace()).unwrap_or(self.len());
        &self[start..]
    }

    pub fn trim_ascii_end(&self) -> &[u8] {
        let end = self.iter().rposition(|byte| !byte.is_ascii_whitespace()).map_or(0, |last| last + 1);
        &self[..end]
    }
}
