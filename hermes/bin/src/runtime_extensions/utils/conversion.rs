//! Conversion utilities.

/// Convert a 32 bytes array to a tuple of u64 values.
pub(crate) fn array_u8_32_to_tuple(array: &[u8; 32]) -> (u64, u64, u64, u64) {
    let mut tuple = (0u64, 0u64, 0u64, 0u64);
    let mut arr = [0u8; 8];
    let slice1 = &array[0..8];
    arr.copy_from_slice(slice1);
    tuple.0 = u64::from_be_bytes(arr);

    let slice2 = &array[8..16];
    arr.copy_from_slice(slice2);
    tuple.1 = u64::from_be_bytes(arr);

    let slice3 = &array[16..24];
    arr.copy_from_slice(slice3);
    tuple.2 = u64::from_be_bytes(arr);

    let slice4 = &array[24..32];
    arr.copy_from_slice(slice4);
    tuple.3 = u64::from_be_bytes(arr);

    tuple
}

/// Convert a 64 bytes array to a tuple of u64 values.
pub(crate) fn array_u8_64_to_tuple(array: &[u8; 64]) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
    let mut tuple = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut arr = [0u8; 8];
    let slice1 = &array[0..8];
    arr.copy_from_slice(slice1);
    tuple.0 = u64::from_be_bytes(arr);

    let slice2 = &array[8..16];
    arr.copy_from_slice(slice2);
    tuple.1 = u64::from_be_bytes(arr);

    let slice3 = &array[16..24];
    arr.copy_from_slice(slice3);
    tuple.2 = u64::from_be_bytes(arr);

    let slice4 = &array[24..32];
    arr.copy_from_slice(slice4);
    tuple.3 = u64::from_be_bytes(arr);

    let slice5 = &array[32..40];
    arr.copy_from_slice(slice5);
    tuple.4 = u64::from_be_bytes(arr);

    let slice6 = &array[40..48];
    arr.copy_from_slice(slice6);
    tuple.5 = u64::from_be_bytes(arr);

    let slice7 = &array[48..56];
    arr.copy_from_slice(slice7);
    tuple.6 = u64::from_be_bytes(arr);

    let slice8 = &array[56..64];
    arr.copy_from_slice(slice8);
    tuple.7 = u64::from_be_bytes(arr);

    tuple
}

/// Convert a tuple of u64 values to a 64 bytes array.
pub(crate) fn b512_u64_tuple_to_u8_array(
    tuple: &(u64, u64, u64, u64, u64, u64, u64, u64),
) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let (t1, t2, t3, t4, t5, t6, t7, t8) = tuple;
    bytes[0..8].copy_from_slice(&t1.to_be_bytes());
    bytes[8..16].copy_from_slice(&t2.to_be_bytes());
    bytes[16..24].copy_from_slice(&t3.to_be_bytes());
    bytes[24..32].copy_from_slice(&t4.to_be_bytes());
    bytes[32..40].copy_from_slice(&t5.to_be_bytes());
    bytes[40..48].copy_from_slice(&t6.to_be_bytes());
    bytes[48..56].copy_from_slice(&t7.to_be_bytes());
    bytes[56..64].copy_from_slice(&t8.to_be_bytes());
    bytes
}
