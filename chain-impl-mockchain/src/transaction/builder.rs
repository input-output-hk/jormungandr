use super::input::Input;
use super::payload::{NoExtra, Payload};
use super::transaction::{
    Transaction, TransactionAuthData, TransactionBindingAuthData, TransactionStruct,
};
use super::transfer::Output;
use super::witness::Witness;
use chain_addr::Address;
use std::marker::PhantomData;

/// A Transaction builder with an associated state machine
pub struct TxBuilderState<T> {
    data: Vec<u8>,
    tstruct: TransactionStruct,
    phantom: PhantomData<T>,
}

impl<T> Clone for TxBuilderState<T> {
    fn clone(&self) -> Self {
        TxBuilderState {
            data: self.data.clone(),
            tstruct: self.tstruct.clone(),
            phantom: self.phantom,
        }
    }
}

pub enum SetPayload {}
pub struct SetIOs<P>(PhantomData<P>);
pub struct SetWitnesses<P>(PhantomData<P>);
pub struct SetAuthData<P: Payload>(PhantomData<P>);

pub type TxBuilder = TxBuilderState<SetPayload>;

// TODO not supported yet
pub const FRAGMENT_OVERHEAD: usize = 0;

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
            tstruct: TransactionStruct {
                sz: 0,
                nb_inputs: 0,
                nb_outputs: 0,
                inputs: 0,
                outputs: 0,
                witnesses: 0,
                payload_auth: 0,
            },
            phantom: PhantomData,
        }
    }
}

impl<State> TxBuilderState<State> {
    fn current_pos(&self) -> usize {
        self.data.len() - FRAGMENT_OVERHEAD
    }
}

impl TxBuilderState<SetPayload> {
    /// Set the payload of this transaction
    pub fn set_payload<P: Payload>(mut self, payload: &P) -> TxBuilderState<SetIOs<P>> {
        if P::HAS_DATA {
            self.data.extend_from_slice(payload.payload_data().as_ref());
        }

        TxBuilderState {
            data: self.data,
            tstruct: self.tstruct,
            phantom: PhantomData,
        }
    }

    pub fn set_nopayload(self) -> TxBuilderState<SetIOs<NoExtra>> {
        self.set_payload(&NoExtra)
    }
}

impl<P> TxBuilderState<SetIOs<P>> {
    /// Set the inputs and outputs of this transaction
    ///
    /// This cannot accept more than 255 inputs, 255 outputs, since
    /// the length is encoded as u8, and hence will assert.
    ///
    /// Note that further restriction apply to the ledger,
    /// which only accept up to 254 outputs
    pub fn set_ios(
        mut self,
        inputs: &[Input],
        outputs: &[Output<Address>],
    ) -> TxBuilderState<SetWitnesses<P>> {
        assert!(inputs.len() < 256);
        assert!(outputs.len() < 256);

        let nb_inputs = inputs.len() as u8;
        let nb_outputs = outputs.len() as u8;

        self.data.push(nb_inputs);
        self.data.push(nb_outputs);

        self.tstruct.nb_inputs = nb_inputs;
        self.tstruct.nb_outputs = nb_outputs;

        self.tstruct.inputs = self.current_pos();

        for i in inputs {
            self.data.extend_from_slice(&i.bytes());
        }

        self.tstruct.outputs = self.current_pos();

        for o in outputs {
            self.data.extend_from_slice(&o.address.to_bytes());
            self.data.extend_from_slice(&o.value.bytes());
        }

        TxBuilderState {
            data: self.data,
            tstruct: self.tstruct,
            phantom: PhantomData,
        }
    }
}

impl<P> TxBuilderState<SetWitnesses<P>> {
    /// Get the authenticated data consisting of the payload and the input/outputs
    pub fn get_auth_data_for_witness<'a>(&'a self) -> TransactionAuthData<'a> {
        TransactionAuthData(&self.data[FRAGMENT_OVERHEAD..])
    }

    /// Set the witnesses of the transaction. There's need to be 1 witness per inputs,
    /// although it is not enforced by this construction
    ///
    /// Note that the same number of witnesses as the number of inputs need to be added here,
    /// otherwise an assert will raise.
    pub fn set_witnesses(mut self, witnesses: &[Witness]) -> TxBuilderState<SetAuthData<P>>
    where
        P: Payload,
    {
        assert_eq!(witnesses.len(), self.tstruct.nb_inputs as usize);
        self.tstruct.witnesses = self.current_pos();
        for w in witnesses {
            self.data.extend_from_slice(&w.to_bytes())
        }
        TxBuilderState {
            data: self.data,
            tstruct: self.tstruct,
            phantom: PhantomData,
        }
    }
}

impl<P: Payload> TxBuilderState<SetAuthData<P>> {
    /// Get the authenticated data related to possible overall data for transaction and payload binding
    pub fn get_auth_data<'a>(&'a self) -> TransactionBindingAuthData<'a> {
        TransactionBindingAuthData(&self.data[FRAGMENT_OVERHEAD..])
    }

    /// Set the authenticated data
    pub fn set_payload_auth(mut self, auth_data: &P::Auth) -> Transaction<P> {
        self.tstruct.payload_auth = self.current_pos();
        if P::HAS_DATA && P::HAS_AUTH {
            self.data
                .extend_from_slice(<P as Payload>::payload_auth_data(auth_data).as_ref());
        }
        self.tstruct.sz = self.current_pos();
        Transaction {
            data: self.data.into(),
            tstruct: self.tstruct,
            phantom: PhantomData,
        }
    }
}
