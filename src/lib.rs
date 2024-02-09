use anyhow::Result;
use serde::{de::Error, Deserialize};

pub mod html;

#[cfg(test)]
mod test;

pub fn interleave_vectors<T: Clone + PartialEq>(v1: &[T], v2: &[T]) -> Vec<T> {
    let mut res = Vec::new();
    let mut iter1 = v1.iter();
    let mut iter2 = v2.iter();

    loop {
        let next1 = iter1.next();
        let next2 = iter2.next();

        if next1.is_none() && next2.is_none() {
            break;
        }

        if let Some(item) = next1 {
            if res.contains(item) {
                continue;
            }

            res.push(item.clone());
        }
        if let Some(item) = next2 {
            if res.contains(item) {
                continue;
            }

            res.push(item.clone());
        }
    }
    res
}

pub fn extract_and_parse_json<'a, T: Deserialize<'a>>(input: &'a str) -> Result<T> {
    let start = input.find('{');
    let end = input.rfind('}');

    match (start, end) {
        (Some(start), Some(end)) => {
            let json_str = &input[start..=end];
            Ok(serde_json::from_str::<T>(json_str)?)
        }
        _ => Err(serde_json::Error::custom("No JSON object found").into()),
    }
}
