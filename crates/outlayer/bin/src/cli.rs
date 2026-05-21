use serde_json::Value;

use super::AppConfig;

pub fn print_env_vars() {
    let defaults = serde_json::to_value(AppConfig::default()).unwrap();
    print_value("OUTLAYER", &defaults);
}

fn print_value(prefix: &str, value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                print_value(&format!("{prefix}__{}", key.to_uppercase()), val);
            }
        }
        Value::Null => println!("# {prefix}="),
        other => println!("{prefix}={other}"),
    }
}
