use std::collections::BTreeMap;
use std::ops::{Index, IndexMut};

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

impl<T> SparseVec<T> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T> Index<usize> for SparseVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let Some((_, &segment_idx)) = self.table.range(..=index).last() else {
            panic!(
                "index {index} is not within any segments: {:?}",
                self.table.values().map(|&idx| {
                    let seg = self
                        .segments
                        .get(idx)
                        .expect("segments should have table idx")
                        .as_ref()
                        .expect("table should always only point to valid indices");
                    format!("[{}..{}]", seg.offset, seg.offset + seg.data.len())
                })
            );
        };

        let seg = self
            .segments
            .get(segment_idx)
            .expect("segments should have table segment_idx")
            .as_ref()
            .expect("table should always point to valid indices");

        let index = index - seg.offset;
        if index > seg.data.len() {
            panic!(
                "index {index} is not within any segments: {:?}",
                self.table.values().map(|&idx| {
                    let seg = self
                        .segments
                        .get(idx)
                        .expect("segments should have table idx")
                        .as_ref()
                        .expect("table should always point to valid indices");
                    format!("[{}..{}]", seg.offset, seg.offset + seg.data.len())
                })
            );
        }

        &seg.data[index]
    }
}

impl<T> IndexMut<usize> for SparseVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let Some(segment_idx) = self
            .table
            .range(..=index)
            .last()
            .map(|(_key, &seg_idx)| seg_idx)
        else {
            // we're first in memory

            // do we have a free slot already?
            if let Some(pop) = self.free.pop() {
                // free slot found
                let segment = Segment::new(index);
                self.segments[pop] = Some(segment);
                pop
            } else {
                // no free slots, make a new one
                let segment = Segment::new(index);
                self.segments.push(Some(segment));
                self.segments.len() - 1
            }
        };

        let seg = self
            .segments
            .get(segment_idx)
            .expect("segments should have table segment_idx")
            .as_ref()
            .expect("table should always point to valid indices");

        
    }
}
