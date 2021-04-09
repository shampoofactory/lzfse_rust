use libc::{c_void, size_t};

use std::ptr;

pub fn encode(src: &[u8], dst: &mut [u8]) -> usize {
    unsafe {
        lzfse_encode_buffer(
            dst.as_mut_ptr() as *mut _,
            dst.len() as size_t,
            src.as_ptr() as *const _,
            src.len() as size_t,
            ptr::null_mut::<c_void>(),
        )
    }
}

pub fn decode(src: &[u8], dst: &mut [u8]) -> usize {
    unsafe {
        lzfse_decode_buffer(
            dst.as_mut_ptr() as *mut _,
            dst.len() as size_t,
            src.as_ptr() as *const _,
            src.len() as size_t,
            ptr::null_mut::<c_void>(),
        )
    }
}

extern "C" {
    // *  @return
    // *  The number of bytes written to the destination buffer if the input is
    // *  successfully compressed. If the input cannot be compressed to fit into
    // *  the provided buffer, or an error occurs, zero is returned, and the
    // *  contents of dst_buffer are unspecified.
    pub fn lzfse_encode_buffer(
        dst_buffer: *mut u8,
        dst_size: size_t,
        src_buffer: *const u8,
        src_size: size_t,
        scratch_buffer: *mut c_void,
    ) -> size_t;

    // *  @return
    // *  The number of bytes written to the destination buffer if the input is
    // *  successfully decompressed. If there is not enough space in the destination
    // *  buffer to hold the entire expanded output, only the first dst_size bytes
    // *  will be written to the buffer and dst_size is returned. Note that this
    // *  behavior differs from that of lzfse_encode_buffer.
    pub fn lzfse_decode_buffer(
        dst_buffer: *mut u8,
        dst_size: size_t,
        src_buffer: *const u8,
        src_size: size_t,
        scratch_buffer: *mut c_void,
    ) -> size_t;
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8] = b"The man who does not read good books has no\
    advantage over the man who cannot read them."; //  Mark Twain.

    #[test]
    fn encode_decode_all() {
        let mut enc = Vec::default();
        encode_all(DATA, &mut enc);
        let mut dec = Vec::default();
        decode_all(&enc, dec.as_mut());
        assert_eq!(DATA, &dec);
    }

    #[test]
    fn encode_decode() {
        let mut enc = [0u8; 256];
        let n = encode(DATA, enc.as_mut());
        let mut dec = [0u8; 256];
        let n = decode(&enc[..n], dec.as_mut());
        assert_eq!(DATA, &dec[..n]);
    }
}
