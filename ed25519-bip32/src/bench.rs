use super::*;

fn new_sk() -> XPrv {
    let b = [0; XPRV_SIZE];
    XPrv::normalize_bytes(b)
}

#[bench]
fn derivate_hard_v1(b: &mut test::Bencher) {
    let sk = new_sk();
    b.iter(|| {
        let _ = sk.derive(DerivationScheme::V1, 0x80000000);
    })
}
#[bench]
fn derivate_hard_v2(b: &mut test::Bencher) {
    let sk = new_sk();
    b.iter(|| {
        let _ = sk.derive(DerivationScheme::V2, 0x80000000);
    })
}

#[bench]
fn derivate_soft_v1_xprv(b: &mut test::Bencher) {
    let sk = new_sk();
    b.iter(|| {
        let _ = sk.derive(DerivationScheme::V1, 0);
    })
}
#[bench]
fn derivate_soft_v2_xprv(b: &mut test::Bencher) {
    let sk = new_sk();
    b.iter(|| {
        let _ = sk.derive(DerivationScheme::V2, 0);
    })
}
#[bench]
fn derivate_soft_v1_xpub(b: &mut test::Bencher) {
    let sk = new_sk();
    let pk = sk.public();
    b.iter(|| {
        let _ = pk.derive(DerivationScheme::V1, 0);
    })
}
#[bench]
fn derivate_soft_v2_xpub(b: &mut test::Bencher) {
    let sk = new_sk();
    let pk = sk.public();
    b.iter(|| {
        let _ = pk.derive(DerivationScheme::V2, 0);
    })
}
