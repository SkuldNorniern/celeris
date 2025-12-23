pub fn decode_html_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '&' {
            let mut entity = String::new();
            entity.push(ch);
            
            // Collect the entity - allow #, digits, letters, and x for hex
            while let Some(&next_ch) = chars.peek() {
                if next_ch == ';' {
                    entity.push(chars.next().unwrap());
                    break;
                } else if next_ch == '#' || next_ch.is_alphanumeric() || next_ch == 'x' || next_ch == 'X' {
                    entity.push(chars.next().unwrap());
                } else {
                    // Not a valid entity character, treat as regular text
                    break;
                }
            }
            
            // Decode the entity
            if entity.ends_with(';') {
                let decoded = decode_entity(&entity);
                result.push_str(&decoded);
            } else {
                // Not a complete entity, output as-is
                result.push_str(&entity);
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

fn decode_entity(entity: &str) -> String {
    // Remove & and ;
    let inner = &entity[1..entity.len() - 1];
    
    // Handle numeric entities like &#44160; or &#xAC00;
    if inner.starts_with('#') {
        let num_str = &inner[1..];
        if num_str.starts_with('x') || num_str.starts_with('X') {
            // Hexadecimal: &#xAC00;
            if let Ok(num) = u32::from_str_radix(&num_str[1..], 16) {
                if let Some(ch) = char::from_u32(num) {
                    return ch.to_string();
                }
            }
        } else {
            // Decimal: &#44160;
            if let Ok(num) = num_str.parse::<u32>() {
                if let Some(ch) = char::from_u32(num) {
                    return ch.to_string();
                }
            }
        }
    }
    
    // Handle named entities
    match inner {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => "\u{00A0}",
        "copy" => "©",
        "reg" => "®",
        "trade" => "™",
        "raquo" => "»",
        "laquo" => "«",
        "mdash" => "—",
        "ndash" => "–",
        "hellip" => "…",
        _ => entity, // Return original if unknown
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_numeric_entities() {
        // Test with actual Unicode values
        assert_eq!(decode_html_entities("&#54620;"), "한");
        assert_eq!(decode_html_entities("&#44397;"), "국");
        assert_eq!(decode_html_entities("&#65;"), "A");
    }
    
    #[test]
    fn test_named_entities() {
        assert_eq!(decode_html_entities("&amp;"), "&");
        assert_eq!(decode_html_entities("&lt;"), "<");
        assert_eq!(decode_html_entities("&gt;"), ">");
        assert_eq!(decode_html_entities("&quot;"), "\"");
        assert_eq!(decode_html_entities("&nbsp;"), "\u{00A0}");
    }
    
    #[test]
    fn test_mixed_text() {
        assert_eq!(decode_html_entities("Hello &amp; World"), "Hello & World");
        assert_eq!(decode_html_entities("&#54620;&#44397;"), "한국");
    }
}

