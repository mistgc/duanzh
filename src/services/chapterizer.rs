use crate::models::{Chapter, ProcessResult};
use anyhow::Result;
use regex::Regex;
use std::sync::Arc;

pub async fn process_text(
    text: &str,
    llm_client: &Arc<crate::services::llm::LLMClient>,
) -> Result<ProcessResult> {
    // Step 1: Use regex to find potential chapter markers
    let chapters = identify_chapters_by_regex(text);

    // Step 2: Use LLM to validate chapters
    let validated_chapters = validate_chapters_with_llm(chapters, llm_client).await;

    // Step 3: Create EPUB from chapters
    let epub_id = create_epub_from_chapters(&validated_chapters)?;

    Ok(ProcessResult {
        chapters: validated_chapters,
        epub_id,
    })
}

pub fn identify_chapters_by_regex(text: &str) -> Vec<Chapter> {
    // Common chapter heading patterns including Chinese characters
    let patterns = vec![
        r"(?i)^\s*chapter\s+(\d+|\w+)\s*$", // Chapter 1, Chapter One, etc.
        r"(?i)^\s*chapter\s+(\d+|\w+)\s*-\s*(.+)$", // Chapter 1 - Title
        r"(?i)^\s*chapter\s+(\d+|\w+)\s*:\s*(.+)$", // Chapter 1: Title
        r"(?i)^\s*chap\.?\s*(\d+|\w+)\s*$", // Chap. 1, Chap 1, etc.
        r"(?i)^\s*section\s+(\d+|\w+)\s*$", // Section 1, etc.
        r"(?i)^\s*part\s+(\d+|\w+)\s*$",    // Part 1, etc.
        r"^\s*#\s+([^#].*)$",               // # Title (Markdown style)
        r"^\s*##\s+([^#].*)$",              // ## Title (Markdown style)
        r"^\s*\d+\.\s+([^.].*)$",           // 1. Title, etc.
        r"^\s*\d+\.\d+\s+(.+)$",            // 1.1 Title, etc.
        // Chinese chapter patterns
        r"^第\s*(\d+)\s*章\s*(.*)$", // 第1章 Title, 第 1 章 Title
        r"^第\s*([一二三四五六七八九十百千万]+)\s*章\s*(.*)$", // 第一章 Title, 第 一 章 Title
        r"^第\s*(\d+)\s*节\s*(.*)$", // 第1节 Title
        r"^第\s*([一二三四五六七八九十百千万]+)\s*节\s*(.*)$", // 第一节 Title
        r"^第\s*(\d+)\s*回\s*(.*)$", // 第1回 Title
        r"^第\s*([一二三四五六七八九十百千万]+)\s*回\s*(.*)$", // 第一回 Title
        r"^第\s*(\d+)\s*话\s*(.*)$", // 第1话 Title
        r"^第\s*([一二三四五六七八九十百千万]+)\s*话\s*(.*)$", // 第一话 Title
        r"^Chapter\s*第(\d+)\s*(.*)$", // Chapter第1 Title
        r"^\s*([^\r\n]{1,50})\s*第\s*(\d+)\s*章\s*$", // Title Chapter 1 (when title is before)
    ];

    let lines: Vec<&str> = text.lines().collect();

    // Compile all regex patterns
    let regexes: Vec<Regex> = patterns
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect();

    // If no patterns compiled successfully, return single chapter with all text
    if regexes.is_empty() {
        return vec![Chapter {
            title: "Complete Text".to_string(),
            content: text.to_string(),
            start_pos: 0,
            end_pos: text.len(),
        }];
    }

    // Find all lines that match chapter patterns, along with their position in the text
    let mut chapter_positions = Vec::new();
    let mut cumulative_pos = 0;  // Track position in the full text
    
    for (idx, line) in lines.iter().enumerate() {
        let line_start_pos = cumulative_pos;
        let line_end_pos = cumulative_pos + line.len();
        
        // Check if this line matches a chapter pattern
        for regex in &regexes {
            if let Some(captures) = regex.captures(line.trim()) {
                let chapter_title = if captures.len() > 1 {
                    // If there's a second capture group, it's the title
                    if let Some(title_match) = captures.get(2) {
                        let title = title_match.as_str().trim().to_string();
                        if title.is_empty() {
                            if let Some(num_match) = captures.get(1) {
                                format!("Chapter {}", num_match.as_str().trim())
                            } else {
                                line.trim().to_string()
                            }
                        } else {
                            title
                        }
                    } else if let Some(num_match) = captures.get(1) {
                        // If only the number is captured, create a title
                        format!("Chapter {}", num_match.as_str().trim())
                    } else {
                        line.trim().to_string()
                    }
                } else {
                    line.trim().to_string()
                };
                
                chapter_positions.push((line_start_pos, line_end_pos, chapter_title));
                break; // Found a pattern, don't check others
            }
        }
        
        // Update cumulative position (add line length + 1 for newline, except for last line)
        cumulative_pos = line_end_pos;
        if idx < lines.len() - 1 {  // Not the last line, add newline
            cumulative_pos += 1;
        }
    }

    // If no chapter markers found, return single chapter with all text
    if chapter_positions.is_empty() {
        return vec![Chapter {
            title: "Complete Text".to_string(),
            content: text.to_string(),
            start_pos: 0,
            end_pos: text.len(),
        }];
    }

    // Build chapters based on positions - each chapter marker defines a new chapter 
    // with content that follows it (up to the next marker)
    let mut chapters = Vec::new();
    
    for (i, (_, marker_end, marker_title)) in chapter_positions.iter().enumerate() {
        // Update current_start to after the current marker for this chapter's content
        let mut content_start = *marker_end;  // Start after the marker
        if content_start < text.len() && (text.as_bytes()[content_start] == b'\n' || text.as_bytes()[content_start] == b'\r') {
            // Skip the newline character(s) after the marker
            if text.as_bytes()[content_start] == b'\r' && content_start + 1 < text.len() && text.as_bytes()[content_start + 1] == b'\n' {
                content_start += 2; // Skip \r\n
            } else {
                content_start += 1; // Skip \n
            }
        }
        
        // Calculate end position for this chapter's content (up to next marker or end of text)
        let content_end = if i < chapter_positions.len() - 1 {
            // Up to the next marker
            chapter_positions[i + 1].0  // Start position of next marker
        } else {
            // Up to the end of text
            text.len()
        };
        
        // Extract the content for this chapter
        if content_end > content_start {
            let content = text[content_start..content_end].trim().to_string();
            if !content.is_empty() {
                chapters.push(Chapter {
                    title: marker_title.clone(),
                    content,
                    start_pos: content_start,
                    end_pos: content_end,
                });
            }
        }
    }

    // If no chapters with content were created, create a single chapter with all text
    if chapters.is_empty() {
        return vec![Chapter {
            title: "Complete Text".to_string(),
            content: text.to_string(),
            start_pos: 0,
            end_pos: text.len(),
        }];
    }

    // If we still have no chapters (maybe everything was in chapter headers), return single chapter
    if chapters.is_empty() {
        return vec![Chapter {
            title: "Complete Text".to_string(),
            content: text.to_string(),
            start_pos: 0,
            end_pos: text.len(),
        }];
    }

    chapters
}

async fn validate_chapters_with_llm(
    mut chapters: Vec<Chapter>,
    llm_client: &Arc<crate::services::llm::LLMClient>,
) -> Vec<Chapter> {
    for chapter in &mut chapters {
        match llm_client.validate_chapter(chapter).await {
            Ok(response) => {
                if response.is_valid {
                    if let Some(suggested_title) = response.suggested_title {
                        chapter.title = suggested_title;
                    }
                }
            }
            Err(e) => {
                eprintln!("LLM validation error: {}", e);
                // Continue with the original chapter if LLM validation fails
            }
        }
    }

    // Step 2.2: Sliding window validation of adjacent chapters
    let mut i = 0;
    while i < chapters.len() - 1 {
        match llm_client
            .compare_adjacent_chapters(&chapters[i], &chapters[i + 1])
            .await
        {
            Ok(response) => {
                if !response.is_valid {
                    // Merge the two chapters if the boundary is invalid
                    let next_chapter = chapters.remove(i + 1);
                    chapters[i].content.push_str("\n\n");
                    chapters[i].content.push_str(&next_chapter.content);
                    chapters[i].end_pos = next_chapter.end_pos;

                    // Don't increment i since we need to check the new combined chapter
                    // against the next one
                    continue;
                }
            }
            Err(e) => {
                eprintln!("Adjacent chapter comparison error: {}", e);
            }
        }
        i += 1;
    }

    // Step 2.3: Check for content modifications (our goal is just segmentation)
    chapters.retain(|_chapter| {
        // In a real implementation, we would check if content was modified
        // For now, we assume content is not modified if the process reaches here
        true
    });

    chapters
}

pub fn create_epub_from_chapters(chapters: &[Chapter]) -> Result<String> {
    use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};
    use std::io::Cursor;

    // Generate a unique ID for this EPUB
    let epub_id = uuid::Uuid::new_v4().to_string();

    // Create a temporary file path
    let filename = format!("./output/{}.epub", epub_id);

    // Create directory if it doesn't exist
    std::fs::create_dir_all("./output")?;

    // Create a cursor to hold the EPUB data in memory
    let mut cursor = Cursor::new(Vec::new());

    // Create an EPUB builder - handle the error and convert to anyhow::Result
    let zip_library = match ZipLibrary::new() {
        Ok(z) => z,
        Err(e) => return Err(anyhow::anyhow!("Failed to create ZIP library: {}", e)),
    };

    let mut builder = match EpubBuilder::new(zip_library) {
        Ok(b) => b,
        Err(e) => return Err(anyhow::anyhow!("Failed to create EPUB builder: {}", e)),
    };

    // Set metadata
    if let Err(e) = builder.metadata("title", "Generated Book") {
        return Err(anyhow::anyhow!("Failed to set title metadata: {}", e));
    }
    if let Err(e) = builder.metadata("author", "Text Chapterizer") {
        return Err(anyhow::anyhow!("Failed to set author metadata: {}", e));
    }

    // Add chapters to the EPUB - each with proper titles and navigation
    for (index, chapter) in chapters.iter().enumerate() {
        // Prepare chapter content in proper XHTML format
        let xhtml_content = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\">\n<head>\n  <title>{}</title>\n</head>\n<body>\n  <h1>{}</h1>\n  {}\n</body>\n</html>",
            html_escape::encode_text(&chapter.title),
            html_escape::encode_text(&chapter.title),
            // Convert newlines to paragraph breaks for better formatting
            chapter
                .content
                .split("\n\n") // Split by double newlines (paragraphs)
                .map(|para| {
                    let para_trimmed = para.trim();
                    if !para_trimmed.is_empty() {
                        format!("<p>{}</p>", html_escape::encode_text(para_trimmed))
                    } else {
                        String::new()
                    }
                })
                .filter(|s| !s.is_empty()) // Remove empty paragraphs
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Add the content to the EPUB with proper title and level
        if let Err(e) = builder.add_content(
            EpubContent::new(format!("chap_{}.xhtml", index + 1), xhtml_content.as_bytes())
                .title(&chapter.title)
                .level(1), // Level 1 for main chapters - this helps with navigation
        ) {
            return Err(anyhow::anyhow!(
                "Failed to add content for chapter {}: {}",
                index + 1,
                e
            ));
        }
    }

    // Ensure proper navigation by explicitly creating a navigation structure
    // Add an inline table of contents to help EPUB readers recognize chapters
    builder.inline_toc();

    // Generate the EPUB into our cursor
    if let Err(e) = builder.generate(&mut cursor) {
        return Err(anyhow::anyhow!("Failed to generate EPUB: {}", e));
    }

    // Write the cursor data to the actual file
    std::fs::write(&filename, cursor.into_inner())?;

    Ok(epub_id)
}
