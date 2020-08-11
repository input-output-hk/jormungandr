use bip39::Entropy;
use cryptoxide::hmac::Hmac;
use cryptoxide::pbkdf2::pbkdf2;
use cryptoxide::sha2::Sha512;

pub fn generate_seed(entropy: &Entropy, password: &[u8], output: &mut [u8]) {
    const ITER: u32 = 4096;
    let mut mac = Hmac::new(Sha512::new(), password);
    pbkdf2(&mut mac, entropy.as_ref(), ITER, output)
}
