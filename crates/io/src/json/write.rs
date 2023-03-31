use graph::Models;

pub fn write(models: Models) {
    let json_str = serde_json::to_string_pretty(&models).unwrap();
    println!("{json_str}");
}
