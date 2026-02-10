// this_file: crates/core/src/parser/entities.rs

use std::collections::HashMap;

/// Parse entity declarations from DOCTYPE
pub fn parse_entities_from_doctype(doctype: &str, entities: &mut HashMap<String, String>) {
    // Security check: limit the number of entities to prevent DoS attacks
    const MAX_ENTITIES: usize = 1000;
    let mut entity_count = 0;
    // Handle double-quoted entity declarations: <!ENTITY name "value">
    let entity_pattern = regex::Regex::new(r#"<!ENTITY\s+(\w+)\s+"([^"]*?)"\s*>"#).unwrap();
    for capture in entity_pattern.captures_iter(doctype) {
        if entity_count >= MAX_ENTITIES {
            break; // Prevent DoS attacks with too many entities
        }
        if let (Some(name), Some(value)) = (capture.get(1), capture.get(2)) {
            let entity_value = expand_standard_entities(value.as_str());
            entities.insert(name.as_str().to_string(), entity_value);
            entity_count += 1;
        }
    }

    // Handle single-quoted entity declarations: <!ENTITY name 'value'>
    let entity_pattern_single = regex::Regex::new(r#"<!ENTITY\s+(\w+)\s+'([^']*?)'\s*>"#).unwrap();
    for capture in entity_pattern_single.captures_iter(doctype) {
        if entity_count >= MAX_ENTITIES {
            break; // Prevent DoS attacks with too many entities
        }
        if let (Some(name), Some(value)) = (capture.get(1), capture.get(2)) {
            let entity_value = expand_standard_entities(value.as_str());
            entities.insert(name.as_str().to_string(), entity_value);
            entity_count += 1;
        }
    }

    // Handle parameter entities: <!ENTITY % name "value">
    let param_entity_pattern = regex::Regex::new(r#"<!ENTITY\s+%\s+(\w+)\s+"([^"]*?)"\s*>"#).unwrap();
    for capture in param_entity_pattern.captures_iter(doctype) {
        if entity_count >= MAX_ENTITIES {
            break; // Prevent DoS attacks with too many entities
        }
        if let (Some(name), Some(value)) = (capture.get(1), capture.get(2)) {
            let entity_name = format!("%{};", name.as_str());
            let entity_value = expand_standard_entities(value.as_str());
            entities.insert(entity_name, entity_value);
            entity_count += 1;
        }
    }

    // Handle external entity references: <!ENTITY name SYSTEM "uri">
    // For SVG processing, we typically ignore external entities for security reasons
    // but we can at least log them or provide a placeholder
    let external_entity_pattern = regex::Regex::new(r#"<!ENTITY\s+(\w+)\s+SYSTEM\s+"([^"]+)"\s*>"#).unwrap();
    for capture in external_entity_pattern.captures_iter(doctype) {
        if entity_count >= MAX_ENTITIES {
            break; // Prevent DoS attacks with too many entities
        }
        if let (Some(name), Some(_uri)) = (capture.get(1), capture.get(2)) {
            // For security, we replace external entities with a placeholder
            entities.insert(name.as_str().to_string(), "[external entity]".to_string());
            entity_count += 1;
        }
    }

    // Handle public external entities: <!ENTITY name PUBLIC "public_id" "system_id">
    let public_entity_pattern = regex::Regex::new(r#"<!ENTITY\s+(\w+)\s+PUBLIC\s+"[^"]*"\s+"[^"]*"\s*>"#).unwrap();
    for capture in public_entity_pattern.captures_iter(doctype) {
        if entity_count >= MAX_ENTITIES {
            break; // Prevent DoS attacks with too many entities
        }
        if let Some(name) = capture.get(1) {
            // For security, we replace public entities with a placeholder
            entities.insert(name.as_str().to_string(), "[public entity]".to_string());
            entity_count += 1;
        }
    }
}

/// Expand entity references in text
pub fn expand_entities_in_text(text: &str, entities: &HashMap<String, String>) -> String {
    let mut result = text.to_string();

    // Replace standard XML entities first
    result = expand_standard_entities(&result);

    // Replace numeric character references (&#123; or &#x7B;)
    result = expand_numeric_entities(&result);

    // Replace custom entity references (&entity;)
    for (name, value) in entities {
        let entity_ref = format!("&{name};");
        result = result.replace(&entity_ref, value);
    }

    result
}

/// Expand standard XML entities (&lt;, &gt;, &amp;, &quot;, &apos;)
pub fn expand_standard_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Expand numeric character references (&#123; and &#x7B;)
pub fn expand_numeric_entities(text: &str) -> String {
    // Handle decimal numeric character references (&#123;)
    let decimal_pattern = regex::Regex::new(r"&#(\d+);?").unwrap();
    let mut result = decimal_pattern
        .replace_all(text, |caps: &regex::Captures| {
            if let Some(num_str) = caps.get(1) {
                if let Ok(code_point) = num_str.as_str().parse::<u32>() {
                    if let Some(ch) = std::char::from_u32(code_point) {
                        return ch.to_string();
                    }
                }
            }
            caps[0].to_string() // Return original if parsing fails
        })
        .into_owned();

    // Handle hexadecimal numeric character references (&#x7B; or &#X7B;)
    let hex_pattern = regex::Regex::new(r"&#[xX]([0-9a-fA-F]+);?").unwrap();
    result = hex_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(hex_str) = caps.get(1) {
                if let Ok(code_point) = u32::from_str_radix(hex_str.as_str(), 16) {
                    if let Some(ch) = std::char::from_u32(code_point) {
                        return ch.to_string();
                    }
                }
            }
            caps[0].to_string() // Return original if parsing fails
        })
        .into_owned();

    result
}
