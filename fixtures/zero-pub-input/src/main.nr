fn main(secret: Field) {
    assert(secret != 0);
}

#[test]
fn test_main() {
    main(5);
}
