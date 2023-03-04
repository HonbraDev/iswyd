use serde::Serialize;

#[allow(dead_code)]
fn serde_print<T>(value: &T)
where
    T: Serialize,
{
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(e) => println!("Failed to serialize for print: {e}"),
    }
}
