use crate::error::Result;
use crate::proto::{self, Header};
use crate::proto::{Endianness, Flags, MessageType};
use crate::ty;
use crate::{BodyBuf, Signature, Variant};

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
fn write_blobs() -> Result<()> {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    write_blob(&mut buf)?;
    assert_eq!(buf.get(), &LE_BLOB[..]);

    let mut buf = BodyBuf::with_endianness(Endianness::BIG);
    write_blob(&mut buf)?;
    assert_eq!(buf.get(), &BE_BLOB[..]);
    Ok(())
}

fn write_blob(buf: &mut BodyBuf) -> Result<()> {
    buf.store(Header {
        endianness: buf.endianness(),
        message_type: MessageType::METHOD_RETURN,
        flags: Flags::default() | Flags::NO_AUTO_START,
        version: 1,
        body_length: 4,
        serial: 0x12345678u32,
    })?;

    let mut array = buf.store_array::<(proto::Variant, ty::Variant)>()?;

    array
        .store_struct()
        .store(proto::Variant::REPLY_SERIAL)
        .store(Variant::U32(0xabcdef12u32))
        .finish();

    array
        .store_struct()
        .store(proto::Variant::SIGNATURE)
        .store(Variant::Signature(Signature::UINT32))
        .finish();

    array.finish();

    buf.store(0xdeadbeefu32)?;
    Ok(())
}

#[test]
fn test_read_buf() -> Result<()> {
    let mut buf = BodyBuf::new();

    buf.store(4u32)?;
    buf.extend_from_slice_nul(b"\x01\x02\x03\x04");

    let mut buf = buf.as_body();

    let mut read_buf = buf.read_until(6)?;

    assert_eq!(read_buf.load::<u32>()?, 4);
    assert_eq!(read_buf.load::<u8>()?, 1);
    assert_eq!(read_buf.load::<u8>()?, 2);
    assert_eq!(buf.get(), &[3, 4, 0]);
    Ok(())
}

#[test]
fn test_read_buf_load() -> Result<()> {
    let mut buf = BodyBuf::new();
    buf.store(7u32)?;
    buf.extend_from_slice_nul(b"foo bar");

    let mut buf = buf.as_body();

    let mut read_buf = buf.read_until(6)?;

    assert_eq!(read_buf.load::<u32>()?, 7u32);
    assert_eq!(read_buf.load::<u8>()?, b'f');
    assert_eq!(read_buf.load::<u8>()?, b'o');
    assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
    Ok(())
}

#[test]
fn test_read_buf_read() -> Result<()> {
    let mut buf = BodyBuf::new();
    buf.store(4u32)?;
    buf.extend_from_slice_nul(b"\x01\x02\x03\x04");

    let mut buf = buf.as_body();

    let mut read_buf = buf.read_until(6)?;

    assert_eq!(read_buf.load::<u32>()?, 4);
    assert_eq!(read_buf.load::<u8>()?, 1);
    assert_eq!(read_buf.load::<u8>()?, 2);
    assert!(read_buf.load::<u8>().is_err());
    assert!(read_buf.is_empty());

    let _ = buf.read_until(3)?;
    assert!(buf.is_empty());
    Ok(())
}

#[test]
fn test_nested_read_buf() -> Result<()> {
    let mut buf = BodyBuf::new();
    buf.store(4u32)?;
    buf.extend_from_slice_nul(b"\x01\x02\x03\x04");

    let mut buf = buf.as_body();

    let mut read_buf = buf.read_until(6)?;
    assert_eq!(read_buf.load::<u32>()?, 4);

    let mut read_buf2 = read_buf.read_until(2)?;
    assert_eq!(read_buf2.load::<u8>()?, 1);
    assert_eq!(read_buf2.load::<u8>()?, 2);

    assert!(read_buf.is_empty());
    assert!(read_buf2.is_empty());

    assert_eq!(buf.get(), &[3, 4, 0]);
    Ok(())
}

/// The unwritten capacity handed to socket reads must be initialized, both
/// after the first allocation and after growing an existing one.
#[test]
#[cfg(feature = "tokio")]
fn get_mut_is_initialized() {
    use crate::buf::{AlignedBuf, UnalignedBuf};

    let mut buf = AlignedBuf::new();
    buf.reserve_bytes(4);
    assert!(buf.get_mut().iter().all(|b| *b == 0));
    buf.extend_from_slice(&[1; 16]);
    buf.reserve_bytes(64);
    assert!(buf.get_mut().iter().all(|b| *b == 0));

    let mut buf = UnalignedBuf::new();
    buf.reserve_bytes(4);
    assert!(buf.get_mut().iter().all(|b| *b == 0));
    buf.extend_from_slice(&[1; 16]);
    buf.reserve_bytes(64);
    assert!(buf.get_mut().iter().all(|b| *b == 0));
}

/// D-Bus alignment is fixed by the specification, regardless of the alignment
/// the target gives the corresponding Rust type.
#[test]
fn wire_alignment_is_fixed() -> Result<()> {
    use crate::Alignment;

    assert_eq!(Alignment::of::<u64>(), Alignment::U64);
    assert_eq!(Alignment::of::<i64>(), Alignment::U64);
    assert_eq!(Alignment::of::<f64>(), Alignment::U64);
    assert_eq!(Alignment::of::<(u8, u8)>(), Alignment::U64);

    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    buf.store(1u32)?;
    buf.store(2u64)?;
    buf.store(3u8)?;
    buf.store(4.0f64)?;
    buf.store_struct::<(u8,)>()?.store(5u8).finish();

    assert_eq!(buf.signature(), "utyd(y)");
    #[rustfmt::skip]
    assert_eq!(buf.get(), &[
        1, 0, 0, 0, 0, 0, 0, 0,
        2, 0, 0, 0, 0, 0, 0, 0,
        3, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0x10, 0x40,
        5,
    ]);

    assert_eq!(buf.get().as_ptr() as usize % 8, 0);
    Ok(())
}
