use std::error::Error;

use outflow_core::vocabulary_post_correction::apply_vocabulary_post_correction;

fn main() -> Result<(), Box<dyn Error>> {
    let input = "I visited north PARKADE today.\nThe cafe\u{301} serves 東京 tea 😀.";
    let corrected = apply_vocabulary_post_correction(input, "north parkade", "North Parkade")?;

    println!("Input: {input}");
    println!("Output: {corrected}");
    println!(
        "Unrelated Unicode preserved: {}",
        corrected.contains("cafe\u{301} serves 東京 tea 😀.")
    );
    Ok(())
}
