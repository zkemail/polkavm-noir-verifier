fn main(secret: Field) -> pub Field {
    let mut acc: Field = secret;
    for i in 0..550000 {
        acc = acc * (secret + (i as Field)) + 1;
    }
    acc
}
