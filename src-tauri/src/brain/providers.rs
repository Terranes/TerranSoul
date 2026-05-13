use serde_json::{Map, Value};

const GEMINI_STRIP_KEYS: &[&str] = &[
    "discriminator",
    "const",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "additionalProperties",
    "$schema",
    "$id",
    "$ref",
    "contentEncoding",
    "contentMediaType",
];

/// Sanitize JSON Schema for Gemini/Vertex tool-calling compatibility.
///
/// Rules:
/// - inline local `$ref` pointers into `#/$defs/*` targets
/// - drop `$defs`
/// - recursively strip Gemini-rejected keys
pub fn sanitize_tool_schema_for_gemini(schema: &mut Value) {
    let defs = collect_defs(schema);
    inline_local_refs(schema, &defs);

    if let Some(obj) = schema.as_object_mut() {
        obj.remove("$defs");
    }

    strip_gemini_rejected_keys(schema);
}

/// Adapter used by free-mode tool-calling paths.
///
/// Gemini and Vertex AI endpoints require schema sanitization, while other
/// providers should receive the original schema unchanged.
pub fn adapt_tool_schema_for_free_provider(provider_id: &str, schema: &mut Value) {
    if is_gemini_or_vertex(provider_id) {
        sanitize_tool_schema_for_gemini(schema);
    }
}

fn is_gemini_or_vertex(provider_id: &str) -> bool {
    let provider = provider_id.to_ascii_lowercase();
    provider.contains("gemini") || provider.contains("vertex")
}

fn collect_defs(schema: &Value) -> Map<String, Value> {
    schema
        .as_object()
        .and_then(|obj| obj.get("$defs"))
        .and_then(|defs| defs.as_object())
        .cloned()
        .unwrap_or_default()
}

fn resolve_local_ref<'a>(ref_path: &str, defs: &'a Map<String, Value>) -> Option<&'a Value> {
    let key = ref_path.strip_prefix("#/$defs/")?;
    defs.get(key)
}

fn inline_local_refs(value: &mut Value, defs: &Map<String, Value>) {
    match value {
        Value::Object(obj) => {
            // Inline local $ref first so recursive descent sees expanded shape.
            if let Some(ref_path) = obj.get("$ref").and_then(Value::as_str) {
                if let Some(resolved) = resolve_local_ref(ref_path, defs) {
                    let mut replacement = resolved.clone();
                    inline_local_refs(&mut replacement, defs);

                    let mut extras = obj.clone();
                    extras.remove("$ref");

                    match &mut replacement {
                        Value::Object(rep_obj) => {
                            for (k, v) in extras {
                                rep_obj.insert(k, v);
                            }
                        }
                        _ => {
                            // If the referenced value is not an object, keep it as-is.
                            // JSON Schema refs for tool parameters are expected to be objects.
                        }
                    }

                    *value = replacement;
                }
            }

            if let Value::Object(new_obj) = value {
                for child in new_obj.values_mut() {
                    inline_local_refs(child, defs);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                inline_local_refs(item, defs);
            }
        }
        _ => {}
    }
}

fn strip_gemini_rejected_keys(value: &mut Value) {
    match value {
        Value::Object(obj) => {
            for key in GEMINI_STRIP_KEYS {
                obj.remove(*key);
            }
            for child in obj.values_mut() {
                strip_gemini_rejected_keys(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_gemini_rejected_keys(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_flattens_refs_and_drops_defs() {
        let mut schema = json!({
            "type": "object",
            "$defs": {
                "Payload": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "const": "fixed"
                        }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }
            },
            "properties": {
                "payload": {
                    "$ref": "#/$defs/Payload"
                }
            },
            "required": ["payload"]
        });

        sanitize_tool_schema_for_gemini(&mut schema);

        assert!(
            schema.get("$defs").is_none(),
            "expected $defs to be removed"
        );
        let payload = &schema["properties"]["payload"];
        assert_eq!(payload["type"], "object");
        assert!(payload.get("$ref").is_none(), "expected $ref to be inlined");
        assert!(payload.get("additionalProperties").is_none());
        assert!(payload["properties"]["query"].get("const").is_none());
    }

    #[test]
    fn sanitize_recursively_strips_gemini_rejected_fields() {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "tool-schema",
            "type": "object",
            "properties": {
                "nested": {
                    "type": "object",
                    "discriminator": { "propertyName": "kind" },
                    "exclusiveMinimum": 1,
                    "exclusiveMaximum": 10,
                    "contentEncoding": "base64",
                    "contentMediaType": "application/json",
                    "additionalProperties": false
                }
            }
        });

        sanitize_tool_schema_for_gemini(&mut schema);

        assert!(schema.get("$schema").is_none());
        assert!(schema.get("$id").is_none());
        let nested = &schema["properties"]["nested"];
        assert!(nested.get("discriminator").is_none());
        assert!(nested.get("exclusiveMinimum").is_none());
        assert!(nested.get("exclusiveMaximum").is_none());
        assert!(nested.get("contentEncoding").is_none());
        assert!(nested.get("contentMediaType").is_none());
        assert!(nested.get("additionalProperties").is_none());
    }

    #[test]
    fn free_provider_adapter_only_sanitizes_gemini_or_vertex() {
        let original = json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": "string",
                    "const": "x"
                }
            }
        });

        let mut non_gemini = original.clone();
        adapt_tool_schema_for_free_provider("openrouter", &mut non_gemini);
        assert_eq!(
            non_gemini, original,
            "non-gemini providers must be untouched"
        );

        let mut gemini = original.clone();
        adapt_tool_schema_for_free_provider("gemini", &mut gemini);
        assert!(gemini["properties"]["field"].get("const").is_none());

        let mut vertex = original;
        adapt_tool_schema_for_free_provider("vertex-ai", &mut vertex);
        assert!(vertex["properties"]["field"].get("const").is_none());
    }
}
