use anyhow::Result;
use mistralrs::{AutoDeviceMapParams, DeviceMapSetting, IsqType, TextMessageRole, TextMessages, VisionModelBuilder};

#[tokio::main]
 async fn main() -> Result<()> {
    let auto_map = AutoDeviceMapParams::default_vision();
    let model = VisionModelBuilder::new("google/gemma-3-1b-it")
        .with_device_mapping(DeviceMapSetting::Auto(auto_map))
        .build()
        .await?;

     let messages = TextMessages::new()
         .add_message(TextMessageRole::User, "Pick a random number between 1 and 100");

     let response = model.send_chat_request(messages).await?;
     println!("{}", response.choices[0].message.content.as_ref().unwrap());
     Ok(())
 }