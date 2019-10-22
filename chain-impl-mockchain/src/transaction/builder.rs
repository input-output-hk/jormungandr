use std::marker::PhantomData;
use super::payload::Payload;
use super::input::Input;
use super::transfer::Output;
use super::transaction::NoExtra;
use super::witness::Witness;
use chain_addr::Address;

/// A Transaction builder with an associated state machine
pub struct TxBuilderState<T> {
    data: Vec<u8>,
    phantom: PhantomData<T>,
}

pub enum SetPayload {}
pub struct SetIOs<P: Payload>(PhantomData<P>);
pub struct SetWitnesses<P: Payload>(PhantomData<P>);
pub struct SetAuthData<P: Payload>(PhantomData<P>);
pub enum Finished {}

pub type TxBuilder = TxBuilderState<SetPayload>;

pub struct TData(Box<[u8]>);

// TODO not supported yet
pub const FRAGMENT_OVERHEAD : usize = 0;

impl TxBuilder {
    /// Create a new Tx builder
    pub fn new() -> Self {
        let mut data = Vec::new();
        // push empty hole for fragment overhead space
        for _ in 0..FRAGMENT_OVERHEAD {
            data.push(0u8);
        }
        TxBuilderState {
            data,
            phantom: PhantomData,
        }
    }
}

struct Hole(usize);

fn push_size_hole(data: &mut Vec<u8>) -> Hole {
    let pos = data.len();
    data.push(0);
    data.push(0);
    Hole(pos)
}

fn fill_hole(data: &mut Vec<u8>, hole: Hole, sz: u16) {
    let bytes = u16::to_le_bytes(sz);
    data[hole.0] = bytes[0];
    data[hole.0 + 1] = bytes[1];
}

fn fill_hole_diff(data: &mut Vec<u8>, hole: Hole) {
    let diff = data.len() - (hole.0 + 2);
    fill_hole(data, hole, diff as u16)
}

//fn current_pos()

impl TxBuilderState<SetPayload> {
    /// Set the payload of this transaction
    pub fn set_payload<P: Payload>(self, payload: &P) -> TxBuilderState<SetIOs<P>> {
        let mut data = self.data;
        if P::HAS_DATA {
            let hole = push_size_hole(&mut data);
            unimplemented!();
            fill_hole_diff(&mut data, hole);
        }
        TxBuilderState { data, phantom: PhantomData }
    }

    pub fn set_nopayload(self) -> TxBuilderState<SetIOs<NoExtra>> {
        self.set_payload(&NoExtra)
    }
}

impl<P: Payload> TxBuilderState<SetIOs<P>> {
    /// Set the inputs and outputs of this transaction
    pub fn set_ios(self, inputs: &[Input], outputs: &[Output<Address>]) -> TxBuilderState<SetWitnesses<P>> {
        assert!(inputs.len() < 255);
        assert!(outputs.len() < 255);
        let mut data = self.data;
        data.push(inputs.len() as u8);
        data.push(outputs.len() as u8);

        for i in inputs {
            data.extend_from_slice(&i.bytes());
        }

        for o in outputs {
            // TODO push output
        }

        TxBuilderState { data, phantom: PhantomData }
    }

}

impl<P: Payload> TxBuilderState<SetWitnesses<P>> {
    /// Get the authenticated data consisting of the payload and the input/outputs
    pub fn get_auth_data<'a>(&'a self) -> &'a [u8] {
        &self.data[FRAGMENT_OVERHEAD..]
    }

    /// Set the witnesses of the transaction. There's need to be 1 witness per inputs,
    /// although it is not enforced by this construction
    pub fn set_witnesses(self, witnesses: &[Witness]) -> TxBuilderState<SetAuthData<P>> {
        let mut data = self.data;
        unimplemented!();
        TxBuilderState { data, phantom: PhantomData }
    }
}

impl<P: Payload> TxBuilderState<SetAuthData<P>> {
    pub fn get_auth_data<'a>(&'a self) -> &'a [u8] {
        &self.data[FRAGMENT_OVERHEAD..]
    }
    //pub fn build_no_auth_data(self) -> TData {
    //    TData(self.0.into())
    //}

    /// Set the authenticated data 
    pub fn set_auth_data(self, auth_data: &P::Auth) -> TxBuilderState<Finished> {
        let mut data = self.data;
        TxBuilderState { data, phantom: PhantomData }
    }
}

impl TxBuilderState<Finished> {
    pub fn build(self) -> TData {
        TData(self.data.into())
    }
}
