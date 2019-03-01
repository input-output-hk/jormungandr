#[inline]
pub fn clone_array_and_insert_at_pos<A: Clone>(v: &Vec<A>, a: A, pos: usize) -> Vec<A> {
    // copy all elements but insert a new elements at position pos
    let mut new_array: Vec<A> = Vec::with_capacity(v.len() + 1);
    new_array.extend_from_slice(&v[0..pos]);
    new_array.push(a);
    new_array.extend_from_slice(&v[pos..]);
    new_array
}

#[inline]
pub fn clone_array_and_set_at_pos<A: Clone>(v: &Vec<A>, a: A, pos: usize) -> Vec<A> {
    // copy all elements except at pos where a replaces it.
    let mut new_array: Vec<A> = Vec::with_capacity(v.len());
    if pos > 0 {
        new_array.extend_from_slice(&v[0..pos]);
    }
    new_array.push(a);
    if pos + 1 < v.len() {
        new_array.extend_from_slice(&v[(pos + 1)..]);
    }
    new_array
}
