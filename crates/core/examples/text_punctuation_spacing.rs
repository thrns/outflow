use outflow_core::text_cleanup::normalize_punctuation_spacing;

fn main() {
    let input = "Hello , world !\nI visited north parkade .\nThe café , 東京 tea 😀 .\nVersion 3 . 14 remains unchanged.";
    let expected = "Hello, world!\nI visited north parkade.\nThe café, 東京 tea 😀 .\nVersion 3 . 14 remains unchanged.";
    let output = normalize_punctuation_spacing(input);

    println!("Before: {input:?}");
    println!("After: {output:?}");

    assert_eq!(output, expected);
    assert_eq!(normalize_punctuation_spacing(&output), output);
}
