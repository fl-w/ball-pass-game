use std::convert::TryFrom;
use std::marker::PhantomData;

use byteorder::ReadBytesExt;
use bytes::{Buf, BufMut, BytesMut};
use log::error;
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not de/serialze")]
    Serialization(#[from] bincode::Error),

    #[error("IO error")]
    IO(#[from] std::io::Error),

    #[error("payload too large")]
    LargePayload,
}

// +----------+--------------------------------+
// | len: u16 |          frame payload         |
// +----------+--------------------------------+
#[derive(Debug, Default)]
pub struct NetworkMessage<T> {
    __: PhantomData<T>,
}

impl<T> NetworkMessage<T> {
    pub fn new() -> Self { Self { __: PhantomData } }
}

impl<T> Encoder<T> for NetworkMessage<T>
where
    T: Serialize,
{
    type Error = Error;

    fn encode(&mut self, msg: T, buf: &mut BytesMut) -> Result<()> {
        let msg = bincode::serialize(&msg)?;
        let msg_len = msg.len();

        // reserve space for bytelen
        if u16::try_from(msg_len).is_err() {
            error!("payload size can't be larger than 16 bytes");
            Err(Error::LargePayload)
        } else {
            buf.reserve(2 + msg_len);

            buf.put_u16(msg_len as u16);
            buf.put(&msg[..]);

            Ok(())
        }
    }
}

impl<T> Decoder for NetworkMessage<T>
where
    for<'de> T: Deserialize<'de>,
{
    type Item = T;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.len() <= 2 {
            // there are no bytes to consume, stop querying the buffer
            return Ok(None);
        }

        // parse out the bytes from the start of the buffer
        let payload_size = src.as_ref().read_u16::<byteorder::BigEndian>()? as usize;

        // read payload
        let current_frame_size = 2 + payload_size;

        if src.len() < current_frame_size {
            // no payload yet
            // reserve place for the current frame and the next header for better efficiency
            src.reserve(current_frame_size);
            return Ok(None);
        }

        // skip header frame
        src.advance(2);

        let data = &src.split_to(payload_size).freeze();
        Ok(Some(bincode::deserialize(data)?))
    }
}
