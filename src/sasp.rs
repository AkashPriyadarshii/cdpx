use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaspElement {
    pub id: u32,
    pub tag: String,
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaspSnapshot {
    pub title: String,
    pub url: String,
    pub elements: Vec<SaspElement>,
}

impl SaspSnapshot {
    /// Formats the snapshot into a token-budgeted representation for AI models.
    pub fn to_compact_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("[Page: {}] (url: {})\n", self.title, self.url));
        out.push_str("------------------------------------------------------------\n");

        if self.elements.is_empty() {
            out.push_str("(No visible interactive elements)\n");
            return out;
        }

        for el in &self.elements {
            let label = if el.name.is_empty() {
                el.tag.clone()
            } else {
                format!("\"{}\"", el.name)
            };

            let val_str = match &el.value {
                Some(v) if !v.is_empty() => format!(" [value: \"{}\"]", v),
                _ => String::new(),
            };

            out.push_str(&format!(
                "@e{}: {} {} (x: {}, y: {}){}\n",
                el.id, el.role, label, el.x, el.y, val_str
            ));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sasp_compact_string_formatting() {
        let snapshot = SaspSnapshot {
            title: "Hacker News".to_string(),
            url: "https://news.ycombinator.com".to_string(),
            elements: vec![
                SaspElement {
                    id: 1,
                    tag: "a".to_string(),
                    role: "link".to_string(),
                    name: "Hacker News".to_string(),
                    value: None,
                    x: 20,
                    y: 10,
                    w: 100,
                    h: 20,
                },
                SaspElement {
                    id: 2,
                    tag: "input".to_string(),
                    role: "textbox".to_string(),
                    name: "Search".to_string(),
                    value: Some("rust".to_string()),
                    x: 200,
                    y: 10,
                    w: 120,
                    h: 24,
                },
            ],
        };

        let formatted = snapshot.to_compact_string();
        assert!(formatted.contains("[Page: Hacker News]"));
        assert!(formatted.contains(r#"@e1: link "Hacker News" (x: 20, y: 10)"#));
        assert!(formatted.contains(r#"@e2: textbox "Search" (x: 200, y: 10) [value: "rust"]"#));
    }
}
