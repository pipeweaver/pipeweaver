use interprocess::local_socket::prelude::LocalSocketStream;
use serde::{Serialize, de::DeserializeOwned};
use serde_json;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug)]
pub struct Socket<In, Out> {
    address: SocketAddr,
    stream: LocalSocketStream,
    _marker: std::marker::PhantomData<(In, Out)>,
}

impl<In, Out> Socket<In, Out>
where
    In: DeserializeOwned,
    Out: Serialize,
{
    pub fn new(stream: LocalSocketStream) -> Self {
        Self {
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            stream,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn read(&mut self) -> Option<Result<In, Error>> {
        let mut len = [0u8; 4];

        if let Err(e) = self.stream.read_exact(&mut len) {
            if e.kind() == ErrorKind::UnexpectedEof {
                return None;
            }
            return Some(Err(e));
        }

        let len = u32::from_be_bytes(len) as usize;

        let mut data = vec![0u8; len];
        if let Err(e) = self.stream.read_exact(&mut data) {
            return Some(Err(e));
        }

        Some(serde_json::from_slice(&data).map_err(|e| Error::new(ErrorKind::InvalidData, e)))
    }

    pub fn try_read(&mut self) -> Result<Option<In>, Error> {
        match self.read() {
            Some(Ok(value)) => Ok(Some(value)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    pub fn send(&mut self, out: Out) -> Result<(), Error> {
        let data = serde_json::to_vec(&out).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        let len = (data.len() as u32).to_be_bytes();

        self.stream.write_all(&len)?;
        self.stream.write_all(&data)?;
        self.stream.flush()
    }

    pub fn address(&self) -> &SocketAddr {
        &self.address
    }
}
