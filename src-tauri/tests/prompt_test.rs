use pixtrace_lib::metadata::{ImageMetadata, TextChunk};
use pixtrace_lib::prompt::{self, ParseStatus};

fn meta(prompt: Option<&str>, workflow: Option<&str>, parameters: Option<&str>) -> ImageMetadata {
    let mut meta = ImageMetadata {
        width: Some(512),
        height: Some(512),
        ..ImageMetadata::default()
    };
    for (keyword, value) in [
        ("prompt", prompt),
        ("workflow", workflow),
        ("parameters", parameters),
    ] {
        if let Some(value) = value {
            meta.texts.push(TextChunk {
                keyword: keyword.to_string(),
                value: value.as_bytes().to_vec(),
            });
        }
    }
    meta
}

#[test]
fn extracts_sd15_api_prompt() {
    let json = r#"{
      "3": {"class_type": "KSampler", "inputs": {
        "seed": 12345, "steps": 28, "cfg": 7.0, "sampler_name": "euler",
        "scheduler": "normal", "denoise": 1.0,
        "model": ["4", 0], "positive": ["6", 0], "negative": ["7", 0],
        "latent_image": ["5", 0]}},
      "4": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "sd_v1-5.safetensors"}},
      "5": {"class_type": "EmptyLatentImage", "inputs": {"width": 512, "height": 768, "batch_size": 1}},
      "6": {"class_type": "CLIPTextEncode", "inputs": {"text": "masterpiece, (1girl:1.2)", "clip": ["4", 1]}},
      "7": {"class_type": "CLIPTextEncode", "inputs": {"text": "lowres, bad anatomy", "clip": ["4", 1]}}
    }"#;

    let extracted = prompt::extract(&meta(Some(json), None, None));
    assert_eq!(extracted.status, ParseStatus::Ok);
    assert_eq!(
        extracted.positive.as_deref(),
        Some("masterpiece, (1girl:1.2)")
    );
    assert_eq!(extracted.negative.as_deref(), Some("lowres, bad anatomy"));
    assert_eq!(extracted.model.as_deref(), Some("sd_v1-5.safetensors"));
    assert_eq!(extracted.sampler.as_deref(), Some("euler"));
    assert_eq!(extracted.steps, Some(28));
    assert_eq!(extracted.cfg, Some(7.0));
    assert_eq!(extracted.seed, Some(12345));
    assert_eq!(extracted.width, Some(512));
    assert_eq!(extracted.height, Some(768));
}

#[test]
fn collects_sdxl_global_and_local_text() {
    let json = r#"{
      "1": {"class_type": "KSampler", "inputs": {
        "seed": 7, "steps": 30, "cfg": 8, "sampler_name": "dpmpp_2m",
        "positive": ["4", 0], "negative": ["5", 0]}},
      "2": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "sdxl.safetensors"}},
      "4": {"class_type": "CLIPTextEncodeSDXL", "inputs": {
        "text_g": "global style", "text_l": "local detail",
        "width": 1024, "height": 1024}},
      "5": {"class_type": "CLIPTextEncode", "inputs": {"text": "blurry"}}
    }"#;

    let extracted = prompt::extract(&meta(Some(json), None, None));
    let positive = extracted.positive.unwrap();
    assert!(positive.contains("global style"));
    assert!(positive.contains("local detail"));
    assert_eq!(extracted.negative.as_deref(), Some("blurry"));
    assert_eq!(extracted.width, Some(1024));
    assert_eq!(extracted.height, Some(1024));
}

#[test]
fn excludes_nodes_shared_with_negative_branch() {
    let json = r#"{
      "1": {"class_type": "KSampler", "inputs": {"positive": ["2", 0], "negative": ["3", 0]}},
      "2": {"class_type": "ConditioningCombine", "inputs": {
        "conditioning_1": ["4", 0], "conditioning_2": ["5", 0]}},
      "3": {"class_type": "ConditioningCombine", "inputs": {
        "conditioning_1": ["5", 0], "conditioning_2": ["6", 0]}},
      "4": {"class_type": "CLIPTextEncode", "inputs": {"text": "positive only"}},
      "5": {"class_type": "CLIPTextEncode", "inputs": {"text": "shared negative"}},
      "6": {"class_type": "CLIPTextEncode", "inputs": {"text": "negative only"}}
    }"#;

    let extracted = prompt::extract(&meta(Some(json), None, None));
    assert_eq!(extracted.positive.as_deref(), Some("positive only"));
    let negative = extracted.negative.unwrap();
    assert!(negative.contains("shared negative"));
    assert!(negative.contains("negative only"));
    assert!(!negative.contains("positive only"));
}

#[test]
fn extracts_flux_workflow_through_guider() {
    let json = r#"{
      "1": {"class_type": "UNETLoader", "inputs": {"unet_name": "flux1-dev.safetensors"}},
      "2": {"class_type": "CLIPTextEncode", "inputs": {"text": "a cat on the moon"}},
      "3": {"class_type": "BasicGuider", "inputs": {"conditioning": ["9", 0]}},
      "4": {"class_type": "SamplerCustomAdvanced", "inputs": {"guider": ["3", 0], "latent_image": ["8", 0]}},
      "5": {"class_type": "RandomNoise", "inputs": {"noise_seed": 42}},
      "6": {"class_type": "KSamplerSelect", "inputs": {"sampler_name": "euler"}},
      "7": {"class_type": "BasicScheduler", "inputs": {"scheduler": "simple", "steps": 20}},
      "8": {"class_type": "EmptySD3LatentImage", "inputs": {"width": 1024, "height": 768}},
      "9": {"class_type": "FluxGuidance", "inputs": {"guidance": 3.5, "conditioning": ["2", 0]}}
    }"#;

    let extracted = prompt::extract(&meta(Some(json), None, None));
    assert_eq!(extracted.positive.as_deref(), Some("a cat on the moon"));
    assert_eq!(extracted.model.as_deref(), Some("flux1-dev.safetensors"));
    assert_eq!(extracted.sampler.as_deref(), Some("euler"));
    assert_eq!(extracted.steps, Some(20));
    assert_eq!(extracted.cfg, Some(3.5));
    assert_eq!(extracted.seed, Some(42));
    assert_eq!(extracted.width, Some(1024));
    assert_eq!(extracted.height, Some(768));
}

#[test]
fn extracts_loras_and_model() {
    let json = r#"{
      "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "base.safetensors"}},
      "2": {"class_type": "LoraLoader", "inputs": {"lora_name": "detail.safetensors", "strength_model": 0.8}},
      "3": {"class_type": "KSampler", "inputs": {"positive": ["4", 0]}},
      "4": {"class_type": "CLIPTextEncode", "inputs": {"text": "test"}}
    }"#;
    let extracted = prompt::extract(&meta(Some(json), None, None));
    assert_eq!(extracted.model.as_deref(), Some("base.safetensors"));
    assert_eq!(extracted.loras.len(), 1);
    assert_eq!(extracted.loras[0].name, "detail.safetensors");
    assert_eq!(extracted.loras[0].weight, 0.8);
}

#[test]
fn falls_back_to_workflow_ui_format() {
    let workflow = r#"{
      "nodes": [
        {"id": 1, "type": "CheckpointLoaderSimple", "widgets_values": ["sdxl.safetensors"], "inputs": []},
        {"id": 2, "type": "KSampler", "widgets_values": [42, "fixed", 30, 7.5, "dpmpp_2m", "karras", 1.0],
         "inputs": [{"name": "positive", "link": 10}, {"name": "negative", "link": 11}]},
        {"id": 3, "type": "CLIPTextEncode", "widgets_values": ["positive text"], "inputs": []},
        {"id": 4, "type": "CLIPTextEncode", "widgets_values": ["negative text"], "inputs": []},
        {"id": 5, "type": "EmptyLatentImage", "widgets_values": [1024, 1024, 1], "inputs": []}
      ],
      "links": [[10, 3, 0, 2, 1, "CONDITIONING"], [11, 4, 0, 2, 2, "CONDITIONING"]]
    }"#;

    let extracted = prompt::extract(&meta(None, Some(workflow), None));
    assert_eq!(extracted.positive.as_deref(), Some("positive text"));
    assert_eq!(extracted.negative.as_deref(), Some("negative text"));
    assert_eq!(extracted.model.as_deref(), Some("sdxl.safetensors"));
    assert_eq!(extracted.steps, Some(30));
    assert_eq!(extracted.cfg, Some(7.5));
    assert_eq!(extracted.seed, Some(42));
    assert_eq!(extracted.width, Some(1024));
}

#[test]
fn falls_back_to_a1111_parameters() {
    let parameters = "masterpiece, 1girl\nNegative prompt: lowres, bad hands\nSteps: 20, Sampler: Euler a, CFG scale: 7, Seed: 123, Size: 512x768, Model: anything-v5";
    let extracted = prompt::extract(&meta(None, None, Some(parameters)));
    assert_eq!(extracted.status, ParseStatus::Ok);
    assert_eq!(extracted.positive.as_deref(), Some("masterpiece, 1girl"));
    assert_eq!(extracted.negative.as_deref(), Some("lowres, bad hands"));
    assert_eq!(extracted.steps, Some(20));
    assert_eq!(extracted.sampler.as_deref(), Some("Euler a"));
    assert_eq!(extracted.cfg, Some(7.0));
    assert_eq!(extracted.seed, Some(123));
    assert_eq!(extracted.width, Some(512));
    assert_eq!(extracted.height, Some(768));
    assert_eq!(extracted.model.as_deref(), Some("anything-v5"));
}

#[test]
fn invalid_json_is_reported_without_panicking() {
    let extracted = prompt::extract(&meta(Some("{not json"), None, None));
    assert_eq!(extracted.status, ParseStatus::Error);
    assert!(extracted.error.is_some());
}

#[test]
fn png_without_text_is_no_meta() {
    let extracted = prompt::extract(&meta(None, None, None));
    assert_eq!(extracted.status, ParseStatus::NoMeta);
    assert_eq!(extracted.width, Some(512));
    assert_eq!(extracted.height, Some(512));
}

#[test]
fn normalizes_and_hashes_prompt() {
    let normalized = prompt::normalize_prompt("  Masterpiece,\n(1Girl:1.2)  \\(test\\) ");
    assert_eq!(normalized, "masterpiece, (1girl:1.2) (test)");
    let first = prompt::prompt_hash(&normalized);
    let second = prompt::prompt_hash(&prompt::normalize_prompt("masterpiece, (1girl:1.2) (test)"));
    assert_eq!(first, second);
    assert_eq!(first.len(), 64);
}

#[test]
fn builds_search_text_without_weights() {
    assert_eq!(
        prompt::search_text("Masterpiece, (1girl:1.2), <lora:detail:0.8>"),
        "masterpiece 1girl detail"
    );
}
