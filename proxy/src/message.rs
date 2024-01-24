use anyhow::{anyhow, Result};
use byteorder::ByteOrder;
// use serde::{Deserialize, Serialize};
// use serde_big_array::BigArray;

// #[derive(Deserialize, Serialize, Debug)]
pub struct Message {
    // #[serde(with = "BigArray")]
    pub body: [u8; 502],
    len: usize,
    id: usize,
}

// [u8; 512] <-> Msg <-> Vec<u8> <-> [u8; 528] <-> Msg <-> Msg.body([u8; 512])
impl Message {
    pub fn from_str_to_vec(id: usize, req: &str) -> [u8; 512] {
        let mut tmp = [0; 512];
        let req_u8 = req.as_bytes();
        let len = req_u8.len();
        // println!("len is {len}");
        for i in 0..len {
            tmp[i] = req_u8[i];
        }

        byteorder::NativeEndian::write_u64(&mut tmp[496..504], len as u64);
        byteorder::NativeEndian::write_u64(&mut tmp[504..512], id as u64);

        tmp
    }

    pub fn from_body_to_vec(id: usize, req: &mut [u8], len: usize) -> Result<()> {
        if len > 512 {
            return Err(anyhow!("len is too big!"));
        }

        byteorder::NativeEndian::write_u64(&mut req[496..504], len as u64);
        byteorder::NativeEndian::write_u64(&mut req[504..512], id as u64);

        Ok(())
    }

    // pub fn from_vecu8(req: Vec<u8>) -> Result<Message> {
    //     match bincode::deserialize(&req) {
    //         Ok(s) => Ok(s),
    //         Err(e) => return Err(anyhow!("error in from Vec<u8>:\n{e}")),
    //     }
    // }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn len_fromn_u8(msg: &[u8]) -> usize {
        let buf = msg[496..504].try_into().unwrap();
        usize::from_ne_bytes(buf)
    }

    pub fn id_fromn_u8(msg: &[u8]) -> usize {
        let buf = msg[504..512].try_into().unwrap();
        usize::from_ne_bytes(buf)
    }
}

#[test]
fn test() {
    let tmp = "I am good boy";
    println!("tmp is: {:?}", tmp.as_bytes());
    let s = Message::from_str_to_vec(1, tmp);
    let len = Message::len_fromn_u8(&s);
    let req = std::str::from_utf8(&s[..len]).unwrap();
    println!("{req}");
}
