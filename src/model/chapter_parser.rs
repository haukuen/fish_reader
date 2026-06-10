use crate::model::novel::Chapter;

/// 候选类型：用于同类编号的分组评分
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateType {
    Arabic,  // 阿拉伯数字：1、12. 3：
    Chinese, // 中文数字：一、十二.
}

/// 弱候选信息，Phase 1 收集，Phase 2 评分
#[derive(Debug, Clone)]
struct WeakCandidate {
    line_num: usize,
    title: String,
    candidate_type: CandidateType,
    /// 解析出的数值，用于递增检测
    number: u32,
}

/// 排除行：不作为任何候选
fn is_excluded(line: &str) -> bool {
    let exclude_prefixes = [
        "正文完",
        "正文结束",
        "正文结尾",
        "第一节课",
        "第一部分",
        "第一集合",
        "第一集和",
    ];
    for prefix in &exclude_prefixes {
        if line.starts_with(prefix) {
            return true;
        }
    }
    false
}

/// 强候选：匹配后立即作为章节标题，无需评分
fn try_strong_candidate(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // 1. "第X章/回/节/卷/部/篇" 格式
    //    要求"第"和关键字之间的内容必须是合法章节编号，
    //    避免"他回过头"中的"回"被误匹配。
    let chapter_keywords = ['章', '回', '节', '卷', '部', '篇'];
    if line.starts_with("第")
        && let Some(keyword_pos) = line.find(chapter_keywords)
    {
        let start_index = "第".len();
        if keyword_pos > start_index {
            let number_part = &line[start_index..keyword_pos];
            if !number_part.chars().any(|c| c.is_whitespace()) && is_chapter_number(number_part) {
                return Some(line.to_string());
            }
        }
    }

    // 2. English "Chapter X" 格式
    if line.to_lowercase().starts_with("chapter") {
        return Some(line.to_string());
    }

    // 3. 特殊篇章名
    let special_chapters = [
        "序章", "序言", "楔子", "尾声", "后记", "番外", "终章", "结语", "引子", "开篇",
    ];
    for special in &special_chapters {
        if line.starts_with(special) {
            return Some(line.to_string());
        }
    }

    // 4. "卷X" 格式（如 "卷一 开始"、"卷五 长夜"）
    if line.starts_with("卷") {
        let after_juan: String = line.chars().skip(1).collect();
        if after_juan.starts_with(|c: char| c.is_ascii_digit())
            || is_chinese_number_start(&after_juan)
        {
            return Some(line.to_string());
        }
    }

    None
}

/// 判断字符是否可能是中文数字的开头
fn is_chinese_number_start(s: &str) -> bool {
    let chinese_numbers = [
        '一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '百', '千', '万', '零',
    ];
    s.chars()
        .next()
        .is_some_and(|c| chinese_numbers.contains(&c))
}

/// 判断字符串是否仅由合法章节编号字符组成
/// 用于"第X章"格式中 X 部分的校验，防止正文中的"回过头"被误匹配
fn is_chapter_number(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars().all(|c| {
        c.is_ascii_digit()
            || matches!(
                c,
                '一' | '二'
                    | '三'
                    | '四'
                    | '五'
                    | '六'
                    | '七'
                    | '八'
                    | '九'
                    | '十'
                    | '百'
                    | '千'
                    | '万'
                    | '零'
                    | '壹'
                    | '贰'
                    | '叁'
                    | '肆'
                    | '伍'
                    | '陆'
                    | '柒'
                    | '捌'
                    | '玖'
                    | '拾'
                    | '佰'
                    | '仟'
            )
    })
}

/// 将中文字符串开头的数字部分解析为 u32
/// 支持：一到九十九、百、千、万组合
/// 例："十二"→12, "一百零一"→101, "二千"→2000, "十万"→100000
fn parse_chinese_number(s: &str) -> Option<u32> {
    let chinese_digits: &[(char, u32)] = &[
        ('一', 1),
        ('二', 2),
        ('三', 3),
        ('四', 4),
        ('五', 5),
        ('六', 6),
        ('七', 7),
        ('八', 8),
        ('九', 9),
        ('十', 10),
        ('百', 100),
        ('千', 1000),
        ('万', 10000),
        ('零', 0),
    ];

    fn get_val(digits: &[(char, u32)], c: char) -> Option<u32> {
        digits.iter().find(|(ch, _)| *ch == c).map(|(_, v)| *v)
    }

    let chars: Vec<char> = s
        .chars()
        .take_while(|c| get_val(chinese_digits, *c).is_some())
        .collect();
    if chars.is_empty() {
        return None;
    }

    /// 解析万以内数值段：如 "三千四百五十六" → 3456
    fn parse_segment(chars: &[char], digits: &[(char, u32)]) -> u32 {
        let mut result: u32 = 0;
        let mut current: u32 = 0;
        for &c in chars {
            let val = get_val(digits, c).unwrap();
            match val {
                1000 | 100 | 10 => {
                    if current == 0 {
                        current = 1;
                    }
                    result += current * val;
                    current = 0;
                }
                0 => {}
                n => {
                    current += n;
                }
            }
        }
        result + current
    }

    // 在万处拆分：万之前的部分 * 10000 + 万之后的部分
    if let Some(wan_pos) = chars.iter().position(|&c| c == '万') {
        let before = &chars[..wan_pos];
        let after = &chars[wan_pos + 1..];
        let high = if before.is_empty() {
            1
        } else {
            parse_segment(before, chinese_digits)
        };
        let low = if after.is_empty() {
            0
        } else {
            parse_segment(after, chinese_digits)
        };
        Some(high * 10000 + low)
    } else {
        Some(parse_segment(&chars, chinese_digits))
    }
}

/// 弱候选：匹配后需要评分确认
/// 返回 (标题, 候选类型, 解析出的数值)
fn try_weak_candidate(line: &str) -> Option<(String, CandidateType, u32)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // 阿拉伯数字 + 、 (如 "12、雨夜")
    if let Some(result) = try_arabic_dunhao(line) {
        return Some(result);
    }

    // 阿拉伯数字 + . + 标题 (如 "12. 归来")
    if let Some(result) = try_arabic_dot_title(line) {
        return Some(result);
    }

    // 阿拉伯数字 + ： (如 "3：旧事")
    if let Some(result) = try_arabic_colon(line) {
        return Some(result);
    }

    // 中文数字 + 、 (如 "十二、旧事")
    if let Some(result) = try_chinese_dunhao(line) {
        return Some(result);
    }

    // 中文数字 + . (如 "十二.旧事")
    if let Some(result) = try_chinese_dot(line) {
        return Some(result);
    }

    None
}

fn try_arabic_dunhao(line: &str) -> Option<(String, CandidateType, u32)> {
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let digits_end = chars.iter().position(|c| !c.is_ascii_digit())?;
    if digits_end == 0 {
        return None;
    }
    if digits_end >= chars.len() {
        return None;
    }

    let separator = chars[digits_end];
    if separator != '、' {
        return None;
    }

    let num_str: String = chars[..digits_end].iter().collect();
    let number: u32 = num_str.parse().ok()?;

    // 必须后面有内容（标题部分）
    if digits_end + 1 >= chars.len() {
        return None;
    }

    Some((line.to_string(), CandidateType::Arabic, number))
}

fn try_arabic_dot_title(line: &str) -> Option<(String, CandidateType, u32)> {
    let dot_pos = line.find('.')?;
    if dot_pos == 0 {
        return None;
    }
    if dot_pos >= line.len() - 1 {
        return None;
    } // dot 后面必须有内容

    let num_str = &line[..dot_pos];
    if !num_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let number: u32 = num_str.parse().ok()?;

    // dot 后面必须有非空内容
    let after = line[dot_pos + 1..].trim();
    if after.is_empty() {
        return None;
    }

    Some((line.to_string(), CandidateType::Arabic, number))
}

fn try_arabic_colon(line: &str) -> Option<(String, CandidateType, u32)> {
    let colon_pos = line.find('：')?;
    if colon_pos == 0 {
        return None;
    }
    if colon_pos >= line.len() - '：'.len_utf8() {
        return None;
    }

    let num_str = &line[..colon_pos];
    if !num_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let number: u32 = num_str.parse().ok()?;

    let after = line[colon_pos + '：'.len_utf8()..].trim();
    if after.is_empty() {
        return None;
    }

    Some((line.to_string(), CandidateType::Arabic, number))
}

fn try_chinese_dunhao(line: &str) -> Option<(String, CandidateType, u32)> {
    let dunhao_pos = line.find('、')?;
    if dunhao_pos == 0 {
        return None;
    }

    let num_part = &line[..dunhao_pos];
    let number = parse_chinese_number(num_part)?;

    // 后面必须有内容
    if dunhao_pos + '、'.len_utf8() >= line.len() {
        return None;
    }

    Some((line.to_string(), CandidateType::Chinese, number))
}

fn try_chinese_dot(line: &str) -> Option<(String, CandidateType, u32)> {
    let dot_pos = line.find('.')?;
    if dot_pos == 0 {
        return None;
    }

    let num_part = &line[..dot_pos];
    let number = parse_chinese_number(num_part)?;

    // 后面必须有内容
    if dot_pos + 1 >= line.len() {
        return None;
    }

    Some((line.to_string(), CandidateType::Chinese, number))
}

/// 判断字符是否为正文标点（句号、问号、感叹号、逗号、分号等）
fn is_sentence_punctuation(c: char) -> bool {
    matches!(
        c,
        '。' | '？' | '！' | '，' | '；' | '.' | '?' | '!' | ',' | ';' | '…'
    )
}

/// 判断标题是否包含引号
fn contains_quotes(s: &str) -> bool {
    s.contains('\u{201c}')
        || s.contains('\u{201d}')
        || s.contains('"')
        || s.contains('"')
        || s.contains('\u{300c}')
        || s.contains('\u{300d}')
        || s.contains('\u{300e}')
        || s.contains('\u{300f}')
        || s.contains('\'')
}

/// 计算单个弱候选的得分
/// 得分 ≥2 分则通过为章节
fn score_weak_candidate(
    candidate: &WeakCandidate,
    lines: &[String],
    all_weak: &[WeakCandidate],
) -> i32 {
    let mut score: i32 = 0;

    // 预计算同类候选组，供多个评分函数复用
    let same_type: Vec<&WeakCandidate> = all_weak
        .iter()
        .filter(|c| c.candidate_type == candidate.candidate_type)
        .collect();

    // === 加分项 ===

    // 1. 标题 ≤ 14 个字符: +2
    if candidate.title.chars().count() <= 14 {
        score += 2;
    }

    // 2. 前一行或后一行为空行: +2
    let prev_empty = candidate.line_num == 0 || lines[candidate.line_num - 1].trim().is_empty();
    let next_empty =
        candidate.line_num + 1 >= lines.len() || lines[candidate.line_num + 1].trim().is_empty();
    if prev_empty || next_empty {
        score += 2;
    }

    // 3. 全文存在同类递增编号候选，且行距 ≥3: +3
    if has_sparse_incrementing_siblings(candidate, &same_type) {
        score += 3;
    }

    // === 扣分项 ===

    // 4. 标题以正文标点结尾: -4
    if candidate
        .title
        .chars()
        .last()
        .is_some_and(is_sentence_punctuation)
    {
        score -= 4;
    }

    // 5. 标题包含引号: -4
    if contains_quotes(&candidate.title) {
        score -= 4;
    }

    // 6. 标题 > 24 个字符: -3
    if candidate.title.chars().count() > 24 {
        score -= 3;
    }

    // 7. 连续同类编号候选，行距 ≤2 且无空行: -4
    if is_dense_consecutive(candidate, &same_type, lines) {
        score -= 4;
    }

    // 8. 紧跟着不同编号类型的子项且无空行: -3
    if followed_by_different_type_children(candidate, all_weak, lines) {
        score -= 3;
    }

    // 9. 紧邻的不同类型弱候选（无空行间隔）: -4
    if has_different_type_neighbor(candidate, all_weak, lines) {
        score -= 4;
    }

    // 10. 同类顺序编号候选但行距 <3: -2
    if has_close_sequential_sibling(candidate, &same_type) {
        score -= 2;
    }

    score
}

/// 全文是否存在同类递增编号且行距≥3的候选对
fn has_sparse_incrementing_siblings(
    _candidate: &WeakCandidate,
    same_type: &[&WeakCandidate],
) -> bool {
    for i in 0..same_type.len() {
        for j in (i + 1)..same_type.len() {
            let a = same_type[i];
            let b = same_type[j];
            let line_gap = b.line_num.abs_diff(a.line_num);
            // 行距 ≥3 且编号严格连续递增
            if line_gap >= 3 && b.number == a.number + 1 {
                return true;
            }
        }
    }
    false
}

/// 是否存在紧邻的前后同类候选，行距 ≤2 且中间无空行
fn is_dense_consecutive(
    candidate: &WeakCandidate,
    same_type: &[&WeakCandidate],
    lines: &[String],
) -> bool {
    let pos = same_type
        .iter()
        .position(|c| c.line_num == candidate.line_num);
    if let Some(idx) = pos {
        // 检查前一个
        if idx > 0 {
            let prev = same_type[idx - 1];
            let gap = candidate.line_num - prev.line_num;
            if gap <= 2 && !has_empty_line_between(lines, prev.line_num, candidate.line_num) {
                return true;
            }
        }
        // 检查后一个
        if idx + 1 < same_type.len() {
            let next = same_type[idx + 1];
            let gap = next.line_num - candidate.line_num;
            if gap <= 2 && !has_empty_line_between(lines, candidate.line_num, next.line_num) {
                return true;
            }
        }
    }
    false
}

/// 紧跟着不同编号类型的子项（如 "二、" 后紧跟 "1."）且无空行
fn followed_by_different_type_children(
    candidate: &WeakCandidate,
    all: &[WeakCandidate],
    lines: &[String],
) -> bool {
    let next_line = candidate.line_num + 1;
    if next_line >= lines.len() {
        return false;
    }

    let next_trimmed = lines[next_line].trim();
    if next_trimmed.is_empty() {
        return false; // 有空行，不符合"且无空行"
    }

    // 检查下一行是否是弱候选且类型不同
    let next_candidate = all.iter().find(|c| c.line_num == next_line);
    if let Some(next_c) = next_candidate
        && next_c.candidate_type != candidate.candidate_type
    {
        return true;
    }

    false
}

/// 同类顺序编号候选但行距 <3
fn has_close_sequential_sibling(candidate: &WeakCandidate, same_type: &[&WeakCandidate]) -> bool {
    let pos = same_type
        .iter()
        .position(|c| c.line_num == candidate.line_num);
    if let Some(idx) = pos {
        // 检查前一个：编号顺序且行距 <3
        if idx > 0 {
            let prev = same_type[idx - 1];
            let gap = candidate.line_num - prev.line_num;
            if gap < 3 && candidate.number > prev.number {
                return true;
            }
        }
        // 检查后一个：编号顺序且行距 <3
        if idx + 1 < same_type.len() {
            let next = same_type[idx + 1];
            let gap = next.line_num - candidate.line_num;
            if gap < 3 && next.number > candidate.number {
                return true;
            }
        }
    }
    false
}

/// 检查紧邻的前后行是否有不同类型的弱候选（无空行间隔）
fn has_different_type_neighbor(
    candidate: &WeakCandidate,
    all: &[WeakCandidate],
    lines: &[String],
) -> bool {
    // 检查前一行
    if candidate.line_num > 0 {
        let prev_line = candidate.line_num - 1;
        if !lines[prev_line].trim().is_empty()
            && let Some(prev_c) = all.iter().find(|c| c.line_num == prev_line)
            && prev_c.candidate_type != candidate.candidate_type
        {
            return true;
        }
    }
    // 检查后一行
    if candidate.line_num + 1 < lines.len() {
        let next_line = candidate.line_num + 1;
        if !lines[next_line].trim().is_empty()
            && let Some(next_c) = all.iter().find(|c| c.line_num == next_line)
            && next_c.candidate_type != candidate.candidate_type
        {
            return true;
        }
    }
    false
}

/// 检查两个行号之间是否存在空行（不含起点和终点行）
fn has_empty_line_between(lines: &[String], from: usize, to: usize) -> bool {
    lines
        .iter()
        .take(to)
        .skip(from + 1)
        .any(|l| l.trim().is_empty())
}

/// 解析章节目录（两阶段：候选收集 + 评分过滤）
///
/// # Arguments
/// * `lines` - 小说的所有行
///
/// # Returns
/// 章节列表。空文件或无章节时返回单章 `[("全文", 0)]`。
/// 第一个章节前有内容时自动生成 `("前言", 0)` 章。
pub fn parse(lines: &[String]) -> Vec<Chapter> {
    if lines.iter().all(|l| l.trim().is_empty()) {
        return vec![Chapter {
            title: "全文".to_string(),
            start_line: 0,
        }];
    }

    // Phase 1: 收集候选
    let mut strong_chapters: Vec<Chapter> = Vec::new();
    let mut weak_candidates: Vec<WeakCandidate> = Vec::new();

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_excluded(trimmed) {
            continue;
        }

        if let Some(title) = try_strong_candidate(trimmed) {
            strong_chapters.push(Chapter {
                title,
                start_line: line_num,
            });
        } else if let Some((title, ct, number)) = try_weak_candidate(trimmed) {
            weak_candidates.push(WeakCandidate {
                line_num,
                title,
                candidate_type: ct,
                number,
            });
        }
    }

    // Phase 2: 弱候选评分
    let mut all_chapters: Vec<Chapter> = Vec::new();

    // 添加强候选
    all_chapters.extend(strong_chapters);

    // 添加通过评分的弱候选
    for candidate in &weak_candidates {
        let score = score_weak_candidate(candidate, lines, &weak_candidates);
        if score >= 2 {
            all_chapters.push(Chapter {
                title: candidate.title.clone(),
                start_line: candidate.line_num,
            });
        }
    }

    // 按行号排序
    all_chapters.sort_by_key(|ch| ch.start_line);

    // 如果没有章节，返回单章"全文"
    if all_chapters.is_empty() {
        return vec![Chapter {
            title: "全文".to_string(),
            start_line: 0,
        }];
    }

    // 如果第一个章节前有非空内容，自动生成"前言"章
    let first_start = all_chapters[0].start_line;
    if first_start > 0 {
        let has_content_before = lines[..first_start]
            .iter()
            .any(|l| !l.trim().is_empty() && !is_excluded(l.trim()));
        if has_content_before {
            all_chapters.insert(
                0,
                Chapter {
                    title: "前言".to_string(),
                    start_line: 0,
                },
            );
        }
    }

    all_chapters
}

#[cfg(test)]
mod tests {
    use super::*;

    // === 强候选检测 ===

    #[test]
    fn test_strong_di_zhang() {
        assert!(try_strong_candidate("第一章 雨夜").is_some());
        assert!(try_strong_candidate("第100回 风起").is_some());
        assert!(try_strong_candidate("第壹卷 山河").is_some());
        assert!(try_strong_candidate("第  章").is_none()); // 数字部分空
        assert!(try_strong_candidate("第一 章").is_none()); // 数字部分有空格
        assert!(try_strong_candidate("第 一章").is_none()); // 数字部分有空格
    }

    #[test]
    fn test_strong_english() {
        assert!(try_strong_candidate("Chapter 1 Beginning").is_some());
        assert!(try_strong_candidate("chapter 12 Return").is_some());
        assert!(try_strong_candidate("CHAPTER THREE").is_some());
    }

    #[test]
    fn test_strong_special() {
        assert!(try_strong_candidate("序章").is_some());
        assert!(try_strong_candidate("楔子").is_some());
        assert!(try_strong_candidate("尾声").is_some());
        assert!(try_strong_candidate("后记 全文完").is_some());
        assert!(try_strong_candidate("番外篇").is_some());
    }

    #[test]
    fn test_strong_juan() {
        assert!(try_strong_candidate("卷一 开始").is_some());
        assert!(try_strong_candidate("卷五 长夜").is_some());
        assert!(try_strong_candidate("卷十二").is_some());
    }

    // === 弱候选检测 ===

    #[test]
    fn test_weak_arabic_dunhao() {
        let result = try_weak_candidate("12、雨夜");
        assert!(result.is_some());
        let (title, ct, num) = result.unwrap();
        assert_eq!(title, "12、雨夜");
        assert_eq!(num, 12);
        assert!(matches!(ct, CandidateType::Arabic));
    }

    #[test]
    fn test_weak_arabic_dot() {
        let result = try_weak_candidate("3. 归来");
        assert!(result.is_some());
        assert_eq!(result.unwrap().2, 3);
    }

    #[test]
    fn test_weak_arabic_colon() {
        let result = try_weak_candidate("5：旧事");
        assert!(result.is_some());
        assert_eq!(result.unwrap().2, 5);
    }

    #[test]
    fn test_weak_chinese() {
        let result = try_weak_candidate("十二、旧事");
        assert!(result.is_some());
        let (_, ct, num) = result.unwrap();
        assert!(matches!(ct, CandidateType::Chinese));
        assert_eq!(num, 12);
    }

    // === 中文数字解析 ===

    #[test]
    fn test_parse_chinese_number() {
        assert_eq!(parse_chinese_number("一"), Some(1));
        assert_eq!(parse_chinese_number("十"), Some(10));
        assert_eq!(parse_chinese_number("十二"), Some(12));
        assert_eq!(parse_chinese_number("二十"), Some(20));
        assert_eq!(parse_chinese_number("二十五"), Some(25));
        assert_eq!(parse_chinese_number("一百"), Some(100));
        assert_eq!(parse_chinese_number("一百零一"), Some(101));
        assert_eq!(parse_chinese_number("一百二十"), Some(120));
        assert_eq!(parse_chinese_number("一千"), Some(1000));
        assert_eq!(parse_chinese_number("一万"), Some(10000));
        assert_eq!(parse_chinese_number("九十九"), Some(99));
        // 万+组合
        assert_eq!(parse_chinese_number("十万"), Some(100000));
        assert_eq!(parse_chinese_number("十二万"), Some(120000));
        assert_eq!(parse_chinese_number("一万二千"), Some(12000));
        assert_eq!(parse_chinese_number("十二万三千四百五十六"), Some(123456));
    }

    #[test]
    fn test_parse_chinese_number_negative() {
        assert_eq!(parse_chinese_number(""), None);
        assert_eq!(parse_chinese_number("abc"), None);
        assert_eq!(parse_chinese_number("零"), Some(0));
    }

    #[test]
    fn test_is_chinese_number_start() {
        assert!(is_chinese_number_start("一..."));
        assert!(is_chinese_number_start("十二"));
        assert!(!is_chinese_number_start("abc"));
        assert!(!is_chinese_number_start(""));
    }

    // === 排除检测 ===

    #[test]
    fn test_excluded() {
        assert!(is_excluded("正文完"));
        assert!(is_excluded("正文结束"));
        assert!(is_excluded("第一节课 数学"));
        assert!(is_excluded("第一集合"));
        assert!(!is_excluded("第一章 开始"));
        assert!(!is_excluded("普通文本"));
    }

    // === 完整解析测试 ===

    fn lines_from(text: &str) -> Vec<String> {
        text.lines().map(String::from).collect()
    }

    #[test]
    fn test_empty_file() {
        let lines = lines_from("");
        let chapters = parse(&lines);
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "全文");
        assert_eq!(chapters[0].start_line, 0);
    }

    #[test]
    fn test_no_chapters() {
        let lines = lines_from("这是一段普通的文字。\n没有章节标题。\n只是正文。");
        let chapters = parse(&lines);
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "全文");
    }

    #[test]
    fn test_normal_chapters() {
        let text = "\
第一章 雨夜
正文内容...
第二章 归来
正文内容...";
        let chapters = parse(&lines_from(text));
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "第一章 雨夜");
        assert_eq!(chapters[1].title, "第二章 归来");
    }

    #[test]
    fn test_dense_list_not_chapters() {
        let text = "\
第一章 开始
他把规则写在纸上：
1、不要回头。
2、不要出声。
3、天亮之前不要开门。
第二章 后续";
        let chapters = parse(&lines_from(text));
        assert_eq!(chapters.len(), 2, "应该只有两章，列表不应被切章");
        assert_eq!(chapters[0].title, "第一章 开始");
        assert_eq!(chapters[1].title, "第二章 后续");
    }

    #[test]
    fn test_real_numbered_chapters() {
        let text = "\
1、雨夜

正文内容...

2、归来

正文内容...

3、旧事

正文内容...";
        let chapters = parse(&lines_from(text));
        assert_eq!(chapters.len(), 3, "三个数字章节应该被识别");
        assert_eq!(chapters[0].title, "1、雨夜");
        assert_eq!(chapters[1].title, "2、归来");
        assert_eq!(chapters[2].title, "3、旧事");
    }

    #[test]
    fn test_nested_list_not_chapters() {
        let text = "\
第一章 开始
信件内容如下：
二、设备
1.一支魔杖
2.一顶学院帽
三、其他物品
1.一支羽毛笔
第二章 后续";
        let chapters = parse(&lines_from(text));
        assert_eq!(chapters.len(), 2, "嵌套列表不应被切章");
        assert_eq!(chapters[0].title, "第一章 开始");
        assert_eq!(chapters[1].title, "第二章 后续");
    }

    #[test]
    fn test_excluded_patterns() {
        let text = "正文完\n后面还有内容";
        let chapters = parse(&lines_from(text));
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "全文");
    }

    #[test]
    fn test_first_class_excluded() {
        let text = "第一节课 数学\n第一章 真正的开始";
        let chapters = parse(&lines_from(text));
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "第一章 真正的开始");
    }

    #[test]
    fn test_preface_auto_generation() {
        let text = "这是前言内容\n\n第一章 开始\n正文";
        let chapters = parse(&lines_from(text));
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "前言");
        assert_eq!(chapters[0].start_line, 0);
        assert_eq!(chapters[1].title, "第一章 开始");
    }

    #[test]
    fn test_mixed_strong_and_weak() {
        // "1. 重要事件" and "2. 次要事件" are weak candidates.
        // They have consecutive numbers but get penalized:
        // - has_close_sequential_sibling: -2 (line gap < 3 between them)
        // Net: +2 (title ≤14) +2 (empty line) +3 (sparse incrementing) -2 (close sibling) = 5, passes
        // So we make them adjacent without enough separation to avoid the +3 bonus
        // by placing them with line_gap < 3 (no empty line between).
        let text = "\
楔子

第一章 开始
正文
1. 重要事件
2. 次要事件

更多正文

第二章 继续";
        let chapters = parse(&lines_from(text));
        assert_eq!(
            chapters.len(),
            3,
            "楔子、第一章、第二章 should be 3 chapters"
        );
        assert_eq!(chapters[0].title, "楔子");
        assert_eq!(chapters[1].title, "第一章 开始");
        assert_eq!(chapters[2].title, "第二章 继续");
    }

    #[test]
    fn test_di_er_ge_not_chapter() {
        // "第二个" 中的"回"出现在"回过头"中，不是章节关键字
        let line = "第二个跳出来的是塞德里克，他回过头紧张道：“谁如果不动手帮忙，事后让乌姆里奇教授知道了，一定会被罚关禁闭擦马桶的！”";
        assert!(
            try_strong_candidate(line).is_none(),
            "带有'回过头'的正文不应被视为章节"
        );
        assert!(try_weak_candidate(line).is_none(), "不应被识别为弱候选");

        // 也确认 "第二天" "第一次" "第二回"（这里"回"是真实关键字）的区分：
        assert!(try_strong_candidate("第二天").is_none(), "第二天不是章节");
        assert!(try_strong_candidate("第一次").is_none(), "第一次不是章节");
        assert!(
            try_strong_candidate("第二回 风起").is_some(),
            "第二回是真实章节"
        );
    }

    #[test]
    fn test_quoted_title_penalized() {
        // 引号+句号结尾的行看起来像对话而非章节标题
        let text = "\
一、\u{201c}第一章开始\u{201d}。

正文

二、真正章节

正文";
        let chapters = parse(&lines_from(text));
        let titles: Vec<&str> = chapters.iter().map(|c| c.title.as_str()).collect();
        assert!(!titles.contains(&"一、\u{201c}第一章开始\u{201d}。"));
        assert!(titles.contains(&"二、真正章节"));
    }
}
