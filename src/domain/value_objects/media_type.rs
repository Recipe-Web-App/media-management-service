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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_media_type() {
        let media_type = MediaType::Image { format: ImageFormat::Jpeg, width: 1920, height: 1080 };

        assert!(media_type.is_image());
        assert!(!media_type.is_video());
        assert_eq!(media_type.file_extension(), "jpg");
        assert_eq!(media_type.mime_type(), "image/jpeg");
        assert_eq!(media_type.dimensions(), (1920, 1080));
    }

    #[test]
    fn test_video_media_type() {
        let media_type = MediaType::Video {
            format: VideoFormat::Mp4,
            width: 1280,
            height: 720,
            duration_seconds: Some(120),
        };

        assert!(!media_type.is_image());
        assert!(media_type.is_video());
        assert_eq!(media_type.file_extension(), "mp4");
        assert_eq!(media_type.mime_type(), "video/mp4");
        assert_eq!(media_type.dimensions(), (1280, 720));
    }

    #[test]
    fn test_image_format_properties() {
        assert_eq!(ImageFormat::Png.file_extension(), "png");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert!(ImageFormat::Png.supports_transparency());
        assert!(!ImageFormat::Png.supports_animation());

        assert_eq!(ImageFormat::Gif.file_extension(), "gif");
        assert_eq!(ImageFormat::Gif.mime_type(), "image/gif");
        assert!(ImageFormat::Gif.supports_transparency());
        assert!(ImageFormat::Gif.supports_animation());

        assert!(!ImageFormat::Jpeg.supports_transparency());
        assert!(!ImageFormat::Jpeg.supports_animation());
    }

    #[test]
    fn test_video_format_properties() {
        assert_eq!(VideoFormat::WebM.file_extension(), "webm");
        assert_eq!(VideoFormat::WebM.mime_type(), "video/webm");

        assert_eq!(VideoFormat::Mov.file_extension(), "mov");
        assert_eq!(VideoFormat::Mov.mime_type(), "video/quicktime");
    }
}
