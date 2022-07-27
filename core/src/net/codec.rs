use super::{cipher::Cipher, packet::Packet, shanda};
use crate::{Error, Result};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub struct MapleCodec {
    send: Cipher,
    recv: Cipher,
}

impl MapleCodec {
    pub fn new(send: Cipher, recv: Cipher) -> Self {
        Self { send, recv }
    }

    fn create_packet_header(&self, len: usize) -> [u8; 4] {
        let len = len as u32;
        let mut a = u32::from(self.send.iv[3] & 0xff);
        a |= (u32::from(self.send.iv[2]) << 8) & 0xff00;
        a ^= u32::from(self.send.version);

        let b = a ^ (((len << 8) & 0xff00) | len >> 8);

        [
            ((a >> 8) & 0xff) as u8,
            (a & 0xff) as u8,
            ((b >> 8) & 0xff) as u8,
            (b & 0xff) as u8,
        ]
    }

    fn is_valid_header(&self, header: &BytesMut) -> bool {
        ((header[0] ^ self.recv.iv[2]) & 0xff) == ((self.recv.version >> 8) as u8 & 0xff)
            && (((header[1] ^ self.recv.iv[3]) & 0xff) == (self.recv.version & 0xff) as u8)
    }
}

impl Encoder<Packet> for MapleCodec {
    type Error = Error;

    fn encode(&mut self, packet: Packet, buf: &mut BytesMut) -> Result<()> {
        // create the packet header
        let header = self.create_packet_header(packet.len());
        // encrypt the packet body
        let encrypted = self.send.transform(shanda::encrypt(packet.bytes));

        buf.reserve(header.len() + encrypted.len());
        buf.put_slice(&header);
        buf.put(encrypted);

        Ok(())
    }
}

impl Decoder for MapleCodec {
    type Item = Packet;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Packet>> {
        // we need to check if the packet is as least 4 bytes otherwise
        // split_off will panic (and packet/header is invalid)
        if buf.len() < 4 {
            log::error!("Invalid packet length");
            return Ok(None);
        }

        // first 4 bytes of packet contain the header, remaining contain the body
        let mut bytes = buf.split_to(buf.len());
        // after split_off(), bytes will contain the first 4 bytes/header
        let body = bytes.split_off(4);

        // validate the packet header
        if !self.is_valid_header(&bytes) {
            log::error!("Invalid packet header");
            return Ok(None);
        }

        // decrypt the packet body
        let decrypted = shanda::decrypt(self.recv.transform(body));
        let packet = Packet::wrap(decrypted);

        Ok(Some(packet))
    }
}