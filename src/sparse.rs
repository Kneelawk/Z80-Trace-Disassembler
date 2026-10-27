use std::collections::BTreeMap;
use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct SparseVec<T> {
    table: BTreeMap<usize, usize>,
    segments: Vec<Segment<T>>,
}

#[derive(Clone)]
struct Segment<T> {
    data: Vec<T>,
    offset: usize,
}

impl<T> Default for SparseVec<T> {
    fn default() -> Self {
        Self {
            table: Default::default(),
            segments: vec![],
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
            panic!("index {index} is not within any segments: {:?}", self.table.values().map(|&idx| {
                let seg = self.segments.get(idx).expect("segments should have table idx");
                format!("[{}..{}]", seg.offset, seg.offset + seg.data.len())
            }));
        };

        let seg = self.segments.get(segment_idx).expect("segments should have table segment_idx");
        let index = index - seg.offset;
        if index > seg.data.len() {
            panic!("index {index} is not within any segments: {:?}", self.table.values().map(|&idx| {
                let seg = self.segments.get(idx).expect("segments should have table idx");
                format!("[{}..{}]", seg.offset, seg.offset + seg.data.len())
            }));
        }

        &seg.data[index]
    }
}

impl<T> IndexMut<usize> for SparseVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        
    }
}
