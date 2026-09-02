fn main(secret: Field, pub_inputs: pub [Field; 1000], sum: pub Field) {
    let mut acc: Field = secret;
    for i in 0..1000 {
        acc = acc + pub_inputs[i];
    }
    assert(acc == sum);
}
