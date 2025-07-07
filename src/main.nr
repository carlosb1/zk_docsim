 fn main(a: [i32; 8], b: [i32; 8], threshold: i32) -> pub bool {
    let mut sum = 0;
    for i in 0..8 {
        let diff = a[i] - b[i];
        sum += diff * diff;
    }
    sum < threshold
}