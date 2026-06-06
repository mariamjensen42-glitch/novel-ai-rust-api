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

// ============== 新增动作：翻译 ==============

pub fn build_translate(
    novel: &Novel,
    chapter: &Chapter,
    characters: &[Character],
    target_language: &str,
    source_language: Option<&str>,
    preserve_style: bool,
) -> Vec<ChatMessage> {
    let src = source_language
        .map(|s| format!("（源语言：{}）", s))
        .unwrap_or_default();
    let style_guard = if preserve_style {
        "务必保留原作的：叙事节奏、修辞、口吻、对话风格；译文应让读者察觉不到'这是译作'。"
    } else {
        "以自然、地道为目标，不必逐字对照。"
    };
    let system = format!(
        "你是小说《{}》的文学译者。类型={}，风格={}，基调={}。\n\
         将用户提供的章节正文翻译为{}。\n\
         {}\n\
         直接输出译文正文，不要任何前缀、说明、注释。",
        novel.title,
        non_blank(&novel.genre, "通用"),
        non_blank(&novel.style, "通用"),
        non_blank(&novel.tone, "平稳"),
        target_language,
        style_guard,
    );

    let mut ctx_parts: Vec<String> = Vec::new();
    ctx_parts.push(format!("【目标语言】\n{}{}", target_language, src));
    if !characters.is_empty() {
        let list: Vec<String> = characters
            .iter()
            .map(|c| {
                format!(
                    "- {}（{}）：{}",
                    c.name,
                    non_blank(&c.role, "角色"),
                    non_blank(&c.description, "")
                )
            })
            .collect();
        ctx_parts.push(format!(
            "【人物名翻译参考】\n保持人名、地名、术语与下文一致：\n{}",
            list.join("\n")
        ));
    }

    let user = format!(
        "{}\n\n【章节标题】{}\n\n【原文】\n{}\n\n请翻译并只返回译文。",
        ctx_parts.join("\n\n"),
        chapter.title,
        chapter.content
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

// ============== 新增动作：润色 ==============

pub fn build_polish(
    novel: &Novel,
    chapter: &Chapter,
    focus: &str,
) -> Vec<ChatMessage> {
    let focus_desc = match focus {
        "dialogue" => "重点润色对话：让台词更贴合人物性格、节奏更自然、潜台词更丰富。",
        "description" => "重点润色环境与动作描写：用词更精准、画面感更强、避免冗余。",
        "pacing" => "重点润色节奏：句式长短交错、段落疏密有致、张弛有度。",
        "grammar" => "重点修正语法、用词、标点等硬伤；保持文风不变。",
        _ => "全面润色：兼顾语言流畅、节奏、对话、画面感，但保留作者原有的个人风格。",
    };
    let system = format!(
        "你是《{}》的资深文学编辑。类型={}，风格={}，视角={}，基调={}。\n\
         对章节正文进行润色，提升文字质量。\n\
         {}\n\
         只返回润色后的完整正文，不要任何说明、前缀、批注。",
        novel.title,
        non_blank(&novel.genre, "通用"),
        non_blank(&novel.style, "通用"),
        non_blank(&novel.pov, "第三人称"),
        non_blank(&novel.tone, "平稳"),
        focus_desc,
    );
    let user = format!(
        "【章节标题】{}\n\n【原文】\n{}\n\n请润色后返回完整正文。",
        chapter.title, chapter.content
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

// ============== 新增动作：风格转换 ==============

pub fn build_style_transfer(
    novel: &Novel,
    chapter: &Chapter,
    target_style: &str,
    source_style: Option<&str>,
) -> Vec<ChatMessage> {
    let src_hint = source_style
        .map(|s| format!("原文风格：{}\n", s))
        .unwrap_or_default();
    let system = format!(
        "你是文学风格的模仿大师。请将用户提供的章节正文改写为「{}」风格。\n\
         原作品类型={}，视角={}，基调={}。\n\
         要求：\n\
         - 保留核心情节、人物、关键信息\n\
         - 句式、用词、节奏、修辞要明显地呈现「{}」的标志性特征\n\
         - 不要输出任何说明、前缀、对比分析，只返回改写后的完整正文",
        target_style,
        non_blank(&novel.genre, "通用"),
        non_blank(&novel.pov, "第三人称"),
        non_blank(&novel.tone, "平稳"),
        target_style,
    );
    let user = format!(
        "【目标风格】\n{}{}\n【章节标题】{}\n\n【原文】\n{}\n\n请改写为「{}」风格并返回完整正文。",
        target_style, src_hint, chapter.title, chapter.content, target_style
    );
    vec![ChatMessage::system(system), ChatMessage::user(user)]
}

// ============== 新增动作：人设一致性检查 ==============

pub fn build_consistency_check(
    novel: &Novel,
    chapter: &Chapter,
    characters: &[Character],
) -> Vec<ChatMessage> {
    let mut char_block: Vec<String> = characters
        .iter()
        .map(|c| {
            let traits = non_blank(&c.traits, "（无）");
            let backstory = non_blank(&c.backstory, "（无）");
            format!(
                "## {}（{}）\n- 描述：{}\n- 性格标签：{}\n- 出身/经历：{}",
                c.name,
                non_blank(&c.role, "角色"),
                non_blank(&c.description, "（无）"),
                traits,
                backstory,
            )
        })
        .collect();

    if char_block.is_empty() {
        char_block.push("（未指定角色，将基于章节内出现的角色做一般性检查）".to_string());
    }

    let system = format!(
        "你是《{}》的人设一致性审校。\n\
         仔细阅读章节正文与人物设定，找出**客观存在**的人设矛盾：\n\
         - 角色言行是否违反其性格标签、背景、价值观\n\
         - 角色之间的关系是否前后一致（敌友、亲疏、师徒等）\n\
         - 角色已知的身体特征、年龄、技能、过往经历是否被违反\n\
         - 同一角色在前后文是否出现不可调和的逻辑冲突\n\n\
         输出 Markdown 报告，包含以下结构：\n\
         1. **总体评估**（一段话：人设一致度百分比 + 简要说明）\n\
         2. **发现的问题**（按角色分组，每条问题用 `###` 三级标题，列出原文摘录、对应设定、冲突分析、修改建议）\n\
         3. **未发现问题的角色**（简短列出）\n\n\
         如果找不到问题，也要明确说\"未发现明显人设冲突\"，不要编造问题。",
        novel.title
    );
    let user = format!(
        "【作品类型】{}\n【本章标题】{}\n\n【人物设定】\n{}\n\n【章节正文】\n{}\n\n请输出一致性检查报告。",
        non_blank(&novel.genre, "通用"),
        chapter.title,
        char_block.join("\n\n"),
        chapter.content,
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
