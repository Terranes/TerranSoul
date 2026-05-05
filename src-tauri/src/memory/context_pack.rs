/// Format retrieved memory snippets as a query-scoped context pack.
///
/// The inner `[LONG-TERM MEMORY]` marker is kept for backward-compatible
/// prompt tests and existing LLM instructions, while the outer contract makes
/// clear that this is a database retrieval result, not the whole memory system.
pub fn format_retrieved_context_pack(memory_block: &str) -> String {
    format!(
        "\n\n[RETRIEVED CONTEXT]\n\
Source: TerranSoul queryable memory/RAG store.\n\
Contract: These are relevant retrieved records, not an exhaustive transcript or complete database. Use them when helpful, ignore irrelevant records, and say when retrieved context is insufficient.\n\
[LONG-TERM MEMORY]\n\
The following facts from your memory were retrieved for this turn:\n\
{memory_block}\n\
[/LONG-TERM MEMORY]\n\
[/RETRIEVED CONTEXT]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_pack_preserves_memory_marker_and_adds_contract() {
        let pack = format_retrieved_context_pack("- [long] User likes Python");

        assert!(pack.contains("[RETRIEVED CONTEXT]"));
        assert!(pack.contains("queryable memory/RAG store"));
        assert!(pack.contains("not an exhaustive transcript or complete database"));
        assert!(pack.contains("[LONG-TERM MEMORY]"));
        assert!(pack.contains("User likes Python"));
        assert!(pack.contains("[/RETRIEVED CONTEXT]"));
    }
}
