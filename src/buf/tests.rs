use crate::protocol::Header;
use crate::protocol::{Endianness, Flags, MessageType, Variant};
use crate::{OwnedBuf, Signature};

#[rustfmt::skip]
const LE_BLOB: [u8; 36] = [
    // byte 0
    // yyyyuu fixed headers
    b'l',
    // reply (which is the simplest message)
    b'\x02',
    // no auto-starting
    b'\x02',
    // D-Bus version = 1
    b'\x01',
    // byte 4
    // bytes in body = 4
    b'\x04', b'\x00', b'\x00', b'\x00',
    // byte 8
    // serial number = 0x12345678
    b'\x78', b'\x56', b'\x34', b'\x12',
    // byte 12
    // a(uv) variable headers start here
    // bytes in array of variable headers = 15
    // pad to 8-byte boundary = nothing
    b'\x0f', b'\0', b'\0', b'\0',
    // byte 12
    // a(uv) variable headers start here
    // byte 16
    // in reply to:
    b'\x05',
    // variant signature = u
    // pad to 4-byte boundary = nothing
    b'\x01', b'u', b'\0',
    // 0xabcdef12
    // pad to 8-byte boundary = nothing
    b'\x12', b'\xef', b'\xcd', b'\xab', 
    // byte 24
    // signature:
    b'\x08',
    // variant signature = g
    b'\x01', b'g', b'\0',        
    // 1 byte, u, NUL (no alignment needed)
    b'\x01', b'u', b'\0',
    // pad to 8-byte boundary for body
    b'\0',
    // body; byte 32
    // 0xdeadbeef
    b'\xef', b'\xbe', b'\xad', b'\xde'
];

#[rustfmt::skip]
const BE_BLOB: [u8; 36] = [
    // byte 0
    // yyyyuu fixed headers
    b'B',
    // reply (which is the simplest message)
    b'\x02',
    // no auto-starting
    b'\x02',
    // D-Bus version = 1
    b'\x01',
    // byte 4
    // bytes in body = 4
    b'\x00', b'\x00', b'\x00', b'\x04',
    // byte 8
    // serial number = 0x12345678
    b'\x12', b'\x34', b'\x56', b'\x78',
    // byte 12
    // a(uv) variable headers start here
    // bytes in array of variable headers = 15
    // pad to 8-byte boundary = nothing
    b'\0', b'\0', b'\0', b'\x0f',
    // byte 12
    // a(uv) variable headers start here
    // byte 16
    // in reply to:
    b'\x05',
    // variant signature = u
    // pad to 4-byte boundary = nothing
    b'\x01', b'u', b'\0',
    // 0xabcdef12
    // pad to 8-byte boundary = nothing
    b'\xab', b'\xcd', b'\xef', b'\x12', 
    // byte 24
    // signature:
    b'\x08',
    // variant signature = g
    b'\x01', b'g', b'\0',        
    // 1 byte, u, NUL (no alignment needed)
    b'\x01', b'u', b'\0',
    // pad to 8-byte boundary for body
    b'\0',
    // body; byte 32
    // 0xdeadbeef
    b'\xde', b'\xad', b'\xbe', b'\xef',
];

#[test]
fn write_blobs() {
    let mut buf = OwnedBuf::with_endianness(Endianness::LITTLE);
    write_blob(&mut buf);
    assert_eq!(buf.get(), &LE_BLOB[..]);

    let mut buf = OwnedBuf::with_endianness(Endianness::BIG);
    write_blob(&mut buf);
    assert_eq!(buf.get(), &BE_BLOB[..]);
}

fn write_blob(buf: &mut OwnedBuf) {
    buf.write(&Header {
        endianness: buf.endianness(),
        message_type: MessageType::METHOD_RETURN,
        flags: Flags::default() | Flags::NO_AUTO_START,
        version: 1,
        body_length: 4,
        serial: 0x12345678u32,
    });

    let mut array = buf.write_array();

    let mut st = array.write_struct();
    st.write(&Variant::REPLY_SERIAL);
    st.write(Signature::UINT32);
    st.write(&0xabcdef12u32);

    let mut st = array.write_struct();
    st.write(&Variant::SIGNATURE);
    st.write(Signature::SIGNATURE);
    st.write(Signature::UINT32);

    array.finish();

    buf.align_mut::<u64>();
    buf.write(&0xdeadbeefu32);
}
