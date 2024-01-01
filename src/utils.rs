use tokio::{
    io::{ReadHalf, WriteHalf},
    net::TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::encoding::NetworkMessage;

pub type MessageWriter<T> = FramedWrite<WriteHalf<TcpStream>, NetworkMessage<T>>;
pub type MessageReader<T> = FramedRead<ReadHalf<TcpStream>, NetworkMessage<T>>;

pub fn frame_socket<R, W>(st: TcpStream) -> (MessageReader<R>, MessageWriter<W>)
where
    for<'de> R: serde::Deserialize<'de>,
    W: serde::Serialize,
{
    let (r, w) = tokio::io::split(st);
    (
        FramedRead::new(r, NetworkMessage::<R>::new()),
        FramedWrite::new(w, NetworkMessage::<W>::new()),
    )
}
