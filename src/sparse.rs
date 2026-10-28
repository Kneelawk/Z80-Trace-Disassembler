use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter, Write};
use std::ops::{Bound, Index, IndexMut};

#[derive(Clone)]
pub struct SparseVec<T> {
    table: BTreeMap<usize, usize>,
    free: Vec<usize>,
    segments: Vec<Option<Segment<T>>>,
}

#[derive(Clone)]
struct Segment<T> {
    pub data: Vec<T>,
    pub offset: usize,
}

impl<T> Default for SparseVec<T> {
    fn default() -> Self {
        Self {
            table: Default::default(),
            free: vec![],
            segments: vec![],
        }
    }
}

impl<T> Segment<T> {
    fn new(offset: usize) -> Self {
        Self {
            data: vec![],
            offset,
        }
    }
}

impl<T> Index<usize> for SparseVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if let Some(output) = self.get(index) {
            output
        } else {
            panic!(
                "index {index} is not within any segments: {:?}",
                self.segment_list_strs()
            );
        }
    }
}

impl<T> IndexMut<usize> for SparseVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if let Some(output) = self.get_mut(index) {
            output
        } else {
            panic!(
                "index {index} is not within any segments: {:?}",
                self.segment_list_strs()
            );
        }
    }
}

impl<T> SparseVec<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let Some((_offset, &segment_idx)) = self.table.range(..=index).next_back() else {
            return None;
        };

        let seg = self
            .segments
            .get(segment_idx)
            .expect("segments should have table segment_idx")
            .as_ref()
            .expect("table should always point to valid indices");

        let index = index - seg.offset;
        if index > seg.data.len() {
            return None;
        }

        Some(&seg.data[index])
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let Some((_offset, &segment_idx)) = self.table.range(..=index).next_back() else {
            return None;
        };

        let seg = self
            .segments
            .get_mut(segment_idx)
            .expect("segments should have table segment_idx")
            .as_mut()
            .expect("table should always point to valid indices");

        let index = index - seg.offset;
        if index > seg.data.len() {
            return None;
        }

        Some(&mut seg.data[index])
    }

    /// Sets a value in this sparse vec.
    ///
    /// If index was not previously part of this vec, this vec will sparsely add space for index.
    pub fn set(&mut self, index: usize, value: T) {
        let segment_idx =
            if let Some((_offset, &segment_idx)) = self.table.range(..=index).next_back() {
                let seg = self
                    .segments
                    .get_mut(segment_idx)
                    .expect("segments should have table segment_idx")
                    .as_mut()
                    .expect("table should always point to valid indices");

                let index_in_seg = index - seg.offset;
                if index_in_seg < seg.data.len() {
                    seg.data[index_in_seg] = value;
                    segment_idx
                } else {
                    if index_in_seg == seg.data.len() {
                        seg.data.push(value);
                        segment_idx
                    } else {
                        // there is a gap between us and the end of seg,
                        // so we'll need to add a new segment after all
                        self.insert_new_segment(index, value)
                    }
                }
            } else {
                // we're first in memory
                self.insert_new_segment(index, value)
            };

        // are right before someone else in memory?
        if let Some((&next_seg_offset, &next_seg_idx)) = self
            .table
            .range((Bound::Excluded(index), Bound::Unbounded))
            .next()
            && next_seg_offset == index + 1
        {
            // we are, so just absorb them and remove their segment in memory

            // first we grab the one we want to absorb and take it,
            // so we won't have to worry about multiple mutable refs
            let mut to_absorb = self
                .segments
                .get_mut(next_seg_idx)
                .expect("segments should have table next_seg_idx")
                .take()
                .expect("table should always point to valid next indices");
            // we need to make sure to remove it from the table
            self.table.remove(&next_seg_offset);

            // now we absorb the later data
            let seg = self
                .segments
                .get_mut(segment_idx)
                .expect("segments should have table segment_idx")
                .as_mut()
                .expect("table should always point to valid indices");
            seg.data.append(&mut to_absorb.data);
        }
    }

    fn insert_new_segment(&mut self, index: usize, value: T) -> usize {
        // do we have a free slot already?
        if let Some(pop) = self.free.pop() {
            // free slot found
            let mut segment = Segment::new(index);
            segment.data.push(value);
            self.segments[pop] = Some(segment);
            self.table.insert(index, pop);
            pop
        } else {
            // no free slots, make a new one
            let mut segment = Segment::new(index);
            segment.data.push(value);
            self.segments.push(Some(segment));
            let segment_idx = self.segments.len() - 1;
            self.table.insert(index, segment_idx);
            segment_idx
        }
    }

    fn segment_list_strs(&self) -> Vec<String> {
        self.table
            .values()
            .map(|&idx| {
                let seg = self
                    .segments
                    .get(idx)
                    .expect("segments should have table idx")
                    .as_ref()
                    .expect("table should always only point to valid indices");
                format!("[{}..{}]", seg.offset, seg.offset + seg.data.len())
            })
            .collect()
    }
}

impl<T: Debug> Debug for SparseVec<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = f.debug_map();
        for (&offset, &seg_idx) in &self.table {
            let seg = self
                .segments
                .get(seg_idx)
                .expect("segments should have table idx")
                .as_ref()
                .expect("table should always point to valid indices");
            for (index, data) in seg.data.iter().enumerate() {
                debug_map.entry(&(index + offset), data);
            }
        }
        debug_map.finish()
    }
}
