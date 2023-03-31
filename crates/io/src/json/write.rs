use graph::Models;

pub fn write(models: Models) -> Result<(), String> {
    let res = serde_json::to_string_pretty(&models);
    match res {
        Ok(json_str) => {
            println!("{json_str}");
            Ok(())
        }
        Err(why) => {
            log::error!("{:?}", why);
            Err(why.to_string())
        }
    }
}
