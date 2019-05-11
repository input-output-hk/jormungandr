use crate::key::{
    deserialize_public_key, deserialize_signature, serialize_public_key, serialize_signature,
};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::{Ed25519Extended, PublicKey, Verification};

use std::collections::BTreeMap;

use super::declaration::{
    owners_to_identifier, DeclElement, Declaration, Pk, Sig, WitnessMultisigData,
};
use super::index::{Index, TreeIndex};
use super::ledger::LedgerError;

/// Witness for multisig
#[derive(Debug, Clone)]
pub struct Witness(Vec<(TreeIndex, Pk, Sig)>);

impl Witness {
    pub fn verify(&self, declaration: &Declaration, msg: &WitnessMultisigData) -> bool {
        let mut v = Vec::new();
        for (ti, pk, sig) in self.0.iter() {
            match ti {
                TreeIndex::D2(_, _) => {
                    // Code doesn't support multi level verification yet
                    return false;
                }
                TreeIndex::D1(i) => {
                    //let sig: Signature<[u8], account::AccountAlg> = sig.clone().coerce();
                    if sig.verify(pk, msg) == Verification::Failed {
                        return false;
                    };
                    v.push((*i, pk.clone()))
                }
            }
        }
        if verify_identifier_threshold(declaration, &v[..]).is_err() {
            return false;
        };
        true
    }
}

fn deserialize_index<'a>(buf: &mut ReadBuf<'a>) -> Result<TreeIndex, ReadError> {
    let idx = buf.get_u16()?;
    match TreeIndex::unpack(idx) {
        None => Err(ReadError::StructureInvalid("invalid index".to_string())),
        Some(ti) => Ok(ti),
    }
}

impl property::Serialize for Witness {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::new(writer);
        codec.put_u8(self.0.len() as u8)?;
        for (ti, pk, sig) in self.0.iter() {
            codec.put_u16(ti.pack())?;
            serialize_public_key(pk, &mut codec)?;
            serialize_signature(sig, &mut codec)?;
        }
        Ok(())
    }
}

impl Readable for Witness {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let len = buf.get_u8()? as usize;

        if len == 0 {
            return Err(ReadError::StructureInvalid(
                "zero length not permitted".to_string(),
            ));
        }

        let first_index = deserialize_index(buf)?;
        let first_key = deserialize_public_key(buf)?;
        let first_sig = deserialize_signature(buf)?;

        let mut v = Vec::new();
        v.push((first_index, first_key, first_sig));

        let mut prev_index = first_index;
        for _ in 0..len {
            let ti = deserialize_index(buf)?;
            if ti <= prev_index {
                return Err(ReadError::StructureInvalid(
                    "index not in order".to_string(),
                ));
            }
            let pk = deserialize_public_key(buf)?;
            let sig = deserialize_signature(buf)?;
            prev_index = ti;
            v.push((ti, pk, sig))
        }
        Ok(Witness(v))
    }
}

pub struct WitnessBuilder(BTreeMap<TreeIndex, (Pk, Sig)>);

impl WitnessBuilder {
    pub fn new() -> Self {
        WitnessBuilder(BTreeMap::new())
    }

    pub fn append(&mut self, index: TreeIndex, publickey: Pk, sig: Sig) {
        // TODO turn this into a proper error
        let r = self.0.insert(index, (publickey, sig));
        assert_eq!(r.is_none(), true);
    }

    pub fn finalize(&self) -> Witness {
        let mut v = Vec::new();
        for (idx, (pk, sig)) in self.0.iter() {
            v.push((*idx, pk.clone(), sig.clone()))
        }
        Witness(v)
    }
}

/// Verify that the declaration and the witnesses in parameters fulfill the requirements:
///
/// * The threshold is met: there's at least T or more witnesses available
/// * the witnesses and declaration together can re-create
pub fn verify_identifier_threshold(
    declaration: &Declaration,
    witnesses: &[(Index, PublicKey<Ed25519Extended>)],
) -> Result<(), LedgerError> {
    if witnesses.len() < declaration.threshold() {
        return Err(LedgerError::ThresholdNotMet);
    }

    let mut opt = vec![None; declaration.total()];

    for (i, w) in witnesses {
        let idx = i.to_usize();
        if idx >= opt.len() {
            return Err(LedgerError::ParticipantOutOfBound);
        }
        opt[idx] = Some(w.clone())
    }
    let mut r = Vec::new();
    for (i, v) in opt.iter().enumerate() {
        // here we abuse DeclElement::Owner to mean hash
        match v {
            //None => r.push(DeclElement::Owner(declaration.owners[i].to_hash().clone())),
            None => r.push(declaration.owners[i].clone()),
            Some(p) => r.push(DeclElement::from_publickey(p)),
        }
    }
    let got = owners_to_identifier(declaration.threshold() as u8, &r);
    if got != declaration.to_identifier() {
        return Err(LedgerError::IdentifierMismatch);
    }
    Ok(())
}
