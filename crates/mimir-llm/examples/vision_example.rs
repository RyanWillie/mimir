//! Working vision model example using MistralRS
//! 
//! This example demonstrates how to properly use vision models with MistralRS.
//! Note: This requires image dependency to be enabled in Cargo.toml

use anyhow::Result;
use image;
use mistralrs::{AutoDeviceMapParams, DeviceMapSetting, IsqType, RequestBuilder, TextMessageRole, VisionMessages, VisionModelBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    // Use a working vision model (not UQFF format)
    let auto_map = AutoDeviceMapParams::default_vision();
    let model = VisionModelBuilder::new("microsoft/Phi-3.5-vision-instruct")
        .with_isq(IsqType::Q4K)
        .with_logging()
        .with_device_mapping(DeviceMapSetting::Auto(auto_map))
        .build()
        .await?;

    // Create a simple test image (1×1 black pixel)
    let image = image::DynamicImage::new_rgb8(1, 1);

    // Build VisionMessages with image + text
    let messages = VisionMessages::new().add_image_message(
        TextMessageRole::User,
        "What do you see in this image?",
        vec![image],
        &model,
    )?;
    
    println!("{:?}", messages);
    
    let request = RequestBuilder::from(messages)
        .set_sampler_max_len(50);
    
    println!("Sending request to vision model...");
    let response = model.send_chat_request(request).await?;
    println!("Response: {}", response.choices[0].message.content.as_ref().unwrap());
    
    Ok(())
} 