// A circuit with enough constraints to push LOG_N well above the other
// fixtures, by chaining many field multiplications whose result is exposed
// as a public return value (so the ACIR optimizer can't eliminate it).
fn main(secret: Field) -> pub Field {
    let mut acc: Field = secret;
    for i in 0..400 {
        acc = acc * (secret + (i as Field)) + 1;
    }
    acc
}

#[test]
fn test_main() {
    assert(main(3) != 0);
}
