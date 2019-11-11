error_chain! {
    errors {
        InternalError(msg: String) {
            description("an error that shouldn't happen"),
            display("{}", msg)
        }
        NotFound(msg: String) {
            description("resource not found"),
            display("{}", msg)
        }
        Unimplemented {
            description("feature not implemented yet"),
            display("unimplemented")
        }
        ArgumentError(msg: String) {
            description("invalid argument in query"),
            display("invalid argument: {}", msg)
        }
        InvalidCursor(msg: String) {
            description("invalid cursor in pagination query"),
            display("invalid cursor in pagination query: {}", msg)
        }
        InvalidAddress(address: String) {
            description("failed to parse address"),
            display("invalid address: {}", address)
        }
    }
}
