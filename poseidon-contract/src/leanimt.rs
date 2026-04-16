//#![no_std]
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

pub struct LeanIMT<const N: usize> {
    nodes: Vec<Vec<[u8; N]>>,
}

impl<const N: usize> Default for LeanIMT<N> {
    fn default() -> Self {
        Self {
            nodes: vec![Vec::new()],
        }
    }
}

impl<const N: usize> LeanIMT<N> {
    pub fn new(leaves: &[[u8; N]], hash: impl Fn(&[u8]) -> [u8; N]) -> Self {
        let mut tree = Self::default();
        for leaf in leaves {
            tree.insert(leaf, &hash);
        }
        tree
    }
    //  #![feature(generic_const_exprs)]
    pub fn insert(&mut self, leaf: &[u8; N], hash: &impl Fn(&[u8]) -> [u8; N]) {
        let mut node = *leaf;
        let mut index = self.size();
        let mut depth = self.depth();

        if self.size() + 1 > (1 << depth) {
            self.nodes.push(Vec::new());
            depth += 1;
        }

        for level in &mut self.nodes {
            if level.len() <= index {
                level.push(node);
            } else {
                level[index] = node;
            }
            if index % 2 == 1 {
                let mut hash_input = Vec::with_capacity(N * 2);
                hash_input.extend_from_slice(&level[index - 1]);
                hash_input.extend_from_slice(&node);
                //                let mut hash_input = [0u8; N * 2];
                hash_input[..N].copy_from_slice(&level[index - 1]);
                hash_input[N..].copy_from_slice(&node);
                node = hash(&hash_input);
            }
            index >>= 1;
        }

        self.nodes[depth] = vec![node];
    }

    pub fn root(&self) -> Option<[u8; N]> {
        self.nodes.last()?.first().copied()
    }

    pub fn size(&self) -> usize {
        self.nodes.get(0).map(|v| v.len()).unwrap_or(0)
    }

    pub fn depth(&self) -> usize {
        self.nodes.len().saturating_sub(1)
    }
}
