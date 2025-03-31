
#[test]
fn te() {
    let mut x = 1_i32;
    println!("{x}");
    x.saturating_sub(1);
    println!("{x}");
}
