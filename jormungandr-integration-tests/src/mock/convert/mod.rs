use crate::mock::proto::node::Block;
use chain_core::mempack::{ReadBuf, Readable};
use chain_impl_mockchain::block::Block as ChainBlock;

impl Into<ChainBlock> for Block {
    fn into(self) -> ChainBlock {
        let mut buf = ReadBuf::from(self.get_content());
        ChainBlock::read(&mut buf).expect("Cannot convert grpc Block type into mockchain Block")
    }
}

#[cfg(test)]
mod tests {
    extern crate hex;
    use crate::mock::proto::node::Block;
    use chain_impl_mockchain::block::Block as ChainBlock;

    #[test]
    pub fn convert_test() {
        let block_content = "02b6000200000000000079950000009700000acf0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8f9df7d506e596debfe3615938a6a8acc2ac754d7d7ef745a23355c7690251bfdec1edd225a8955f2f38ba2a7eac7d55c6b23401c45dd96e94ce9d95a987fb42022144f625c1944293daa6748eb225aae15498d0f1bcb90d5bd7b8e146c12532b6c71ef2d81a7e0b9865334165c654835420416312d5cb6eef0f77618611bf20459d9fc5ce08fc6df2dd207232b909d23acff0bdc23cc9191e040c5d232c127040000000006284f24fc69e4a6efdf5dbdd48b4b46cc58fa0b05f0a6d570188c510f55558e4940384f3517ff508da7f0d99f89a7119b9eb4b3d97c6ebfb4d305903b57770f42abf68d8309eaebef686debc84d7ef63c96978ea1200b958eb540ae7e9d1177297e77d28b0217f05c780b366e2c712befb2e497914b5e8d4cb9de7c032fbdef33b56b2162dd3866bb13550cf1090cace816148813f1c69321e23e841e22ea97b5c2e1ebfe4437626b713b629ce0590f51dae7114c801b343e22c40d4f168b8fee2051ac18807ff06f4f9d54c62a229d611ba0472b25a0ac013ab39cf723aca57e91203e22f220d4526b9d65ff3dfe38b1eaa24886d1a9df464a95cac29b315b3a742c3c45e5fefb90c7debf619f21dc043091325c15f6ae7f94fb543b6251213a620e38241bd4055771283613056b45053dc7d15647f5036289249a807362d189e54a84f748992a1de01937fac0a626e3f3f747895a01c5e09ea2af81d0513413ce77b9d8d043707f9e305c975d36a815f49c3d7ec6be0ad2fbc1d5842d01003590bba3b288c164471d0163be24a66f6dfca069f9d4775c4c4d48500d41c1044bed2175c29c57f16174600d045092766506fc160e89b540adca15a83ebe58b7e84abf78ba160b4bac52c868a733ef219037ed8de248a61aa09088e76a6fb8e5";
        let content = hex::decode(block_content).unwrap();

        let mut block = Block::new();
        block.set_content(content);
        let chain_block: ChainBlock = block.into();
        println!("{:?}", chain_block);
    }
}
