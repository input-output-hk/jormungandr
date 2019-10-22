pub trait Payload {
    const HAS_DATA : bool;
    const HAS_AUTH : bool;
    type Auth;
}
