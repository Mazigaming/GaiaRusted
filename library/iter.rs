//! Iteration.

/// A sequence produced one element at a time.
///
/// Only `next` has to be written; everything else is built on it. The
/// adapters (`map`, `filter`, ...) are lazy: they return a new iterator and
/// do no work until that one is advanced.
pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;

    // -- adapters --------------------------------------------------------------

    fn map<B, F: FnMut(Self::Item) -> B>(self, f: F) -> Map<Self, F> {
        Map { iter: self, f }
    }

    fn filter<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> Filter<Self, P> {
        Filter { iter: self, predicate }
    }

    fn filter_map<B, F: FnMut(Self::Item) -> Option<B>>(self, f: F) -> FilterMap<Self, F> {
        FilterMap { iter: self, f }
    }

    fn enumerate(self) -> Enumerate<Self> {
        Enumerate { iter: self, count: 0 }
    }

    fn zip<U: IntoIterator>(self, other: U) -> Zip<Self, U::IntoIter> {
        Zip { a: self, b: other.into_iter() }
    }

    fn chain<U: IntoIterator<Item = Self::Item>>(self, other: U) -> Chain<Self, U::IntoIter> {
        Chain { a: Some(self), b: Some(other.into_iter()) }
    }

    fn take(self, count: usize) -> Take<Self> {
        Take { iter: self, remaining: count }
    }

    fn skip(self, count: usize) -> Skip<Self> {
        Skip { iter: self, remaining: count }
    }

    fn take_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> TakeWhile<Self, P> {
        TakeWhile { iter: self, predicate, done: false }
    }

    fn skip_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> SkipWhile<Self, P> {
        SkipWhile { iter: self, predicate, skipping: true }
    }

    fn map_while<B, P: FnMut(Self::Item) -> Option<B>>(self, f: P) -> MapWhile<Self, P> {
        MapWhile { iter: self, f, done: false }
    }

    /// Every `step`-th element, starting with the first.
    fn step_by(self, step: usize) -> StepBy<Self> {
        if step == 0 {
            panic!("assertion failed: step != 0");
        }
        StepBy { iter: self, step, first: true }
    }

    fn flat_map<U: IntoIterator, F: FnMut(Self::Item) -> U>(self, f: F) -> FlatMap<Self, F, U::IntoIter> {
        FlatMap { iter: self, f, front: None, back: None }
    }

    fn flatten(self) -> Flatten<Self, Self::Item::IntoIter> {
        Flatten { iter: self, front: None, back: None }
    }

    fn peekable(self) -> Peekable<Self> {
        Peekable { iter: self, peeked: None }
    }

    fn scan<St, B, F: FnMut(&mut St, Self::Item) -> Option<B>>(self, initial: St, f: F) -> Scan<Self, St, F> {
        Scan { iter: self, state: initial, f, done: false }
    }

    fn inspect<F: FnMut(&Self::Item)>(self, f: F) -> Inspect<Self, F> {
        Inspect { iter: self, f }
    }

    fn fuse(self) -> Fuse<Self> {
        Fuse { iter: Some(self) }
    }

    /// The sequence over and over; it has to be cloned to start again.
    fn cycle(self) -> Cycle<Self> {
        Cycle { original: self.clone(), current: self }
    }

    fn cloned(self) -> Cloned<Self> {
        Cloned { iter: self }
    }

    fn copied(self) -> Copied<Self> {
        Copied { iter: self }
    }

    fn rev(self) -> Rev<Self> {
        Rev { iter: self }
    }

    fn by_ref(&mut self) -> &mut Self {
        self
    }

    // -- consumers ---------------------------------------------------------------

    fn fold<B, F: FnMut(B, Self::Item) -> B>(self, initial: B, f: F) -> B {
        let mut iter = self;
        let mut f = f;
        let mut accumulator = initial;
        while let Some(item) = iter.next() {
            accumulator = f(accumulator, item);
        }
        accumulator
    }

    fn reduce<F: FnMut(Self::Item, Self::Item) -> Self::Item>(self, f: F) -> Option<Self::Item> {
        let mut iter = self;
        let first = iter.next()?;
        Some(iter.fold(first, f))
    }

    fn for_each<F: FnMut(Self::Item)>(self, f: F) {
        let mut iter = self;
        let mut f = f;
        while let Some(item) = iter.next() {
            f(item);
        }
    }

    fn count(self) -> usize {
        let mut iter = self;
        let mut count = 0;
        while let Some(_) = iter.next() {
            count += 1;
        }
        count
    }

    fn last(self) -> Option<Self::Item> {
        let mut iter = self;
        let mut last = None;
        while let Some(item) = iter.next() {
            last = Some(item);
        }
        last
    }

    fn nth(&mut self, index: usize) -> Option<Self::Item> {
        let mut skipped = 0;
        while skipped < index {
            if self.next().is_none() {
                return None;
            }
            skipped += 1;
        }
        self.next()
    }

    fn any<P: FnMut(Self::Item) -> bool>(&mut self, predicate: P) -> bool {
        let mut predicate = predicate;
        while let Some(item) = self.next() {
            if predicate(item) {
                return true;
            }
        }
        false
    }

    fn all<P: FnMut(Self::Item) -> bool>(&mut self, predicate: P) -> bool {
        let mut predicate = predicate;
        while let Some(item) = self.next() {
            if !predicate(item) {
                return false;
            }
        }
        true
    }

    fn find<P: FnMut(&Self::Item) -> bool>(&mut self, predicate: P) -> Option<Self::Item> {
        let mut predicate = predicate;
        while let Some(item) = self.next() {
            if predicate(&item) {
                return Some(item);
            }
        }
        None
    }

    fn find_map<B, F: FnMut(Self::Item) -> Option<B>>(&mut self, f: F) -> Option<B> {
        let mut f = f;
        while let Some(item) = self.next() {
            if let Some(found) = f(item) {
                return Some(found);
            }
        }
        None
    }

    fn position<P: FnMut(Self::Item) -> bool>(&mut self, predicate: P) -> Option<usize> {
        let mut predicate = predicate;
        let mut index = 0;
        while let Some(item) = self.next() {
            if predicate(item) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    /// The position counted from the front of the last element that matches.
    fn rposition<P: FnMut(Self::Item) -> bool>(&mut self, predicate: P) -> Option<usize> {
        let mut predicate = predicate;
        let mut index = self.len();
        while let Some(item) = self.next_back() {
            index -= 1;
            if predicate(item) {
                return Some(index);
            }
        }
        None
    }

    /// The last of the largest elements.
    fn max(self) -> Option<Self::Item> {
        self.max_by(|a, b| a.cmp(b))
    }

    /// The first of the smallest elements.
    fn min(self) -> Option<Self::Item> {
        self.min_by(|a, b| a.cmp(b))
    }

    fn max_by<F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering>(self, compare: F) -> Option<Self::Item> {
        let mut iter = self;
        let mut compare = compare;
        let mut best = iter.next()?;
        while let Some(item) = iter.next() {
            if compare(&item, &best) != std::cmp::Ordering::Less {
                best = item;
            }
        }
        Some(best)
    }

    fn min_by<F: FnMut(&Self::Item, &Self::Item) -> std::cmp::Ordering>(self, compare: F) -> Option<Self::Item> {
        let mut iter = self;
        let mut compare = compare;
        let mut best = iter.next()?;
        while let Some(item) = iter.next() {
            if compare(&item, &best) == std::cmp::Ordering::Less {
                best = item;
            }
        }
        Some(best)
    }

    fn max_by_key<B: Ord, F: FnMut(&Self::Item) -> B>(self, key: F) -> Option<Self::Item> {
        let mut iter = self;
        let mut key = key;
        let mut best = iter.next()?;
        let mut best_key = key(&best);
        while let Some(item) = iter.next() {
            let item_key = key(&item);
            if item_key >= best_key {
                best = item;
                best_key = item_key;
            }
        }
        Some(best)
    }

    fn min_by_key<B: Ord, F: FnMut(&Self::Item) -> B>(self, key: F) -> Option<Self::Item> {
        let mut iter = self;
        let mut key = key;
        let mut best = iter.next()?;
        let mut best_key = key(&best);
        while let Some(item) = iter.next() {
            let item_key = key(&item);
            if item_key < best_key {
                best = item;
                best_key = item_key;
            }
        }
        Some(best)
    }

    fn sum<S: Sum<Self::Item>>(self) -> S {
        Sum::<Self::Item>::sum(self)
    }

    fn product<P: Product<Self::Item>>(self) -> P {
        Product::<Self::Item>::product(self)
    }

    fn collect<C: FromIterator<Self::Item>>(self) -> C {
        FromIterator::<Self::Item>::from_iter(self)
    }

    /// Split into the elements that satisfy `predicate` and those that do not.
    fn partition<C: Default + Extend<Self::Item>, F: FnMut(&Self::Item) -> bool>(self, predicate: F) -> (C, C) {
        let mut iter = self;
        let mut predicate = predicate;
        let mut matching: C = Default::default();
        let mut rest: C = Default::default();
        while let Some(item) = iter.next() {
            if predicate(&item) {
                matching.extend_one(item);
            } else {
                rest.extend_one(item);
            }
        }
        (matching, rest)
    }

    /// Turn an iterator of pairs into a pair of collections.
    fn unzip<A, B, FromA: Default + Extend<A>, FromB: Default + Extend<B>>(self) -> (FromA, FromB)
    where
        Self: Iterator<Item = (A, B)>,
    {
        let mut iter = self;
        let mut left: FromA = Default::default();
        let mut right: FromB = Default::default();
        while let Some((a, b)) = iter.next() {
            left.extend_one(a);
            right.extend_one(b);
        }
        (left, right)
    }

    fn eq<I: IntoIterator>(self, other: I) -> bool {
        let mut iter = self;
        let mut other = other.into_iter();
        loop {
            match (iter.next(), other.next()) {
                (None, None) => return true,
                (Some(a), Some(b)) => {
                    if a != b {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }

    fn ne<I: IntoIterator>(self, other: I) -> bool {
        !self.eq(other)
    }

    /// Lexicographic comparison.
    fn cmp<I: IntoIterator>(self, other: I) -> std::cmp::Ordering {
        let mut iter = self;
        let mut other = other.into_iter();
        loop {
            match (iter.next(), other.next()) {
                (None, None) => return std::cmp::Ordering::Equal,
                (None, Some(_)) => return std::cmp::Ordering::Less,
                (Some(_), None) => return std::cmp::Ordering::Greater,
                (Some(a), Some(b)) => {
                    let order = a.cmp(&b);
                    if order != std::cmp::Ordering::Equal {
                        return order;
                    }
                }
            }
        }
    }

    fn lt<I: IntoIterator>(self, other: I) -> bool { self.cmp(other) == std::cmp::Ordering::Less }
    fn le<I: IntoIterator>(self, other: I) -> bool { self.cmp(other) != std::cmp::Ordering::Greater }
    fn gt<I: IntoIterator>(self, other: I) -> bool { self.cmp(other) == std::cmp::Ordering::Greater }
    fn ge<I: IntoIterator>(self, other: I) -> bool { self.cmp(other) != std::cmp::Ordering::Less }

    fn is_sorted(self) -> bool {
        let mut iter = self;
        let Some(mut previous) = iter.next() else { return true };
        while let Some(item) = iter.next() {
            if item < previous {
                return false;
            }
            previous = item;
        }
        true
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// An iterator that can also be taken from the back.
pub trait DoubleEndedIterator: Iterator {
    fn next_back(&mut self) -> Option<Self::Item>;

    fn nth_back(&mut self, index: usize) -> Option<Self::Item> {
        let mut skipped = 0;
        while skipped < index {
            if self.next_back().is_none() {
                return None;
            }
            skipped += 1;
        }
        self.next_back()
    }

    fn rfold<B, F: FnMut(B, Self::Item) -> B>(self, initial: B, f: F) -> B {
        let mut iter = self;
        let mut f = f;
        let mut accumulator = initial;
        while let Some(item) = iter.next_back() {
            accumulator = f(accumulator, item);
        }
        accumulator
    }

    fn rfind<P: FnMut(&Self::Item) -> bool>(&mut self, predicate: P) -> Option<Self::Item> {
        let mut predicate = predicate;
        while let Some(item) = self.next_back() {
            if predicate(&item) {
                return Some(item);
            }
        }
        None
    }
}

/// An iterator that knows how many elements it has left.
pub trait ExactSizeIterator: Iterator {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Something that can be turned into an iterator: what `for` loops accept.
pub trait IntoIterator {
    type Item;
    type IntoIter;

    fn into_iter(self) -> Self::IntoIter;
}

impl<I: Iterator> IntoIterator for I {
    type Item = I::Item;
    type IntoIter = I;

    fn into_iter(self) -> I {
        self
    }
}

/// Building a collection from an iterator: what `collect` produces.
pub trait FromIterator<A> {
    fn from_iter<I>(items: I) -> Self;
}

/// Adding the elements of an iterator to a collection.
pub trait Extend<A> {
    fn extend<I: IntoIterator<Item = A>>(&mut self, items: I);

    fn extend_one(&mut self, item: A);
}

pub trait Sum<A> {
    fn sum<I>(items: I) -> Self;
}

pub trait Product<A> {
    fn product<I>(items: I) -> Self;
}

// -- adapters ------------------------------------------------------------------

pub struct Map<I, F> {
    iter: I,
    f: F,
}

impl<I: Iterator, F> Iterator for Map<I, F> {
    type Item = F::Output;

    fn next(&mut self) -> Option<F::Output> {
        match self.iter.next() {
            Some(item) => Some((self.f)(item)),
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I: DoubleEndedIterator, F> DoubleEndedIterator for Map<I, F> {
    fn next_back(&mut self) -> Option<F::Output> {
        match self.iter.next_back() {
            Some(item) => Some((self.f)(item)),
            None => None,
        }
    }
}

impl<I: ExactSizeIterator, F> ExactSizeIterator for Map<I, F> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct Filter<I, P> {
    iter: I,
    predicate: P,
}

impl<I: Iterator, P> Iterator for Filter<I, P> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        while let Some(item) = self.iter.next() {
            if (self.predicate)(&item) {
                return Some(item);
            }
        }
        None
    }
}

impl<I: DoubleEndedIterator, P> DoubleEndedIterator for Filter<I, P> {
    fn next_back(&mut self) -> Option<I::Item> {
        while let Some(item) = self.iter.next_back() {
            if (self.predicate)(&item) {
                return Some(item);
            }
        }
        None
    }
}

pub struct FilterMap<I, F> {
    iter: I,
    f: F,
}

impl<I: Iterator, B, F: FnMut(I::Item) -> Option<B>> Iterator for FilterMap<I, F> {
    type Item = B;

    fn next(&mut self) -> Option<B> {
        while let Some(item) = self.iter.next() {
            if let Some(mapped) = (self.f)(item) {
                return Some(mapped);
            }
        }
        None
    }
}

impl<I: DoubleEndedIterator, B, F: FnMut(I::Item) -> Option<B>> DoubleEndedIterator for FilterMap<I, F> {
    fn next_back(&mut self) -> Option<B> {
        while let Some(item) = self.iter.next_back() {
            if let Some(mapped) = (self.f)(item) {
                return Some(mapped);
            }
        }
        None
    }
}

pub struct Enumerate<I> {
    iter: I,
    count: usize,
}

impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);

    fn next(&mut self) -> Option<(usize, I::Item)> {
        let item = self.iter.next()?;
        let index = self.count;
        self.count += 1;
        Some((index, item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Enumerate<I> {
    fn next_back(&mut self) -> Option<(usize, I::Item)> {
        let item = self.iter.next_back()?;
        Some((self.count + self.iter.len(), item))
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Enumerate<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct Zip<A, B> {
    a: A,
    b: B,
}

impl<A: Iterator, B: Iterator> Iterator for Zip<A, B> {
    type Item = (A::Item, B::Item);

    fn next(&mut self) -> Option<(A::Item, B::Item)> {
        let first = self.a.next()?;
        let second = self.b.next()?;
        Some((first, second))
    }
}

impl<A: ExactSizeIterator, B: ExactSizeIterator> ExactSizeIterator for Zip<A, B> {
    fn len(&self) -> usize {
        self.a.len().min(self.b.len())
    }
}

impl<A: DoubleEndedIterator + ExactSizeIterator, B: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Zip<A, B> {
    fn next_back(&mut self) -> Option<(A::Item, B::Item)> {
        // Line the two up first: the longer one's extra elements come off.
        while self.a.len() > self.b.len() {
            self.a.next_back();
        }
        while self.b.len() > self.a.len() {
            self.b.next_back();
        }
        let first = self.a.next_back()?;
        let second = self.b.next_back()?;
        Some((first, second))
    }
}

pub struct Chain<A, B> {
    a: Option<A>,
    b: Option<B>,
}

impl<A: Iterator, B: Iterator<Item = A::Item>> Iterator for Chain<A, B> {
    type Item = A::Item;

    fn next(&mut self) -> Option<A::Item> {
        if let Some(a) = &mut self.a {
            match a.next() {
                Some(item) => return Some(item),
                None => self.a = None,
            }
        }
        match &mut self.b {
            Some(b) => b.next(),
            None => None,
        }
    }
}

impl<A: DoubleEndedIterator, B: DoubleEndedIterator<Item = A::Item>> DoubleEndedIterator for Chain<A, B> {
    fn next_back(&mut self) -> Option<A::Item> {
        if let Some(b) = &mut self.b {
            match b.next_back() {
                Some(item) => return Some(item),
                None => self.b = None,
            }
        }
        match &mut self.a {
            Some(a) => a.next_back(),
            None => None,
        }
    }
}

pub struct Take<I> {
    iter: I,
    remaining: usize,
}

impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.iter.next()
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Take<I> {
    fn len(&self) -> usize {
        self.iter.len().min(self.remaining)
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Take<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        if self.remaining == 0 {
            return None;
        }
        while self.iter.len() > self.remaining {
            self.iter.next_back();
        }
        self.remaining -= 1;
        self.iter.next_back()
    }
}

pub struct Skip<I> {
    iter: I,
    remaining: usize,
}

impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        while self.remaining > 0 {
            self.remaining -= 1;
            self.iter.next()?;
        }
        self.iter.next()
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Skip<I> {
    fn len(&self) -> usize {
        self.iter.len().saturating_sub(self.remaining)
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Skip<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        if self.iter.len() > self.remaining { self.iter.next_back() } else { None }
    }
}

pub struct TakeWhile<I, P> {
    iter: I,
    predicate: P,
    done: bool,
}

impl<I: Iterator, P> Iterator for TakeWhile<I, P> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.done {
            return None;
        }
        let item = self.iter.next()?;
        if (self.predicate)(&item) {
            Some(item)
        } else {
            self.done = true;
            None
        }
    }
}

pub struct SkipWhile<I, P> {
    iter: I,
    predicate: P,
    skipping: bool,
}

impl<I: Iterator, P> Iterator for SkipWhile<I, P> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        while self.skipping {
            let item = self.iter.next()?;
            if !(self.predicate)(&item) {
                self.skipping = false;
                return Some(item);
            }
        }
        self.iter.next()
    }
}

pub struct MapWhile<I, F> {
    iter: I,
    f: F,
    done: bool,
}

impl<I: Iterator, B, F: FnMut(I::Item) -> Option<B>> Iterator for MapWhile<I, F> {
    type Item = B;

    fn next(&mut self) -> Option<B> {
        if self.done {
            return None;
        }
        let item = self.iter.next()?;
        match (self.f)(item) {
            Some(mapped) => Some(mapped),
            None => {
                self.done = true;
                None
            }
        }
    }
}

pub struct StepBy<I> {
    iter: I,
    step: usize,
    first: bool,
}

impl<I: Iterator> Iterator for StepBy<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.first {
            self.first = false;
            self.iter.next()
        } else {
            self.iter.nth(self.step - 1)
        }
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for StepBy<I> {
    fn len(&self) -> usize {
        let left = self.iter.len();
        if self.first {
            if left == 0 { 0 } else { 1 + (left - 1) / self.step }
        } else {
            left / self.step
        }
    }
}

impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for StepBy<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        // The last element taken is the one a whole number of steps in.
        let left = self.iter.len();
        if left == 0 {
            return None;
        }
        let extra = if self.first { (left - 1) % self.step } else { (left - self.step) % self.step };
        if !self.first && left < self.step {
            return None;
        }
        self.iter.nth_back(extra)
    }
}

pub struct FlatMap<I, F, J> {
    iter: I,
    f: F,
    front: Option<J>,
    back: Option<J>,
}

impl<I: Iterator, U: IntoIterator<IntoIter = J>, F: FnMut(I::Item) -> U, J: Iterator> Iterator for FlatMap<I, F, J> {
    type Item = J::Item;

    fn next(&mut self) -> Option<J::Item> {
        loop {
            if let Some(inner) = &mut self.front {
                if let Some(item) = inner.next() {
                    return Some(item);
                }
                self.front = None;
            }
            match self.iter.next() {
                Some(outer) => self.front = Some((self.f)(outer).into_iter()),
                None => {
                    return match &mut self.back {
                        Some(inner) => inner.next(),
                        None => None,
                    };
                }
            }
        }
    }
}

impl<I: DoubleEndedIterator, U: IntoIterator<IntoIter = J>, F: FnMut(I::Item) -> U, J: DoubleEndedIterator> DoubleEndedIterator for FlatMap<I, F, J> {
    fn next_back(&mut self) -> Option<J::Item> {
        loop {
            if let Some(inner) = &mut self.back {
                if let Some(item) = inner.next_back() {
                    return Some(item);
                }
                self.back = None;
            }
            match self.iter.next_back() {
                Some(outer) => self.back = Some((self.f)(outer).into_iter()),
                None => {
                    return match &mut self.front {
                        Some(inner) => inner.next_back(),
                        None => None,
                    };
                }
            }
        }
    }
}

pub struct Flatten<I, J> {
    iter: I,
    front: Option<J>,
    back: Option<J>,
}

impl<I: Iterator, J: Iterator> Iterator for Flatten<I, J> {
    type Item = J::Item;

    fn next(&mut self) -> Option<J::Item> {
        loop {
            if let Some(inner) = &mut self.front {
                if let Some(item) = inner.next() {
                    return Some(item);
                }
                self.front = None;
            }
            match self.iter.next() {
                Some(outer) => self.front = Some(outer.into_iter()),
                None => {
                    return match &mut self.back {
                        Some(inner) => inner.next(),
                        None => None,
                    };
                }
            }
        }
    }
}

impl<I: DoubleEndedIterator, J: DoubleEndedIterator> DoubleEndedIterator for Flatten<I, J> {
    fn next_back(&mut self) -> Option<J::Item> {
        loop {
            if let Some(inner) = &mut self.back {
                if let Some(item) = inner.next_back() {
                    return Some(item);
                }
                self.back = None;
            }
            match self.iter.next_back() {
                Some(outer) => self.back = Some(outer.into_iter()),
                None => {
                    return match &mut self.front {
                        Some(inner) => inner.next_back(),
                        None => None,
                    };
                }
            }
        }
    }
}

/// An iterator that can look at its next element without taking it.
pub struct Peekable<I: Iterator> {
    iter: I,
    /// `Some(next)` once peeked; `next` is `None` if the iterator is done.
    peeked: Option<Option<I::Item>>,
}

impl<I: Iterator> Peekable<I> {
    pub fn peek(&mut self) -> Option<&I::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match &self.peeked {
            Some(Some(item)) => Some(item),
            _ => None,
        }
    }

    pub fn peek_mut(&mut self) -> Option<&mut I::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match &mut self.peeked {
            Some(Some(item)) => Some(item),
            _ => None,
        }
    }

    /// The next element, if it satisfies `predicate`.
    pub fn next_if<F: FnOnce(&I::Item) -> bool>(&mut self, predicate: F) -> Option<I::Item> {
        let next = match self.peeked.take() {
            Some(peeked) => peeked,
            None => self.iter.next(),
        };
        match next {
            Some(item) if predicate(&item) => Some(item),
            other => {
                self.peeked = Some(other);
                None
            }
        }
    }

    pub fn next_if_eq<T>(&mut self, expected: &T) -> Option<I::Item> {
        self.next_if(|item| *item == *expected)
    }
}

impl<I: Iterator> Iterator for Peekable<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        match self.peeked.take() {
            Some(peeked) => peeked,
            None => self.iter.next(),
        }
    }
}

pub struct Scan<I, St, F> {
    iter: I,
    state: St,
    f: F,
    done: bool,
}

impl<I: Iterator, St, B, F: FnMut(&mut St, I::Item) -> Option<B>> Iterator for Scan<I, St, F> {
    type Item = B;

    fn next(&mut self) -> Option<B> {
        if self.done {
            return None;
        }
        let item = self.iter.next()?;
        let result = (self.f)(&mut self.state, item);
        if result.is_none() {
            self.done = true;
        }
        result
    }
}

pub struct Inspect<I, F> {
    iter: I,
    f: F,
}

impl<I: Iterator, F: FnMut(&I::Item)> Iterator for Inspect<I, F> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let item = self.iter.next()?;
        (self.f)(&item);
        Some(item)
    }
}

pub struct Fuse<I> {
    iter: Option<I>,
}

impl<I: Iterator> Iterator for Fuse<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let item = match &mut self.iter {
            Some(iter) => iter.next(),
            None => return None,
        };
        if item.is_none() {
            self.iter = None;
        }
        item
    }
}

pub struct Cycle<I> {
    original: I,
    current: I,
}

impl<I: Iterator + Clone> Iterator for Cycle<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        match self.current.next() {
            Some(item) => Some(item),
            None => {
                self.current = self.original.clone();
                self.current.next()
            }
        }
    }
}

/// Turns an iterator of `&T` into one of `T`.
pub struct Cloned<I> {
    iter: I,
}

impl<T: Clone, I: Iterator<Item = &T>> Iterator for Cloned<I> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self.iter.next() {
            Some(item) => Some(Clone::clone(item)),
            None => None,
        }
    }
}

impl<T: Clone, I: DoubleEndedIterator<Item = &T>> DoubleEndedIterator for Cloned<I> {
    fn next_back(&mut self) -> Option<T> {
        match self.iter.next_back() {
            Some(item) => Some(Clone::clone(item)),
            None => None,
        }
    }
}

impl<T: Clone, I: ExactSizeIterator<Item = &T>> ExactSizeIterator for Cloned<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// Turns an iterator of `&T` into one of `T`, for `T: Copy`.
pub struct Copied<I> {
    iter: I,
}

impl<T: Copy, I: Iterator<Item = &T>> Iterator for Copied<I> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self.iter.next() {
            Some(item) => Some(*item),
            None => None,
        }
    }
}

impl<T: Copy, I: DoubleEndedIterator<Item = &T>> DoubleEndedIterator for Copied<I> {
    fn next_back(&mut self) -> Option<T> {
        match self.iter.next_back() {
            Some(item) => Some(*item),
            None => None,
        }
    }
}

impl<T: Copy, I: ExactSizeIterator<Item = &T>> ExactSizeIterator for Copied<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// The elements of a double-ended iterator, back to front.
pub struct Rev<I> {
    iter: I,
}

impl<I: DoubleEndedIterator> Iterator for Rev<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next_back()
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Rev<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for Rev<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<I: Iterator> Iterator for &mut I {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
}

// -- sources ---------------------------------------------------------------------

/// An iterator that calls `f` for each element, until it returns `None`.
pub fn from_fn<T, F: FnMut() -> Option<T>>(f: F) -> FromFn<F> {
    FromFn { f }
}

pub struct FromFn<F> {
    f: F,
}

impl<T, F: FnMut() -> Option<T>> Iterator for FromFn<F> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        (self.f)()
    }
}

/// `first`, then each element computed from the one before, until `None`.
pub fn successors<T, F: FnMut(&T) -> Option<T>>(first: Option<T>, next: F) -> Successors<T, F> {
    Successors { next_item: first, f: next }
}

pub struct Successors<T, F> {
    next_item: Option<T>,
    f: F,
}

impl<T, F: FnMut(&T) -> Option<T>> Iterator for Successors<T, F> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let item = self.next_item.take()?;
        self.next_item = (self.f)(&item);
        Some(item)
    }
}

/// The same value forever.
pub fn repeat<T: Clone>(value: T) -> Repeat<T> {
    Repeat { value }
}

pub struct Repeat<T> {
    value: T,
}

impl<T: Clone> Iterator for Repeat<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        Some(self.value.clone())
    }
}

impl<T: Clone> DoubleEndedIterator for Repeat<T> {
    fn next_back(&mut self) -> Option<T> {
        Some(self.value.clone())
    }
}

/// `f()` forever.
pub fn repeat_with<T, F: FnMut() -> T>(f: F) -> RepeatWith<F> {
    RepeatWith { f }
}

pub struct RepeatWith<F> {
    f: F,
}

impl<T, F: FnMut() -> T> Iterator for RepeatWith<F> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        Some((self.f)())
    }
}

/// One element.
pub fn once<T>(value: T) -> Once<T> {
    Once { value: Some(value) }
}

pub struct Once<T> {
    value: Option<T>,
}

impl<T> Iterator for Once<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T> DoubleEndedIterator for Once<T> {
    fn next_back(&mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T> ExactSizeIterator for Once<T> {
    fn len(&self) -> usize {
        if self.value.is_some() { 1 } else { 0 }
    }
}

/// No elements.
pub fn empty<T>() -> Empty<T> {
    Empty { none: None }
}

pub struct Empty<T> {
    none: Option<T>,
}

impl<T> Iterator for Empty<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        None
    }
}

impl<T> DoubleEndedIterator for Empty<T> {
    fn next_back(&mut self) -> Option<T> {
        None
    }
}

/// Pairs of elements from two iterators.
pub fn zip<A: IntoIterator, B: IntoIterator>(a: A, b: B) -> Zip<A::IntoIter, B::IntoIter> {
    Zip { a: a.into_iter(), b: b.into_iter() }
}

// -- ranges ----------------------------------------------------------------------

macro_rules! range_iterators {
    ($($t:ident)*) => {
        $(
            impl Iterator for std::ops::Range<$t> {
                type Item = $t;

                fn next(&mut self) -> Option<$t> {
                    if self.start < self.end {
                        let value = self.start;
                        self.start += 1;
                        Some(value)
                    } else {
                        None
                    }
                }

                fn nth(&mut self, index: usize) -> Option<$t> {
                    let length = self.len();
                    if index >= length {
                        self.start = self.end;
                        return None;
                    }
                    let value = (self.start as i64 + index as i64) as $t;
                    self.start = value + 1;
                    Some(value)
                }

                fn size_hint(&self) -> (usize, Option<usize>) {
                    let length = self.len();
                    (length, Some(length))
                }
            }

            impl DoubleEndedIterator for std::ops::Range<$t> {
                fn next_back(&mut self) -> Option<$t> {
                    if self.start < self.end {
                        self.end -= 1;
                        Some(self.end)
                    } else {
                        None
                    }
                }
            }

            impl ExactSizeIterator for std::ops::Range<$t> {
                fn len(&self) -> usize {
                    if self.start < self.end { (self.end as i64 - self.start as i64) as usize } else { 0 }
                }
            }

            impl Iterator for std::ops::RangeInclusive<$t> {
                type Item = $t;

                fn next(&mut self) -> Option<$t> {
                    if self.start <= self.end {
                        let value = self.start;
                        // Stepping past `end` would overflow at the type's
                        // maximum; emptying the range by setting it to 1..=0
                        // cannot.
                        if self.start == self.end {
                            self.start = 1;
                            self.end = 0;
                        } else {
                            self.start += 1;
                        }
                        Some(value)
                    } else {
                        None
                    }
                }
            }

            impl DoubleEndedIterator for std::ops::RangeInclusive<$t> {
                fn next_back(&mut self) -> Option<$t> {
                    if self.start <= self.end {
                        let value = self.end;
                        if self.start == self.end {
                            self.start = 1;
                            self.end = 0;
                        } else {
                            self.end -= 1;
                        }
                        Some(value)
                    } else {
                        None
                    }
                }
            }

            impl ExactSizeIterator for std::ops::RangeInclusive<$t> {
                fn len(&self) -> usize {
                    if self.start <= self.end { (self.end as i64 - self.start as i64) as usize + 1 } else { 0 }
                }
            }

            impl Iterator for std::ops::RangeFrom<$t> {
                type Item = $t;

                fn next(&mut self) -> Option<$t> {
                    let value = self.start;
                    self.start += 1;
                    Some(value)
                }
            }
        )*
    };
}

range_iterators!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

/// The `char`s from `start` to `end`, stepping over the surrogate codes,
/// which are not characters.
fn char_step(c: char, forward: bool) -> char {
    let code = c as u32;
    let next = if forward {
        if code == 0xD7FF { 0xE000 } else { code + 1 }
    } else if code == 0xE000 {
        0xD7FF
    } else {
        code - 1
    };
    unsafe { std::char::from_u32_unchecked(next) }
}

impl Iterator for std::ops::Range<char> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.start < self.end {
            let value = self.start;
            self.start = char_step(value, true);
            Some(value)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for std::ops::Range<char> {
    fn next_back(&mut self) -> Option<char> {
        if self.start < self.end {
            self.end = char_step(self.end, false);
            Some(self.end)
        } else {
            None
        }
    }
}

impl Iterator for std::ops::RangeInclusive<char> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.start <= self.end {
            let value = self.start;
            if self.start == self.end {
                self.start = '\u{1}';
                self.end = '\0';
            } else {
                self.start = char_step(value, true);
            }
            Some(value)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for std::ops::RangeInclusive<char> {
    fn next_back(&mut self) -> Option<char> {
        if self.start <= self.end {
            let value = self.end;
            if self.start == self.end {
                self.start = '\u{1}';
                self.end = '\0';
            } else {
                self.end = char_step(value, false);
            }
            Some(value)
        } else {
            None
        }
    }
}

// -- sums and products ---------------------------------------------------------------

macro_rules! sum_product {
    ($($t:ident = $zero:expr, $one:expr;)*) => {
        $(
            impl Sum<$t> for $t {
                fn sum<I>(items: I) -> $t {
                    let mut items = items;
                    let mut total: $t = $zero;
                    while let Some(item) = items.next() {
                        total = total + item;
                    }
                    total
                }
            }

            impl Sum<&$t> for $t {
                fn sum<I>(items: I) -> $t {
                    let mut items = items;
                    let mut total: $t = $zero;
                    while let Some(item) = items.next() {
                        total = total + *item;
                    }
                    total
                }
            }

            impl Product<$t> for $t {
                fn product<I>(items: I) -> $t {
                    let mut items = items;
                    let mut total: $t = $one;
                    while let Some(item) = items.next() {
                        total = total * item;
                    }
                    total
                }
            }

            impl Product<&$t> for $t {
                fn product<I>(items: I) -> $t {
                    let mut items = items;
                    let mut total: $t = $one;
                    while let Some(item) = items.next() {
                        total = total * *item;
                    }
                    total
                }
            }
        )*
    };
}

sum_product! {
    i8 = 0, 1; i16 = 0, 1; i32 = 0, 1; i64 = 0, 1; i128 = 0, 1; isize = 0, 1;
    u8 = 0, 1; u16 = 0, 1; u32 = 0, 1; u64 = 0, 1; u128 = 0, 1; usize = 0, 1;
    f32 = 0.0, 1.0; f64 = 0.0, 1.0;
}
