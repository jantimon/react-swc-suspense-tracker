/// Normalizes a file path for use in IDs by removing common prefixes
/// and converting to a more readable format
pub fn normalize_filename(filename: &str) -> String {
    // Remove common project prefixes
    let cleaned = filename
        .strip_prefix("./")
        .or_else(|| filename.strip_prefix("/"))
        .unwrap_or(filename);
    
    // Convert backslashes to forward slashes for consistency
    cleaned.replace('\\', "/")
}

/// Generates a clean, readable ID for a Suspense boundary
/// Format: "path/to/file.tsx:line"
pub fn generate_boundary_id(filename: &str, line: u32) -> String {
    let normalized = normalize_filename(filename);
    format!("{}:{}", normalized, line)
}

/// Extracts a reasonable line number from a span position
/// This is a rough approximation since we don't have access to the full source map
pub fn extract_line_number(span_lo: u32) -> u32 {
    // This is a simple heuristic - in practice, you might want to 
    // implement more sophisticated line number extraction
    span_lo / 80 + 1 // Assuming ~80 chars per line average
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_filename() {
        assert_eq!(normalize_filename("./src/components/App.tsx"), "src/components/App.tsx");
        assert_eq!(normalize_filename("/Users/dev/project/src/App.tsx"), "Users/dev/project/src/App.tsx");
        assert_eq!(normalize_filename("src\\components\\App.tsx"), "src/components/App.tsx");
    }

    #[test]
    fn test_generate_boundary_id() {
        assert_eq!(generate_boundary_id("./src/App.tsx", 42), "src/App.tsx:42");
        assert_eq!(generate_boundary_id("components/MyComponent.tsx", 123), "components/MyComponent.tsx:123");
    }

    #[test]
    fn test_extract_line_number() {
        assert_eq!(extract_line_number(0), 1);
        assert_eq!(extract_line_number(80), 2);
        assert_eq!(extract_line_number(160), 3);
    }
}