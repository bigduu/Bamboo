use serde::{Deserialize, Serialize};

/// Content type for messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    /// Simple text content
    Text { text: String },
    /// Multimodal content parts
    Parts { parts: Vec<ContentPart> },
}

/// Individual content part (for multimodal messages)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content
    Text { text: String },
    /// Image content
    Image { source: ImageSource },
}

/// Image source for vision models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64 encoded image data
    Base64 { data: String, mime_type: String },
    /// URL to an image
    Url { url: String },
}

impl Content {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create content from parts
    pub fn parts(parts: Vec<ContentPart>) -> Self {
        Self::Parts { parts }
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text { text } => text.is_empty(),
            Self::Parts { parts } => parts.is_empty(),
        }
    }
}

impl ContentPart {
    /// Create a text part
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image part from base64
    pub fn image_base64(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Base64 {
                data: data.into(),
                mime_type: mime_type.into(),
            },
        }
    }

    /// Create an image part from URL
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Url { url: url.into() },
        }
    }
}

impl ImageSource {
    /// Get the MIME type
    pub fn mime_type(&self) -> &str {
        match self {
            Self::Base64 { mime_type, .. } => mime_type,
            Self::Url { .. } => "image/url",
        }
    }

    /// Get the data (for base64) or URL
    pub fn data(&self) -> String {
        match self {
            Self::Base64 { data, .. } => data.clone(),
            Self::Url { url } => url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content() {
        let content = Content::text("Hello");
        match content {
            Content::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_parts_content() {
        let parts = vec![
            ContentPart::text("Hello "),
            ContentPart::text("World"),
        ];
        let content = Content::parts(parts);
        match content {
            Content::Parts { parts: p } => assert_eq!(p.len(), 2),
            _ => panic!("Expected parts content"),
        }
    }

    #[test]
    fn test_image_base64() {
        let part = ContentPart::image_base64("abc123", "image/png");
        match part {
            ContentPart::Image { source: ImageSource::Base64 { data, mime_type } } => {
                assert_eq!(data, "abc123");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected base64 image"),
        }
    }

    #[test]
    fn test_image_url() {
        let part = ContentPart::image_url("https://example.com/image.png");
        match part {
            ContentPart::Image { source: ImageSource::Url { url } } => {
                assert_eq!(url, "https://example.com/image.png");
            }
            _ => panic!("Expected URL image"),
        }
    }
}
