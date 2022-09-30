use multiaddr::Multiaddr;

pub trait MultiaddrExtension {
    fn to_http_addr(self) -> String;
}

impl MultiaddrExtension for Multiaddr {
    fn to_http_addr(mut self) -> String {
        let port = match self.pop().unwrap() {
            multiaddr::Protocol::Tcp(port) => port,
            _ => todo!("explorer can only be attached through grpc(http)"),
        };

        let address = match self.pop().unwrap() {
            multiaddr::Protocol::Ip4(address) => address,
            _ => todo!("only ipv4 supported for now"),
        };
        format!("http://{}:{}/", address, port)
    }
}
