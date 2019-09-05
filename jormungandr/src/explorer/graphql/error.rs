error_chain! {
    //foreign_links {
    //    StorageError(StorageError);
    //}
    errors {
        InternalError(msg: String) {
            description("Internal error: Shouldn't happen"),
            display("Internal error: {}", msg)
        }
    }
}
