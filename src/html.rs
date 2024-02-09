use anyhow::{anyhow, Result};
use scraper::{Html, Selector};
use unicode_segmentation::UnicodeSegmentation;

pub fn extract_code_and_paragraphs(html_content: &str) -> (Vec<String>, Vec<String>) {
    let document = Html::parse_document(html_content);
    let paragraph_selector = Selector::parse("p").unwrap();
    let code_selector = Selector::parse("code, pre").unwrap();

    let paragraphs = document
        .select(&paragraph_selector)
        .map(|element| element.text().collect::<String>())
        .collect::<Vec<_>>();

    let code_blocks = document
        .select(&code_selector)
        .map(|element| element.text().collect::<String>())
        .collect::<Vec<_>>();

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

pub fn get_chunks(input: Vec<String>, chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut chunk = String::new();

    for p in input {
        if chunk.len() + p.len() <= chunk_size {
            chunk.push_str(&(" ".to_string() + &p));
        } else if p.len() > chunk_size {
            let _ = p.split(' ').map(|w| {
                if chunk.len() + w.len() > chunk_size {
                    chunks.push(chunk.clone());
                    chunk = String::new();
                } else {
                    chunk.push_str(&(" ".to_string() + w));
                }
            });
        } else {
            println!("- [{}]", chunk);
            chunks.push(trim_whitespace(&chunk.clone()));
            chunk = String::new();
        }
    }

    // Filter non-empty chunks and return
    chunks.iter().filter(|c| !c.is_empty()).cloned().collect()
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
