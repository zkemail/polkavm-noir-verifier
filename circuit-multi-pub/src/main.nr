fn main(secret: Field, pub_a: pub Field, pub_b: pub Field, pub_c: pub Field, pub_sum: pub Field) {
    // Prove we know a secret that, added to the three public values, equals the public sum
    assert(secret + pub_a + pub_b + pub_c == pub_sum);
}
