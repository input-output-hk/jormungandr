#[derive(Debug, Clone)]
pub struct Utxo {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}
#[derive(Debug, Clone)]
pub struct Account {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}
#[derive(Debug, Clone)]
pub struct Delegation {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
    pub delegation_address: String,
}

pub trait AddressDataProvider {
    fn get_address(&self) -> String;
    fn get_private_key(&self) -> String;
    fn get_address_type(&self) -> String;
}

impl AddressDataProvider for Utxo {
    fn get_address(&self) -> String {
        self.address.clone()
    }

    fn get_private_key(&self) -> String {
        self.private_key.clone()
    }

    fn get_address_type(&self) -> String {
        let address_type = String::from("utxo");
        address_type
    }
}

impl AddressDataProvider for Account {
    fn get_address(&self) -> String {
        self.address.clone()
    }

    fn get_private_key(&self) -> String {
        self.private_key.clone()
    }

    fn get_address_type(&self) -> String {
        let address_type = String::from("account");
        address_type
    }
}

impl AddressDataProvider for Delegation {
    fn get_address(&self) -> String {
        self.address.clone()
    }

    fn get_private_key(&self) -> String {
        self.private_key.clone()
    }

    fn get_address_type(&self) -> String {
        let address_type = String::from("utxo");
        address_type
    }
}
