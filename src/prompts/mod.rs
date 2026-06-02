use crate::models::character::Character;
use crate::models::chapter::Chapter;
use crate::models::novel::Novel;
use crate::models::outline::OutlineNode;
use crate::providers::ChatMessage;

pub fn build_continue(
    novel: &Novel,
    chapter: &Chapter,
    characters: &[Character],
    outline: Option<&OutlineNode>,
) -> Vec<ChatMessage> {
    let system = format!(
        "你是一位资深小说家，擅长为用户续写章节。\n\
         风格：{}\n\
         类型：{}\n\
         叙事视角：{}\n\
         基调：{}\n\
         续写时保持角色言行一致、情节连贯、文风稳定。\n\
         直接输出正文，不要任何解释、前后缀或元说明。",
        non_blank(&novel.style, "通用"),
        non_blank(&novel.genre, "通用"),
        non_blank(&novel.pov, "第三人称"),
        non_blank(&novel.tone, "平稳"),
    );

    let mut context_parts: Vec<String> = Vec::new();
    if !novel.synopsis.is_empty() {
        context_parts.push(format!("【故事概要】\n{}", novel.synopsis));
    }
    if !characters.is_empty() {
        let list: Vec<String> = characters
            .iter()
            .map(|c| {
                format!(
                    "- {}（{}）: {}",
                    c.name,
                    non_blank(&c.role, "角色"),
                    non_blank(&c.description, "（暂无描述）")
                )
            })
            .collect();
        context_parts.push(format!("【出场人物】\n{}", list.join("\n")));
    }
    if let Some(node) = outline {
        context_parts.push(format!("【当前大纲节点】\n{}\n{}", node.title, node.summary));
    }

    let ctx = if context_parts.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", context_parts.join("\n\n"))
    };

    let previous = if chapter.content.is_empty() {
        "（章节目前为空）".to_string()
    } else {
        chapter.content.clone()
    };

    let user = format!(
        "{}\
         【本章已有正文】\n{}\n\n\
         请直接续写下一段（不要重复已有内容，不要复述指令）。",
        ctx, previous
    );

    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_rewrite(
    novel: &Novel,
    chapter: &Chapter,
    instruction: &str,
) -> Vec<ChatMessage> {
    let system = format!(
        "你是一位资深小说家，按用户指令改写章节。\n\
         保持原作的：类型={}, 视角={}, 风格={}, 基调={}。\n\
         只返回改写后的正文，不要任何说明。",
        non_blank(&novel.genre, "通用"),
        non_blank(&novel.pov, "第三人称"),
        non_blank(&novel.style, "通用"),
        non_blank(&novel.tone, "平稳"),
    );
    let user = format!(
        "【原文】\n{}\n\n【改写指令】\n{}\n\n请按指令改写并返回完整正文。",
        chapter.content, instruction
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_expand(
    novel: &Novel,
    chapter: &Chapter,
    anchor: &str,
    target_words: Option<u32>,
) -> Vec<ChatMessage> {
    let target = target_words
        .map(|w| format!("约 {} 字", w))
        .unwrap_or_else(|| "适度扩展".to_string());
    let system = format!(
        "你是一位资深小说家，围绕指定锚点扩写。\n风格={}，视角={}。",
        non_blank(&novel.style, "通用"),
        non_blank(&novel.pov, "第三人称"),
    );
    let user = format!(
        "【章节正文】\n{}\n\n【扩写锚点】\n{}\n\n【目标长度】\n{}\n\n\
         请围绕锚点扩写一段（{}），保持上下文连贯。",
        chapter.content, anchor, target, target
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_summarize(
    novel: &Novel,
    chapter: &Chapter,
    max_words: Option<u32>,
) -> Vec<ChatMessage> {
    let cap = max_words.unwrap_or(200);
    let system = format!(
        "你是一位文学编辑，为《{}》生成章节摘要。",
        novel.title
    );
    let user = format!(
        "【章节标题】{}\n\n【章节正文】\n{}\n\n请用不超过 {} 个汉字输出摘要，包含主要情节、关键人物、悬念。",
        chapter.title, chapter.content, cap
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_dialogue(
    novel: &Novel,
    chapter: &Chapter,
    characters: &[Character],
    situation: &str,
) -> Vec<ChatMessage> {
    let names: Vec<String> = characters.iter().map(|c| c.name.clone()).collect();
    let descs: Vec<String> = characters
        .iter()
        .map(|c| {
            format!(
                "{}（{}）：{}",
                c.name,
                non_blank(&c.role, "角色"),
                non_blank(&c.description, "（暂无描述）")
            )
        })
        .collect();
    let system = format!(
        "你是小说《{}》的对话作者。风格={}，视角={}。\n\
         写出生动、符合人物性格的对话；使用「角色名：台词」格式，必要时穿插简短动作描写。",
        novel.title,
        non_blank(&novel.style, "通用"),
        non_blank(&novel.pov, "第三人称"),
    );
    let user = format!(
        "【情境】\n{}\n\n【在场人物】\n{}\n\n【上文（供参考）】\n{}\n\n请生成一段对话。",
        situation,
        descs.join("\n"),
        tail(&chapter.content, 800),
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_outline(novel: &Novel, idea: &str, depth: Option<u32>) -> Vec<ChatMessage> {
    let d = depth.unwrap_or(2);
    let system = format!(
        "你是《{}》的大纲策划。类型={}，风格={}，视角={}。\n\
         输出一份可挂到章节的大纲节点，结构清晰，可直接写入数据库。",
        novel.title,
        non_blank(&novel.genre, "通用"),
        non_blank(&novel.style, "通用"),
        non_blank(&novel.pov, "第三人称"),
    );
    let user = format!(
        "【故事概要】\n{}\n\n【核心创意】\n{}\n\n【大纲深度】\n层级 {}\n\n\
         请用以下 JSON 数组输出节点（不要额外文本）：\n\
         [{{\"title\": \"...\", \"summary\": \"...\"}}, ...]",
        novel.synopsis, idea, d
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

pub fn build_character(
    novel: &Novel,
    name: Option<&str>,
    concept: &str,
    role: Option<&str>,
) -> Vec<ChatMessage> {
    let system = format!(
        "你是《{}》的人物设定师。风格={}，类型={}。\n\
         用 JSON 返回一个完整人物设定。",
        novel.title,
        non_blank(&novel.style, "通用"),
        non_blank(&novel.genre, "通用"),
    );
    let user = format!(
        "【名字】{}\n【角色定位】{}\n【核心概念】{}\n\n\
         请输出 JSON：\n\
         {{\"name\": \"...\", \"role\": \"...\", \"description\": \"...\", \
         \"traits\": [\"...\"], \"backstory\": \"...\"}}",
        name.unwrap_or("（请起名）"),
        role.unwrap_or("supporting"),
        concept,
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

fn non_blank(s: &str, fallback: &str) -> String {
    if s.trim().is_empty() { fallback.to_string() } else { s.to_string() }
}

fn tail(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let start = s.len().saturating_sub(max * 3);
        format!("…{}", &s[start..])
    }
}

#[allow(dead_code)]
fn _ensure_uses(names: Vec<String>) {
    let _ = names;
}
