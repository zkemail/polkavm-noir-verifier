fn main(secret: Field, a: pub Field, b: pub Field, c: pub Field, d: pub Field, sum: pub Field) {
    assert(secret + a + b + c + d == sum);
}

#[test]
fn test_main() {
    main(1, 2, 3, 4, 5, 15);
}
