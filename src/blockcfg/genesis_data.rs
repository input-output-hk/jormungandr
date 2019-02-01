pub struct GenesisData {

}

impl GenesisData {
    pub fn parse<R: std::io::BufRead>(reader: R) -> Result<Self, impl std::error::Error> {
        Ok(GenesisData {})
    }
}
