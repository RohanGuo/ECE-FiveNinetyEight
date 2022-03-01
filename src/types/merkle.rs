use ring::digest;

use super::hash::{Hashable, H256};

/// A Merkle tree. yeah
//#[derive(Debug, Default)]
#[derive(Eq, PartialEq, Clone, Hash, Default)]
pub struct Node {
    val: H256,
    parent: i32,
    left: i32, //https://www.reddit.com/r/rust/comments/acme4g/how_to_deal_with_a_tree_node_with/
    right: i32, //https://docs.rs/leetcode_prelude/0.1.2/leetcode_prelude/struct.TreeNode.html
    is_empty: bool,
}
#[derive(Eq, PartialEq, Clone, Hash, Default)]
pub struct MerkleTree {
    root: Vec<Node>, //Option<Box<Node>>, //for none
    leaf_size: usize,
    start_index: usize,
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        let input_len = data.len();
        let mut node_vec: Vec<Node> = Vec::new();
        if input_len == 0 {
            return MerkleTree {root: node_vec, leaf_size: 0, start_index: 0};
        }
        if input_len ==1 {
            let empty_node = Node{val:[0u8; 32].into(), parent: -1, left: -1, right: -1, is_empty: false};
            let node: Node = Node{val:data[0].hash(), parent: -1, left: -1, right: -1, is_empty: false};
            node_vec.push(empty_node);
            node_vec.push(node);
            return MerkleTree{root: node_vec, leaf_size: 1, start_index: 1};
        }

        let mut node_size = 1;
        while node_size < input_len {
            node_size = node_size * 2;
        }
        let start_index = node_size;
        node_size = node_size - 1 + input_len;
        //println!("node size {}", node_size);
        for i in 0..node_size + 1 {
            let mut node: Node = Node{val: data[0].hash(), parent: -1, left: -1, right: -1, is_empty: true};
            if i * 2 <= node_size {
                let left_index: i32 = (i * 2) as i32;
                node.left = left_index;
            }
            if i * 2 + 1 <= node_size {
                let right_index: i32 = (i * 2 + 1) as i32;
                node.right = right_index;
            }
            node_vec.push(node);
        }

        for i in start_index .. start_index + input_len {
            node_vec[i].val = data[i - start_index].hash();
            node_vec[i].is_empty = false;
        }
        for i in 1 .. start_index {
            if node_vec[start_index - i].left != -1 {
                let left = node_vec[start_index - i].left as usize;
                if node_vec[left].is_empty {
                    continue;
                }
                node_vec[start_index - i].is_empty = false;
                node_vec[left].parent = (start_index - i) as i32;
                if node_vec[start_index - i].right != -1 {
                    if node_vec[left + 1].is_empty {
                        node_vec[left + 1].val = node_vec[left].val;
                    }
                    let left_hash = node_vec[left].val.as_ref();
                    let right_hash = node_vec[left + 1].val.as_ref(); //as_ref to indice
                    let parent = [&left_hash[..], &right_hash[..]].concat();
                    let parent_hash_digest = digest::digest(&digest::SHA256, &parent); //degest类型的
                    let parent_hash = <H256>::from(parent_hash_digest);
                    node_vec[start_index - i].val = parent_hash;
                    node_vec[left + 1].parent = (start_index - i) as i32;
                } else {
                    node_vec[start_index - i].val = node_vec[left].val;
                }
            }
        }
        MerkleTree { root: node_vec, leaf_size: input_len, start_index: start_index}
    }

    pub fn root(&self) -> H256 {
        if self.leaf_size < 1 {
            return [0u8; 32].into();
        }
        else {
            return self.root[1].val;
        }
    }


    pub fn print(&self) {
        println!("leaf_size: {}, start_index: {}", self.leaf_size, self.start_index);
        for i in 1 .. (self.start_index + self.leaf_size) {
            println!("{}, {:?}, {}, {}, {}, {}", i, self.root[i as usize].val, self.root[i as usize].left, self.root[i as usize].right, self.root[i as usize].is_empty, self.root[i as usize].parent);
        }
    }
    /// Returns the Merkle Proof of data at index i
    /// return the hash from botttom to ceiling 
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut res_vec: Vec<H256> = Vec::new(); //store result
        if self.leaf_size == 1 { //directly store the proof
            res_vec.push(self.root[1].val);
            return res_vec;
        }
        let idx = self.start_index + index; // start_index already add 1
        let mut child: Node = self.root[idx].clone(); // find the input data
        while child.parent != -1 { 
            let father = self.root[child.parent as usize].clone();
            let brother: Node = if self.root[father.left as usize] == child { //find brother
                self.root[father.right as usize].clone()
            }else{
                self.root[father.left as usize].clone()
            };
            res_vec.push(brother.val);
            child = father.clone();
        }
        return res_vec; //the proof not include the root_node
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
/// 
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let mut data = *datum;
    let mut idx = index + 1; // to determine left and right
    if leaf_size == 1 {
        return *root == proof[0];
    }
    for i in 0..proof.len() { //concat from the first hash (bottom) to the last hash
        let proof_to_concat = proof[i].as_ref();
        let data_to_concat = data.as_ref();
        if idx % 2 == 1 { // total input = 6, idx = 5, so left concat
            let right = proof_to_concat;
            let left = data_to_concat;
            let parent_concat = [&left[..], &right[..]].concat();
            let parent_hash = <H256>::from(digest::digest(&digest::SHA256, &parent_concat));
            idx = idx/2+1; //
            data = parent_hash;
        }
        else { // total input = 6, idex = 4, left = proof
            let right = data_to_concat;
            let left = proof_to_concat;
            let parent_concat = [&left[..], &right[..]].concat();
            let parent_hash = <H256>::from(digest::digest(&digest::SHA256, &parent_concat));
            idx = idx/2;
            data = parent_hash;
        }
    }    
    return data == *root;
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use crate::types::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data { 
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );

        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
