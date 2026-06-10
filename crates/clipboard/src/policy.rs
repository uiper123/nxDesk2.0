use crate::traits::{ClipboardPolicy, ClipboardContent};

pub struct ConfigurableClipboardPolicy {
    pub allow_text: bool,
    pub allow_image: bool,
    pub allow_html: bool,
    pub max_text_len: usize,
    pub max_image_size: usize,
    pub max_html_len: usize,
}

impl ConfigurableClipboardPolicy {
    pub fn new_default() -> Self {
        Self {
            allow_text: true,
            allow_image: true,
            allow_html: true,
            max_text_len: 1024 * 1024,      // 1 MB
            max_image_size: 5 * 1024 * 1024, // 5 MB
            max_html_len: 2 * 1024 * 1024,  // 2 MB
        }
    }
}

impl ClipboardPolicy for ConfigurableClipboardPolicy {
    fn is_allowed(&self, content: &ClipboardContent) -> bool {
        match content {
            ClipboardContent::Text(t) => {
                self.allow_text && t.len() <= self.max_text_len
            }
            ClipboardContent::Image(img) => {
                self.allow_image && img.len() <= self.max_image_size
            }
            ClipboardContent::Html(html) => {
                self.allow_html && html.len() <= self.max_html_len
            }
        }
    }

    fn max_size(&self) -> usize {
        self.max_image_size.max(self.max_text_len).max(self.max_html_len)
    }
}
