use std::ops::Add;

use serde::{Serialize, Deserialize}; // 序列化

// 20-byte address
#[derive(Eq, PartialEq, Serialize, Deserialize, Clone, Hash, Default, Copy)]
pub struct Address([u8; 20]); //有很多属性

/*
pub trait Hashable {
    /// Hash the object using SHA256.
    fn hash(&self) -> Address;
}
impl Hashable for H256 {
    fn hash(&self) -> H256 {
        ring::digest::digest(&ring::digest::SHA256, &self.0).into()
    }
}
*/

impl std::convert::From<&[u8; 20]> for Address { //转换类型, 数组类型
    fn from(input: &[u8; 20]) -> Address {
        let mut buffer: [u8; 20] = [0; 20];  //初始化，全零
        buffer[..].copy_from_slice(input);
        Address(buffer)
    }
}

impl std::convert::From<[u8; 20]> for Address { //转换类型
    fn from(input: [u8; 20]) -> Address {
        Address(input)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let start = if let Some(precision) = f.precision() { //start为0到20
            if precision >= 40 {
                0
            } else {
                20 - precision / 2
            }
        } else {
            0
        };
        for byte_idx in start..20 {
            write!(f, "{:>02x}", &self.0[byte_idx])?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:>02x}{:>02x}..{:>02x}{:>02x}",
            &self.0[0], &self.0[1], &self.0[18], &self.0[19]
        )
    }
}
/*impl Address{
    pub fn from_public_key_bytes(bytes: &[u8]) -> Address {
        unimplemented!()
    }
}*/
impl std::convert::From<ring::digest::Digest> for Address {
    fn from(input: ring::digest::Digest) -> Address{
    //fn from(input: ring::digest::Digest) -> Address {
        let mut raw_hash: [u8; 20] = [0; 20];
        raw_hash[0..20].copy_from_slice(input.as_ref());
        Address(raw_hash)
    }
}
impl Address {
    pub fn from_public_key_bytes(bytes: &[u8]) -> Address {
        //hash(bytes);
        //hash(bytes.as_ref());
        let hash_d = ring::digest::digest(&ring::digest::SHA256, bytes);
        // Digest 变为了 Address
        //let input: [u8; 20] = hash_input.0;
        let mut cut_hash:[u8; 20] = [0; 20];
        cut_hash[0..20].copy_from_slice(&hash_d.as_ref()[12..32]);
        Address(cut_hash)
        //hash+last 20 byte;  
    }
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use super::Address;

    #[test]
    fn from_a_test_key() {
        let test_key = hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d");
        let addr = Address::from_public_key_bytes(&test_key);
        let correct_addr: Address = hex!("1851a0eae0060a132cf0f64a0ffaea248de6cba0").into();
        assert_eq!(addr, correct_addr);
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // take the last 20 bytes, we get "1851a0eae0060a132cf0f64a0ffaea248de6cba0"
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
