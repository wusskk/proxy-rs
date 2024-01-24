use anyhow::{anyhow, Result};
use byteorder::ByteOrder;

pub struct Message {
    // #[serde(with = "BigArray")]
    pub body: [u8; 4080],
    len: usize,
    id: usize,
}

pub const ARR_LEN: usize = 4080;
pub const MSG_LEN: usize = ARR_LEN + 16;

impl Message {
    const DIVIDE: usize = ARR_LEN + 8;

    pub fn from_str_to_vec(id: usize, req: &str) -> [u8; MSG_LEN] {
        let mut tmp = [0; MSG_LEN];
        let req_u8 = req.as_bytes();
        let len = req_u8.len();
        // println!("len is {len}");
        for i in 0..len {
            tmp[i] = req_u8[i];
        }

        byteorder::NativeEndian::write_u64(&mut tmp[ARR_LEN..Self::DIVIDE], len as u64);
        byteorder::NativeEndian::write_u64(&mut tmp[Self::DIVIDE..MSG_LEN], id as u64);

        tmp
    }

    pub fn from_body_to_vec(id: usize, req: &mut [u8], len: usize) -> Result<()> {
        if len > ARR_LEN {
            return Err(anyhow!("len is too big!"));
        }

        byteorder::NativeEndian::write_u64(&mut req[ARR_LEN..Self::DIVIDE], len as u64);
        byteorder::NativeEndian::write_u64(&mut req[Self::DIVIDE..MSG_LEN], id as u64);

        Ok(())
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn len_fromn_u8(msg: &[u8]) -> usize {
        let buf = msg[ARR_LEN..Self::DIVIDE].try_into().unwrap();
        usize::from_ne_bytes(buf)
    }

    pub fn id_fromn_u8(msg: &[u8]) -> usize {
        let buf = msg[Self::DIVIDE..MSG_LEN].try_into().unwrap();
        usize::from_ne_bytes(buf)
    }
}
