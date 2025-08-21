use serde::{Deserialize, Serialize};

/// Media type classification for files
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Image { format: ImageFormat, width: u32, height: u32 },
    Video { format: VideoFormat, width: u32, height: u32, duration_seconds: Option<u32> },
}

/// Supported image formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    Gif,
}

/// Supported video formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    Mp4,
    WebM,
    Mov,
    Avi,
}

impl MediaType {
    /// Check if this media type is an image
    #[must_use]
    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image { .. })
    }

    /// Check if this media type is a video
    #[must_use]
    pub fn is_video(&self) -> bool {
        matches!(self, Self::Video { .. })
    }

    /// Get the file extension for this media type
    #[must_use]
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Image { format, .. } => format.file_extension(),
            Self::Video { format, .. } => format.file_extension(),
        }
    }

    /// Get the MIME type string
    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Image { format, .. } => format.mime_type(),
            Self::Video { format, .. } => format.mime_type(),
        }
    }

    /// Get dimensions as a tuple (width, height)
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Image { width, height, .. } | Self::Video { width, height, .. } => {
                (*width, *height)
            }
        }
    }
}

impl ImageFormat {
    #[must_use]
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Gif => "gif",
        }
    }

    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
            Self::Gif => "image/gif",
        }
    }

    /// Check if this format supports transparency
    #[must_use]
    pub fn supports_transparency(&self) -> bool {
        matches!(self, Self::Png | Self::WebP | Self::Avif | Self::Gif)
    }

    /// Check if this format supports animation
    #[must_use]
    pub fn supports_animation(&self) -> bool {
        matches!(self, Self::WebP | Self::Avif | Self::Gif)
    }
}

impl VideoFormat {
    #[must_use]
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::WebM => "webm",
            Self::Mov => "mov",
            Self::Avi => "avi",
        }
    }

    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp4 => "video/mp4",
            Self::WebM => "video/webm",
            Self::Mov => "video/quicktime",
            Self::Avi => "video/x-msvideo",
        }
    }
}
