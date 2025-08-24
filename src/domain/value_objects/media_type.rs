use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Media type using MIME type strings to match database enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaType(String);

/// Supported image formats (for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    Gif,
    Svg,
    Heic,
    Tiff,
}

/// Supported video formats (for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    Mp4,
    WebM,
    Ogg,
    Quicktime,
}

impl MediaType {
    /// Create a new `MediaType` from MIME string
    #[must_use]
    pub fn new(mime_type: &str) -> Self {
        Self(mime_type.to_string())
    }

    /// Get the MIME type string
    #[must_use]
    pub fn mime_type(&self) -> &str {
        &self.0
    }

    /// Check if this media type is an image
    #[must_use]
    pub fn is_image(&self) -> bool {
        self.0.starts_with("image/")
    }

    /// Check if this media type is a video
    #[must_use]
    pub fn is_video(&self) -> bool {
        self.0.starts_with("video/")
    }

    /// Get the file extension for this media type
    #[must_use]
    pub fn file_extension(&self) -> &'static str {
        match self.0.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/avif" => "avif",
            "image/svg+xml" => "svg",
            "image/heic" => "heic",
            "image/tiff" => "tiff",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            "video/ogg" => "ogg",
            "video/quicktime" => "mov",
            _ => "bin", // fallback for unknown types
        }
    }

    /// Create from `ImageFormat` (backward compatibility)
    #[must_use]
    pub fn from_image_format(format: ImageFormat) -> Self {
        Self(format.mime_type().to_string())
    }

    /// Create from `VideoFormat` (backward compatibility)
    #[must_use]
    pub fn from_video_format(format: VideoFormat) -> Self {
        Self(format.mime_type().to_string())
    }
}

impl FromStr for MediaType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Validate against known MIME types
        match s {
            "image/jpeg" | "image/png" | "image/gif" | "image/webp" | "image/avif"
            | "image/svg+xml" | "image/heic" | "image/tiff" | "video/mp4" | "video/webm"
            | "video/ogg" | "video/quicktime" => Ok(Self(s.to_string())),
            _ => Err("Unsupported MIME type"),
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
            Self::Svg => "svg",
            Self::Heic => "heic",
            Self::Tiff => "tiff",
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
            Self::Svg => "image/svg+xml",
            Self::Heic => "image/heic",
            Self::Tiff => "image/tiff",
        }
    }

    /// Check if this format supports transparency
    #[must_use]
    pub fn supports_transparency(&self) -> bool {
        matches!(self, Self::Png | Self::WebP | Self::Avif | Self::Gif | Self::Svg)
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
            Self::Ogg => "ogg",
            Self::Quicktime => "mov",
        }
    }

    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp4 => "video/mp4",
            Self::WebM => "video/webm",
            Self::Ogg => "video/ogg",
            Self::Quicktime => "video/quicktime",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_media_type() {
        let media_type = MediaType::new("image/jpeg");

        assert!(media_type.is_image());
        assert!(!media_type.is_video());
        assert_eq!(media_type.file_extension(), "jpg");
        assert_eq!(media_type.mime_type(), "image/jpeg");
        assert_eq!(media_type.to_string(), "image/jpeg");
    }

    #[test]
    fn test_video_media_type() {
        let media_type = MediaType::new("video/mp4");

        assert!(!media_type.is_image());
        assert!(media_type.is_video());
        assert_eq!(media_type.file_extension(), "mp4");
        assert_eq!(media_type.mime_type(), "video/mp4");
    }

    #[test]
    fn test_media_type_from_str() {
        let media_type = "image/png".parse::<MediaType>().unwrap();
        assert_eq!(media_type.mime_type(), "image/png");
        assert!(media_type.is_image());

        let invalid = "invalid/type".parse::<MediaType>();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_backward_compatibility() {
        let from_image = MediaType::from_image_format(ImageFormat::WebP);
        assert_eq!(from_image.mime_type(), "image/webp");

        let from_video = MediaType::from_video_format(VideoFormat::Mp4);
        assert_eq!(from_video.mime_type(), "video/mp4");
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

        assert_eq!(VideoFormat::Quicktime.file_extension(), "mov");
        assert_eq!(VideoFormat::Quicktime.mime_type(), "video/quicktime");
    }

    #[test]
    fn test_all_supported_mime_types() {
        let image_types = [
            "image/jpeg",
            "image/png",
            "image/gif",
            "image/webp",
            "image/avif",
            "image/svg+xml",
            "image/heic",
            "image/tiff",
        ];

        let video_types = ["video/mp4", "video/webm", "video/ogg", "video/quicktime"];

        for mime_type in image_types {
            let media_type = mime_type.parse::<MediaType>().unwrap();
            assert!(media_type.is_image());
            assert!(!media_type.is_video());
        }

        for mime_type in video_types {
            let media_type = mime_type.parse::<MediaType>().unwrap();
            assert!(media_type.is_video());
            assert!(!media_type.is_image());
        }
    }
}
