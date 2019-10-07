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
    pub spending_key: u64,
}

impl Account {
    pub fn new(private_key: &str, public_key: &str, address: &str) -> Self {
        Account {
            private_key: private_key.to_string(),
            public_key: public_key.to_string(),
            address: address.to_string(),
            spending_key: 0u64,
        }
    }

    pub fn confirm_transaction(&mut self) {
        self.spending_key = self.spending_key + 1;
    }
}

#[derive(Debug, Clone)]
pub struct Delegation {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
    pub delegation_key: String,
}

pub trait AddressDataProvider {
    fn get_address(&self) -> String;
    fn get_private_key(&self) -> String;
    fn get_address_type(&self) -> String;
    fn get_spending_key(&self) -> Option<u64>;
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

    fn get_spending_key(&self) -> Option<u64> {
        None
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

    fn get_spending_key(&self) -> Option<u64> {
        Some(self.spending_key)
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

    fn get_spending_key(&self) -> Option<u64> {
        None
    }
}
