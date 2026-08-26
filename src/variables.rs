use std::ops::Range;

const OPEN: &str = "{{";
const CLOSE: &str = "}}";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub name: String,
}

pub fn spans(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut cursor = 0;

    while let Some(first_open) = text[cursor..].find(OPEN) {
        let first_open = cursor + first_open;
        let value_start = first_open + OPEN.len();

        let Some(close) = text[value_start..].find(CLOSE) else {
            break;
        };
        let close = value_start + close;
        let nested_open = text[first_open..close]
            .rfind(OPEN)
            .expect("the search window starts with an opener");
        let start = first_open + nested_open;
        let end = close + CLOSE.len();
        let name = text[start + OPEN.len()..close].trim();

        if !name.is_empty() {
            spans.push(Span {
                range: start..end,
                name: name.to_string(),
            });
        }

        cursor = end;
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(text: &str) -> Vec<String> {
        spans(text).into_iter().map(|span| span.name).collect()
    }

    #[test]
    fn parser_contract() {
        assert_eq!(names("{{base}}/{{version}}"), ["base", "version"]);
        assert!(spans("{{}}").is_empty());
        assert!(spans("{{   }}").is_empty());
        assert!(spans("{{base").is_empty());
        assert_eq!(names("{{a{{b}}"), ["b"]);
        assert_eq!(names("{{a}}{{b}}"), ["a", "b"]);
        let text = "https://é.example/{{ id }}";
        let found = spans(text);
        assert_eq!(&text[found[0].range.clone()], "{{ id }}");
        assert_eq!(found[0].name, "id");
    }
}
