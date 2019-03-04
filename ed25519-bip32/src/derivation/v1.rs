use cryptoxide::curve25519::sc_reduce;

pub fn add_256bits_v1(x: &[u8], y: &[u8]) -> [u8; 32] {
    assert!(x.len() == 32);
    assert!(y.len() == 32);

    let mut out = [0u8; 32];
    for i in 0..32 {
        let r = x[i].wrapping_add(y[i]);
        out[i] = r;
    }
    out
}

pub fn add_28_mul8_v1(x: &[u8], y: &[u8]) -> [u8; 32] {
    assert!(x.len() == 32);
    assert!(y.len() == 32);

    let yfe8 = {
        let mut acc = 0;
        let mut out = [0u8; 64];
        for i in 0..32 {
            out[i] = (y[i] << 3) + (acc & 0x8);
            acc = y[i] >> 5;
        }
        out
    };

    let mut r32 = [0u8; 32];
    let mut r = [0u8; 64];
    let mut carry = 0u16;
    for i in 0..32 {
        let v = x[i] as u16 + yfe8[i] as u16 + carry;
        r[i] = v as u8;
        carry = v >> 8;
    }
    if carry > 0 {
        r[32] = carry as u8;
    }
    sc_reduce(&mut r);
    r32.clone_from_slice(&r[0..32]);
    r32
}

pub fn be32(i: u32) -> [u8; 4] {
    [(i >> 24) as u8, (i >> 16) as u8, (i >> 8) as u8, i as u8]
}
