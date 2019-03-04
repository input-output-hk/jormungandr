pub fn zero(to_zero: &mut [u8]) {
    // the unsafety of this call is bounded to the existence of the pointer
    // and the accuracy of the length of the array.
    //
    // since to_zero existence is bound to live at least as long as the call
    // of this function and that we use the length (in bytes) of the given
    // slice, this call is safe.
    unsafe { ::std::ptr::write_bytes(to_zero.as_mut_ptr(), 0, to_zero.len()) }
}
