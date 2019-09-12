error_chain! {
    errors {
        InternalError(msg: String) {
            description("an error that shouldn't happen"),
            display("{}", msg)
        }
    }
}
