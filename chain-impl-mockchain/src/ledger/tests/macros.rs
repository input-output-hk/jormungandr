
// assert_err( ExpectedErrorExpression , Expression )
//
// succeed if Expression's value returns a value equal to Err(ExpectedErrorExpression),
// otherwise panic!() with some diagnostic
#[allow(unused_macros)]
macro_rules! assert_err {
    ($left: expr, $right: expr) => {
        match &($left) {
            left_val => match &($right) {
                Err(e) => {
                    if !(e == left_val) {
                        panic!(
                            "assertion failed: error mismatch \
                             (left: `{:?}, right: `{:?}`)",
                            *left_val, *e
                        )
                    }
                }
                Ok(_) => panic!(
                    "assertion failed: expected error {:?} but got success",
                    *left_val
                ),
            },
        }
    };
}

// assert_err_match( ExpectedErrorPattern , Expression )
//
// succeed if Expression's value a Err(E) where E match the ExpectedErrorPattern,
// otherwise panic!() with some diagnostic
macro_rules! assert_err_match {
    ($left: pat, $right: expr) => {
        match &($right) {
            Err(e) => {
                match e {
                    $left => {},
                    _ => panic!("assertion failed: error mismatch got: `{:?}` but expecting {}", *e, stringify!($left))
                }
            }
            Ok(_) => panic!("assertion failed: expected error {:?} but got success", stringify!($left)),
        }
    };
}
