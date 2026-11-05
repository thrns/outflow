use outflow_core::text_cleanup::remove_leading_isolated_fillers;

const INPUT: &str = "um, so I visited north parkade.\nUh, I use Rust at UBC.\nI said um, not uh.";
const CLEAN_OUTPUT: &str = "so I visited north parkade.\nI use Rust at UBC.\nI said um, not uh.";

fn main() {
    let original_input = INPUT.to_owned();
    let clean_output = remove_leading_isolated_fillers(INPUT, true);
    let verbatim_output = remove_leading_isolated_fillers(INPUT, false);

    println!("Input: {INPUT:?}");
    println!("Clean: {clean_output:?}");
    println!("Verbatim: {verbatim_output:?}");

    assert_eq!(
        clean_output, CLEAN_OUTPUT,
        "Clean output differed from expected"
    );
    assert_eq!(
        verbatim_output, INPUT,
        "Verbatim output must preserve the input exactly"
    );
    assert_eq!(
        remove_leading_isolated_fillers(&clean_output, true),
        clean_output,
        "Clean output must be idempotent"
    );
    assert_eq!(
        INPUT, original_input,
        "the original input must remain unchanged"
    );
}
