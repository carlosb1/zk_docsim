use risc0_zkvm::guest::env;


const THRESHOLD: f32 = 0.8;
fn main() {
    let input: (Vec<f32>, Vec<f32>) = env::read();

    let dot = input.0.iter().zip(input.1.iter()).map(|(a, b)| a * b).sum::<f32>();
    let norm_a = input.0.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = input.1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let similarity = dot / (norm_a * norm_b);

    let result = similarity > THRESHOLD;
    // write public output to the journal
    env::commit(&(result, similarity));
}
