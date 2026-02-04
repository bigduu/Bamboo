use crate::models::Memory;

pub fn enhance_prompt(base_prompt: &str, memories: &[Memory]) -> String {
    if memories.is_empty() {
        return base_prompt.to_string();
    }

    let mut prompt = base_prompt.to_string();
    prompt.push_str("\n\n以下是需要长期记住的信息：\n");

    for memory in memories.iter().take(50) {
        prompt.push_str("- ");
        prompt.push_str(memory.content.trim());
        prompt.push('\n');
    }

    prompt
}
