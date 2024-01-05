use accumulo_access::check_authorization;

// Used as a test fixture when RustRover doesn't recognize the #[rstest] macro.
fn main() {
    let expression = "(label1 | label5)";
    let tokens = &Vec::from([
        String::from("label1"),
    ]);

    match check_authorization(expression, tokens) {
        Ok(result) => {
            assert_eq!(result, true);
        }
        Err(e) => println!("Unexpected error {}", e),
    };
}