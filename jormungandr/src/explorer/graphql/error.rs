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
    }
}
