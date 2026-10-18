#[test]
fn all_code_examples_valid() {
    // internally, calls all codeblock validations → errors if invalid.
    super::SherlockDocumentation::render();
}
