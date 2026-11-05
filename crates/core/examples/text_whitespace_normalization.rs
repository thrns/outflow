use outflow_core::text_cleanup::normalize_transcript_whitespace;

fn main() {
    let input = "  I visited   north parkade.\r\n\r\n\tI use rust\t at  ubc.  ";
    let expected = "I visited north parkade.\n\nI use rust at ubc.";
    let actual = normalize_transcript_whitespace(input);
    let idempotent = normalize_transcript_whitespace(&actual) == actual;

    println!("Before: {input:?}");
    println!("After: {actual:?}");
    println!("Idempotent: {idempotent}");

    assert_eq!(actual, expected, "normalized output differed from expected");
    assert!(idempotent, "normalization was not idempotent");
}
