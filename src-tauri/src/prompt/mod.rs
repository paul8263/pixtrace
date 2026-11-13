use std::collections::{HashMap, HashSet, VecDeque};

use serde_json::{Map, Value};
use unicode_normalization::UnicodeNormalization;

use crate::metadata::ImageMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParseStatus {
    Ok,
    Partial,
    #[default]
    NoMeta,
    Error,
}

impl ParseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ParseStatus::Ok => "ok",
            ParseStatus::Partial => "partial",
            ParseStatus::NoMeta => "no_meta",
            ParseStatus::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lora {
    pub name: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Extracted {
    pub positive: Option<String>,
    pub negative: Option<String>,
    pub model: Option<String>,
    pub loras: Vec<Lora>,
    pub sampler: Option<String>,
    pub steps: Option<i64>,
    pub cfg: Option<f64>,
    pub seed: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub status: ParseStatus,
    pub error: Option<String>,
}

impl Extracted {
    pub fn loras_text(&self) -> Option<String> {
        if self.loras.is_empty() {
            return None;
        }
        Some(
            self.loras
                .iter()
                .map(|lora| format!("{}:{}", lora.name, lora.weight))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

const SAMPLER_TYPES: &[&str] = &[
    "KSampler",
    "KSamplerAdvanced",
    "SamplerCustom",
    "SamplerCustomAdvanced",
];

const TEXT_FIELDS: &[&str] = &[
    "text", "text_g", "text_l", "prompt", "positive", "string", "value",
];

const MAX_JSON_BYTES: usize = 32 * 1024 * 1024;

pub const PARSER_VERSION: &str = "1";

pub fn extract(meta: &ImageMetadata) -> Extracted {
    let prompt_raw = meta.text("prompt").and_then(bytes_to_string);
    let workflow_raw = meta.text("workflow").and_then(bytes_to_string);
    let parameters_raw = meta.text("parameters").and_then(bytes_to_string);

    if prompt_raw.is_none() && workflow_raw.is_none() && parameters_raw.is_none() {
        return Extracted {
            width: meta.width,
            height: meta.height,
            status: ParseStatus::NoMeta,
            ..Extracted::default()
        };
    }

    let mut errors = Vec::new();
    let mut result = Extracted::default();

    if let Some(raw) = prompt_raw.as_deref() {
        match parse_json(raw) {
            Ok(json) => {
                let extracted = extract_api(&json);
                if extracted.positive.is_some() || extracted.negative.is_some() {
                    result = extracted;
                } else {
                    errors.push("API prompt 中未找到文本节点".to_string());
                }
            }
            Err(error) => errors.push(format!("API prompt JSON 解析失败: {error}")),
        }
    }

    if result.positive.is_none() && result.negative.is_none() {
        if let Some(raw) = workflow_raw.as_deref() {
            match parse_json(raw) {
                Ok(json) => {
                    let extracted = extract_workflow(&json);
                    if extracted.positive.is_some() || extracted.negative.is_some() {
                        result = extracted;
                    } else {
                        errors.push("workflow 中未找到文本节点".to_string());
                    }
                }
                Err(error) => errors.push(format!("workflow JSON 解析失败: {error}")),
            }
        }
    }

    if result.positive.is_none() && result.negative.is_none() {
        if let Some(raw) = parameters_raw.as_deref() {
            result = extract_a1111(raw);
        }
    }

    if result.positive.is_none() && result.negative.is_none() {
        result.status = if errors.is_empty() {
            ParseStatus::NoMeta
        } else {
            ParseStatus::Error
        };
        result.error = if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        };
    }

    if result.width.is_none() {
        result.width = meta.width;
    }
    if result.height.is_none() {
        result.height = meta.height;
    }
    result
}

fn parse_json(raw: &str) -> Result<Value, String> {
    if raw.len() > MAX_JSON_BYTES {
        return Err(format!("JSON 超过大小限制 ({} 字节)", raw.len()));
    }
    serde_json::from_str(raw).map_err(|error| error.to_string())
}

fn bytes_to_string(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes).ok().map(ToOwned::to_owned)
}

fn extract_api(root: &Value) -> Extracted {
    let Some(nodes) = root.as_object() else {
        return Extracted::default();
    };
    let mut result = Extracted::default();
    let samplers = sorted_samplers(nodes);

    if samplers.is_empty() {
        result.positive = collect_all_text_nodes(nodes);
        result.status = if result.positive.is_some() {
            ParseStatus::Partial
        } else {
            ParseStatus::NoMeta
        };
        extract_api_params(nodes, samplers.first().copied(), &mut result);
        return result;
    }

    let mut positive_seeds: Vec<&Value> = Vec::new();
    let mut negative_seeds: Vec<&Value> = Vec::new();
    for (_, sampler) in &samplers {
        let Some(inputs) = node_inputs(sampler) else {
            continue;
        };
        for key in ["positive", "conditioning"] {
            if let Some(value) = inputs.get(key) {
                positive_seeds.push(value);
            }
        }
        if let Some(guider) = inputs.get("guider").and_then(link_id) {
            if let Some(guider_node) = nodes.get(&guider) {
                if let Some(conditioning) =
                    node_inputs(guider_node).and_then(|inputs| inputs.get("conditioning"))
                {
                    positive_seeds.push(conditioning);
                }
            }
        }
        if let Some(value) = inputs.get("negative") {
            negative_seeds.push(value);
        }
    }

    let positive_order = traversal_order(&positive_seeds, nodes);
    let negative_order = traversal_order(&negative_seeds, nodes);
    let positive_set: HashSet<String> = positive_order.iter().cloned().collect();
    let negative_set: HashSet<String> = negative_order.iter().cloned().collect();
    let allowed: HashSet<String> = positive_set.difference(&negative_set).cloned().collect();

    result.positive = texts_from_nodes(&positive_order, &allowed, nodes);
    result.negative = texts_from_nodes(&negative_order, &negative_set, nodes);
    result.status = if result.positive.is_some() || result.negative.is_some() {
        ParseStatus::Ok
    } else {
        ParseStatus::Partial
    };

    extract_api_params(nodes, samplers.first().copied(), &mut result);
    result
}

fn sorted_samplers(nodes: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut samplers: Vec<(&String, &Value)> = nodes
        .iter()
        .filter(|(_, node)| class_type(node).is_some_and(|kind| SAMPLER_TYPES.contains(&kind)))
        .collect();
    samplers.sort_by_key(|(id, _)| id.parse::<i64>().unwrap_or(i64::MAX));
    samplers
}

fn class_type(node: &Value) -> Option<&str> {
    node.get("class_type").and_then(Value::as_str)
}

fn node_inputs(node: &Value) -> Option<&Map<String, Value>> {
    node.get("inputs").and_then(Value::as_object)
}

fn link_id(value: &Value) -> Option<String> {
    let array = value.as_array()?;
    if array.len() != 2 {
        return None;
    }
    match &array[0] {
        Value::String(id) => Some(id.clone()),
        Value::Number(id) => Some(id.to_string()),
        _ => None,
    }
}

fn traversal_order(seeds: &[&Value], nodes: &Map<String, Value>) -> Vec<String> {
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    for seed in seeds {
        if let Some(id) = link_id(seed) {
            if seen.insert(id.clone()) {
                queue.push_back(id);
            }
        }
    }
    let mut order = Vec::new();
    while let Some(id) = queue.pop_front() {
        order.push(id.clone());
        let Some(node) = nodes.get(&id) else {
            continue;
        };
        let Some(inputs) = node_inputs(node) else {
            continue;
        };
        for value in inputs.values() {
            if let Some(next) = link_id(value) {
                if seen.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }
    }
    order
}

fn texts_from_nodes(
    order: &[String],
    allowed: &HashSet<String>,
    nodes: &Map<String, Value>,
) -> Option<String> {
    let mut texts: Vec<String> = Vec::new();
    for id in order {
        if !allowed.contains(id) {
            continue;
        }
        let Some(inputs) = nodes.get(id).and_then(node_inputs) else {
            continue;
        };
        for field in TEXT_FIELDS {
            if let Some(value) = inputs.get(*field).and_then(Value::as_str) {
                let value = value.trim();
                if !value.is_empty() && !texts.iter().any(|existing| existing == value) {
                    texts.push(value.to_string());
                }
            }
        }
    }
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

fn collect_all_text_nodes(nodes: &Map<String, Value>) -> Option<String> {
    let mut texts: Vec<String> = Vec::new();
    for node in nodes.values() {
        let Some(inputs) = node_inputs(node) else {
            continue;
        };
        for field in TEXT_FIELDS {
            if let Some(value) = inputs.get(*field).and_then(Value::as_str) {
                let value = value.trim();
                if !value.is_empty() && !texts.iter().any(|existing| existing == value) {
                    texts.push(value.to_string());
                }
            }
        }
    }
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

fn extract_api_params(
    nodes: &Map<String, Value>,
    main_sampler: Option<(&String, &Value)>,
    result: &mut Extracted,
) {
    if let Some((_, sampler)) = main_sampler {
        if let Some(inputs) = node_inputs(sampler) {
            result.sampler = inputs
                .get("sampler_name")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            result.steps = inputs.get("steps").and_then(Value::as_i64);
            result.cfg = inputs.get("cfg").and_then(Value::as_f64);
            result.seed = inputs
                .get("seed")
                .or_else(|| inputs.get("noise_seed"))
                .and_then(Value::as_i64);
            if result.width.is_none() {
                if let Some(latent) = inputs
                    .get("latent_image")
                    .and_then(link_id)
                    .and_then(|id| nodes.get(&id))
                {
                    result.width = input_u32(latent, "width");
                    result.height = input_u32(latent, "height");
                }
            }
        }
    }

    for node in nodes.values() {
        match class_type(node) {
            Some("CheckpointLoaderSimple") => {
                if result.model.is_none() {
                    result.model = input_str(node, "ckpt_name");
                }
            }
            Some("UNETLoader") | Some("UnetLoaderGGUF") => {
                if result.model.is_none() {
                    result.model = input_str(node, "unet_name");
                }
            }
            Some("LoraLoader") | Some("LoraLoaderModelOnly") => {
                if let Some(name) = input_str(node, "lora_name") {
                    if !result.loras.iter().any(|lora| lora.name == name) {
                        let weight = input_f64(node, "strength_model").unwrap_or(1.0);
                        result.loras.push(Lora { name, weight });
                    }
                }
            }
            Some("BasicScheduler") => {
                if result.steps.is_none() {
                    result.steps = input_i64(node, "steps");
                }
                if result.sampler.is_none() {
                    result.sampler = input_str(node, "scheduler");
                }
            }
            Some("KSamplerSelect") => {
                if result.sampler.is_none() {
                    result.sampler = input_str(node, "sampler_name");
                }
            }
            Some("EmptyLatentImage") | Some("EmptySD3LatentImage") => {
                if result.width.is_none() {
                    result.width = input_u32(node, "width");
                    result.height = input_u32(node, "height");
                }
            }
            Some("CLIPTextEncodeSDXL") => {
                if result.width.is_none() {
                    result.width = input_u32(node, "width");
                    result.height = input_u32(node, "height");
                }
            }
            Some("FluxGuidance") => {
                if result.cfg.is_none() {
                    result.cfg = input_f64(node, "guidance");
                }
            }
            Some("RandomNoise") if result.seed.is_none() => {
                result.seed = input_i64(node, "noise_seed");
            }
            _ => {}
        }
    }

    if result.steps.is_none() {
        if let Some(inputs) = main_sampler.and_then(|(_, node)| node_inputs(node)) {
            result.steps = inputs.get("steps").and_then(Value::as_i64);
        }
    }
}

fn input_str(node: &Value, key: &str) -> Option<String> {
    node_inputs(node)?.get(key)?.as_str().map(ToOwned::to_owned)
}

fn input_i64(node: &Value, key: &str) -> Option<i64> {
    node_inputs(node)?.get(key)?.as_i64()
}

fn input_f64(node: &Value, key: &str) -> Option<f64> {
    node_inputs(node)?.get(key)?.as_f64()
}

fn input_u32(node: &Value, key: &str) -> Option<u32> {
    let value = node_inputs(node)?.get(key)?.as_f64()?;
    if value.is_finite() && value >= 0.0 {
        Some(value as u32)
    } else {
        None
    }
}

fn extract_workflow(root: &Value) -> Extracted {
    let Some(nodes) = root.get("nodes").and_then(Value::as_array) else {
        return Extracted::default();
    };

    let mut by_id: HashMap<i64, &Value> = HashMap::new();
    for node in nodes {
        if let Some(id) = node.get("id").and_then(Value::as_i64) {
            by_id.insert(id, node);
        }
    }

    let mut links: HashMap<i64, i64> = HashMap::new();
    if let Some(link_array) = root.get("links").and_then(Value::as_array) {
        for link in link_array {
            if let Some(items) = link.as_array() {
                if let (Some(link_id), Some(origin)) = (
                    items.first().and_then(Value::as_i64),
                    items.get(1).and_then(Value::as_i64),
                ) {
                    links.insert(link_id, origin);
                }
            }
        }
    }

    let mut result = Extracted::default();
    let mut samplers: Vec<(i64, &Value)> = nodes
        .iter()
        .filter_map(|node| {
            let id = node.get("id").and_then(Value::as_i64)?;
            let kind = workflow_type(node)?;
            SAMPLER_TYPES.contains(&kind).then_some((id, node))
        })
        .collect();
    samplers.sort_by_key(|(id, _)| *id);

    if samplers.is_empty() {
        let all_nodes: Vec<&Value> = by_id.values().copied().collect();
        result.positive = collect_workflow_texts(&all_nodes);
        result.status = if result.positive.is_some() {
            ParseStatus::Partial
        } else {
            ParseStatus::NoMeta
        };
        extract_workflow_params(nodes, None, &mut result);
        return result;
    }

    let mut positive_seeds = Vec::new();
    let mut negative_seeds = Vec::new();
    for (_, sampler) in &samplers {
        for name in ["positive", "conditioning"] {
            if let Some(link) = workflow_input_link(sampler, name) {
                if let Some(origin) = links.get(&link) {
                    positive_seeds.push(*origin);
                }
            }
        }
        if let Some(link) = workflow_input_link(sampler, "guider") {
            if let Some(guider) = links.get(&link).and_then(|id| by_id.get(id)) {
                if let Some(link) = workflow_input_link(guider, "conditioning") {
                    if let Some(origin) = links.get(&link) {
                        positive_seeds.push(*origin);
                    }
                }
            }
        }
        if let Some(link) = workflow_input_link(sampler, "negative") {
            if let Some(origin) = links.get(&link) {
                negative_seeds.push(*origin);
            }
        }
    }

    let positive_order = workflow_traversal(&positive_seeds, &by_id, &links);
    let negative_order = workflow_traversal(&negative_seeds, &by_id, &links);
    let negative_set: HashSet<i64> = negative_order.iter().copied().collect();
    let positive_set: HashSet<i64> = positive_order.iter().copied().collect();

    result.positive =
        workflow_texts_from_order(&positive_order, &positive_set, &negative_set, &by_id);
    result.negative =
        workflow_texts_from_order(&negative_order, &negative_set, &positive_set, &by_id);
    result.status = if result.positive.is_some() || result.negative.is_some() {
        ParseStatus::Ok
    } else {
        ParseStatus::Partial
    };

    extract_workflow_params(nodes, samplers.first().copied(), &mut result);
    result
}

fn workflow_type(node: &Value) -> Option<&str> {
    node.get("type").and_then(Value::as_str)
}

fn workflow_input_link(node: &Value, name: &str) -> Option<i64> {
    node.get("inputs")?
        .as_array()?
        .iter()
        .find(|input| input.get("name").and_then(Value::as_str) == Some(name))?
        .get("link")?
        .as_i64()
}

fn workflow_traversal(
    seeds: &[i64],
    by_id: &HashMap<i64, &Value>,
    links: &HashMap<i64, i64>,
) -> Vec<i64> {
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    for seed in seeds {
        if seen.insert(*seed) {
            queue.push_back(*seed);
        }
    }
    let mut order = Vec::new();
    while let Some(id) = queue.pop_front() {
        order.push(id);
        let Some(node) = by_id.get(&id) else {
            continue;
        };
        if let Some(inputs) = node.get("inputs").and_then(Value::as_array) {
            for input in inputs {
                if let Some(link) = input.get("link").and_then(Value::as_i64) {
                    if let Some(origin) = links.get(&link) {
                        if seen.insert(*origin) {
                            queue.push_back(*origin);
                        }
                    }
                }
            }
        }
    }
    order
}

fn is_workflow_text_node(node: &Value) -> bool {
    workflow_type(node).is_some_and(|kind| {
        kind.contains("TextEncode") || kind.contains("CLIPText") || kind.contains("Prompt")
    })
}

fn workflow_texts(node: &Value) -> Vec<String> {
    let Some(values) = node.get("widgets_values") else {
        return Vec::new();
    };
    let mut texts = Vec::new();
    match values {
        Value::Array(items) => {
            for item in items {
                if let Some(text) = item.as_str() {
                    let text = text.trim();
                    if !text.is_empty() {
                        texts.push(text.to_string());
                    }
                }
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                if let Some(text) = item.as_str() {
                    let text = text.trim();
                    if !text.is_empty() {
                        texts.push(text.to_string());
                    }
                }
            }
        }
        _ => {}
    }
    texts
}

fn collect_workflow_texts(nodes: &[&Value]) -> Option<String> {
    let mut texts = Vec::new();
    for node in nodes {
        if !is_workflow_text_node(node) {
            continue;
        }
        for text in workflow_texts(node) {
            if !texts.contains(&text) {
                texts.push(text);
            }
        }
    }
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

fn workflow_texts_from_order(
    order: &[i64],
    allowed: &HashSet<i64>,
    excluded: &HashSet<i64>,
    by_id: &HashMap<i64, &Value>,
) -> Option<String> {
    let mut texts = Vec::new();
    for id in order {
        if !allowed.contains(id) || excluded.contains(id) {
            continue;
        }
        let Some(node) = by_id.get(id) else {
            continue;
        };
        if !is_workflow_text_node(node) {
            continue;
        }
        for text in workflow_texts(node) {
            if !texts.contains(&text) {
                texts.push(text);
            }
        }
    }
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

fn workflow_widgets(node: &Value) -> &[Value] {
    node.get("widgets_values")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn widget_str(node: &Value, index: usize) -> Option<String> {
    workflow_widgets(node)
        .get(index)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn widget_i64(node: &Value, index: usize) -> Option<i64> {
    let value = workflow_widgets(node).get(index)?;
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|value| value as i64))
}

fn widget_f64(node: &Value, index: usize) -> Option<f64> {
    workflow_widgets(node).get(index)?.as_f64()
}

fn extract_workflow_params(
    nodes: &[Value],
    main_sampler: Option<(i64, &Value)>,
    result: &mut Extracted,
) {
    if let Some((_, sampler)) = main_sampler {
        result.sampler = widget_str(sampler, 4);
        result.steps = widget_i64(sampler, 2);
        result.cfg = widget_f64(sampler, 3);
        result.seed = widget_i64(sampler, 0);
    }

    for node in nodes {
        match workflow_type(node) {
            Some("CheckpointLoaderSimple") => {
                if result.model.is_none() {
                    result.model = widget_str(node, 0);
                }
            }
            Some("UNETLoader") | Some("UnetLoaderGGUF") => {
                if result.model.is_none() {
                    result.model = widget_str(node, 0);
                }
            }
            Some("LoraLoader") | Some("LoraLoaderModelOnly") => {
                if let Some(name) = widget_str(node, 0) {
                    if !result.loras.iter().any(|lora| lora.name == name) {
                        let weight = widget_f64(node, 1).unwrap_or(1.0);
                        result.loras.push(Lora { name, weight });
                    }
                }
            }
            Some("BasicScheduler") => {
                if result.steps.is_none() {
                    result.steps = widget_i64(node, 1);
                }
                if result.sampler.is_none() {
                    result.sampler = widget_str(node, 0);
                }
            }
            Some("KSamplerSelect") => {
                if result.sampler.is_none() {
                    result.sampler = widget_str(node, 0);
                }
            }
            Some("EmptyLatentImage") | Some("EmptySD3LatentImage") => {
                if result.width.is_none() {
                    result.width = widget_i64(node, 0).map(|value| value as u32);
                    result.height = widget_i64(node, 1).map(|value| value as u32);
                }
            }
            Some("FluxGuidance") => {
                if result.cfg.is_none() {
                    result.cfg = widget_f64(node, 0);
                }
            }
            Some("RandomNoise") => {
                if result.seed.is_none() {
                    result.seed = widget_i64(node, 0);
                }
            }
            Some("KSamplerAdvanced") if result.seed.is_none() => {
                result.seed = widget_i64(node, 1);
            }
            _ => {}
        }
    }
}

fn split_a1111_params(raw: &str) -> (&str, Option<&str>) {
    let trimmed = raw.trim_end();
    if let Some(index) = trimmed.rfind("\nSteps:") {
        (&trimmed[..index], Some(trimmed[index + 1..].trim()))
    } else if trimmed.starts_with("Steps:") {
        ("", Some(trimmed))
    } else {
        (trimmed, None)
    }
}

fn extract_a1111(raw: &str) -> Extracted {
    let mut result = Extracted::default();
    let (body, params) = split_a1111_params(raw);

    let (positive, negative) = match body.find("Negative prompt:") {
        Some(index) => (
            body[..index].trim(),
            body[index + "Negative prompt:".len()..].trim(),
        ),
        None => (body.trim(), ""),
    };

    result.positive = Some(positive.to_string()).filter(|value| !value.is_empty());
    result.negative = Some(negative.to_string()).filter(|value| !value.is_empty());

    if let Some(params) = params {
        for pair in params.split(',') {
            let Some((key, value)) = pair.split_once(':') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "steps" => result.steps = value.parse().ok(),
                "sampler" => result.sampler = Some(value.to_string()),
                "cfg scale" | "cfg" => result.cfg = value.parse().ok(),
                "seed" => result.seed = value.parse().ok(),
                "model" => result.model = Some(value.to_string()),
                "size" => {
                    if let Some((width, height)) = value.split_once('x') {
                        result.width = width.trim().parse().ok();
                        result.height = height.trim().parse().ok();
                    }
                }
                _ => {}
            }
        }
    }

    result.status = if result.positive.is_some() || result.negative.is_some() {
        ParseStatus::Ok
    } else {
        ParseStatus::NoMeta
    };
    result
}

pub fn normalize_prompt(text: &str) -> String {
    let normalized: String = text.nfc().collect();
    let mut output = String::with_capacity(normalized.len());
    let mut chars = normalized.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                output.push(next);
            }
        } else if ch.is_whitespace() {
            if !output.ends_with(' ') {
                output.push(' ');
            }
        } else {
            output.push(ch);
        }
    }
    output.trim().to_lowercase()
}

pub fn prompt_hash(normalized: &str) -> String {
    blake3::hash(normalized.as_bytes()).to_hex().to_string()
}

pub fn search_text(text: &str) -> String {
    let unescaped = text.replace("\\(", "(").replace("\\)", ")");
    let chars: Vec<char> = unescaped.to_lowercase().chars().collect();
    let mut output = String::new();
    let mut index = 0;
    let mut depth = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '(' {
            depth += 1;
            index += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            index += 1;
            continue;
        }
        if ch == ':' && depth > 0 {
            let mut cursor = index + 1;
            let mut digits = false;
            let mut dot = false;
            while cursor < chars.len() && (chars[cursor].is_ascii_digit() || chars[cursor] == '.') {
                if chars[cursor] == '.' {
                    if dot {
                        break;
                    }
                    dot = true;
                } else {
                    digits = true;
                }
                cursor += 1;
            }
            if digits && cursor < chars.len() && chars[cursor] == ')' {
                index = cursor;
                continue;
            }
        }
        if ch == '<' {
            if let Some(end) = chars[index..].iter().position(|&item| item == '>') {
                let inner: String = chars[index + 1..index + end].iter().collect();
                let parts: Vec<&str> = inner.split(':').collect();
                if parts.first() == Some(&"lora") {
                    if let Some(name) = parts.get(1) {
                        output.push_str(name);
                        output.push(' ');
                    }
                    index += end + 1;
                    continue;
                }
            }
        }
        if ch.is_whitespace() {
            if !output.ends_with(' ') {
                output.push(' ');
            }
        } else {
            output.push(ch);
        }
        index += 1;
    }
    output.trim().to_string()
}
