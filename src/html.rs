use anyhow::{anyhow, Result};
use scraper::{Html, Selector};
use unicode_segmentation::UnicodeSegmentation;

pub fn extract_code_and_paragraphs(html_content: &str) -> (Vec<String>, Vec<String>) {
    let document = Html::parse_document(html_content);
    let selectors = Selector::parse("p, span, code, pre").unwrap();

    let mut paragraphs = Vec::new();
    let mut code_blocks = Vec::new();

    // Iterate over elements matching the selectors
    for element in document.select(&selectors) {
        let tag_name = element.value().name();
        match tag_name {
            "p" => {
                let t = trim_whitespace(&element.text().collect::<Vec<_>>().join(" "));
                if !t.is_empty() {
                    paragraphs.push(t);
                }
            }
            "pre" | "code" => {
                let t = trim_whitespace(&element.text().collect::<Vec<_>>().join(" "));
                if !t.is_empty() {
                    code_blocks.push(t);
                }
            }
            _ => {} // Ignore other elements
        }
    }

    //code_blocks.append(&mut pre_blocks);

    (paragraphs, code_blocks)
}

// Extract tables from html and return as a list of csv strings.
pub fn extract_tables_to_csv(html_content: &str) -> Result<Vec<String>> {
    let document = Html::parse_document(html_content);
    let table_selector = Selector::parse("table").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td, th").unwrap();

    let mut tables = Vec::new();

    for table in document.select(&table_selector) {
        let mut one_table = Vec::new();
        for row in table.select(&row_selector) {
            let mut cells = vec![];
            for cell in row.select(&cell_selector) {
                let cell_text = cell.text().collect::<Vec<_>>().join(" ").trim().to_string();
                let trimmed = cell_text.trim();
                if trimmed.is_empty() {
                    cells.push(cell_text);
                }
            }
            one_table.push(cells.join(","));
        }
        tables.push(one_table.join("\n"));
    }

    Ok(tables)
}

// One of the text in the vector can be
// 1. Less than 512 tokens
// 2. greater than 512 tokens, and can contain multiple words
// 3. the text itself can be a single word which is more than 3048 bytes
pub fn get_chunks(input: Vec<String>, chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut chunk = String::new();

    for p in input {
        let p = clean_input(p);
        let p_words = p.split_whitespace().count();
        let chunk_words = chunk.split_whitespace().count();
        // simple condition, the text is less than 512
        if p_words > chunk_size {
            let words: Vec<_> = p.split_whitespace().collect();
            for c in words.chunks(chunk_size) {
                // a long word??
                if c.len() > chunk_size {
                    dbg!(c);
                    dbg!("Not adding long word");
                }
                chunks.push(c.join(" "));
            }
        } else if chunk_words + p_words <= chunk_size {
            chunk += &(" ".to_string() + &p);
        } else {
            // check if the chunk is less than 2048 characters
            if chunk.clone().len() > 2048 {
                dbg!(chunk.clone());
            }
            chunks.push(chunk.clone().trim().to_string());
            dbg!(chunk.len());
            chunk = String::new();
        }
    }

    dbg!(chunk.len());
    chunks.push(chunk);

    chunks
}

fn process_elements(
    elements: Vec<&str>,
    chunk_size: usize,
    chunks: &mut Vec<String>,
    is_paragraph: bool,
) {
    if elements.is_empty() {
        return;
    }

    let mut current_chunk = String::new();
    let mut current_chunk_word_count = 0;

    for element in elements {
        let element_word_count = element.split_whitespace().count();
        let new_chunk = if current_chunk.is_empty() {
            element.to_string()
        } else {
            format!("{} {}", current_chunk, element)
        };

        if new_chunk.len() <= 2048 && current_chunk_word_count + element_word_count <= chunk_size {
            current_chunk = new_chunk;
            current_chunk_word_count += element_word_count;
        } else {
            // If the element itself is too long when starting a new chunk, split it further
            if current_chunk.is_empty() && (element.len() > 2048 || element_word_count > chunk_size)
            {
                let split_elements = if is_paragraph {
                    element.split(|c: char| ".!?".contains(c)).collect()
                } else {
                    element.split_whitespace().collect()
                };
                process_elements(split_elements, chunk_size, chunks, false);
            } else {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                    current_chunk = String::new();
                    current_chunk_word_count = 0;
                }
                // If the element is not a paragraph or already split, add it directly
                if !is_paragraph {
                    current_chunk = element.to_string();
                    current_chunk_word_count = element_word_count;
                }
            }
        }
    }

    // Don't forget to add the last chunk
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }
}

pub fn adaptive_split(contents: Vec<String>, chunk_size: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    process_elements(
        contents.iter().map(|s| s.as_str()).collect(),
        chunk_size,
        &mut chunks,
        true,
    );

    chunks
}

fn clean_input(input: String) -> String {
    input.trim().to_string()
}

// From here: https://stackoverflow.com/questions/71864137/whats-the-ideal-way-to-trim-extra-spaces-from-a-string
pub fn trim_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    s.split_whitespace().for_each(|w| {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(w);
    });
    result
}

// Apart from chunking the data, we must handle multi-byte characters which
// might be present in the body. Some chunks might still be slightly off by a
// byte or two because of unicode, so always pass the chunk_size slightly less
// than your target chunk size.
pub fn chunk_text_with_overlap(
    text: &str,
    chunk_size: usize,
    overlap: usize,
) -> Result<Vec<String>> {
    if chunk_size == 0 || overlap >= chunk_size {
        return Err(anyhow!(
            "Chunk size must be greater than zero and greater than overlap size".to_string(),
        ));
    }

    let trimmed = trim_whitespace(text);
    let mut chunks = Vec::new();

    let mut start = 0;
    let mut end = chunk_size;

    while end <= trimmed.graphemes(true).count() {
        let chunk = text
            .graphemes(true)
            .skip(start)
            .take(chunk_size)
            .collect::<String>();
        chunks.push(chunk);

        start += chunk_size - overlap;
        end += chunk_size - overlap;
    }

    Ok(chunks)
}
