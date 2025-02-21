#![allow(non_snake_case)]
use dioxus::prelude::*;

#[component]
pub fn MetaSeoTemplate(
    title: String,
    keywords: Option<String>,
    author: Option<String>,
    #[props(default = "index, follow".to_string())] robots: String,
    url: String,
    canonical: String,
    twitter_id: Option<String>,
) -> Element {
    rsx! {
        if let Some(keywords) = keywords {
            document::Meta { name: "keywords", content: "{keywords}" }
        }
        if let Some(author) = author {
            document::Meta { name: "author", content: "{author}" }
        }
        document::Meta { name: "robots", content: "{robots}" }
        document::Link { rel: "canonical", href: "{canonical}" }
        document::Meta { property: "og:site_name", content: "{title}" }
        document::Meta { property: "og:url", content: "{url}" }
        if let Some(twitter_id) = twitter_id {
            document::Meta { property: "twitter:site", content: "{twitter_id}" }
            document::Meta { property: "twitter:creator", content: "{twitter_id}" }
        }
    }
}

// MetaPage supports Open Graph and Twitter meta tags.
// Also, it supports SEO, but we recommend using MetaSeo first in main.rs
#[component]
pub fn MetaPage(
    title: String,
    description: Option<String>,
    image: Option<String>,
    video: Option<String>,
    audio: Option<String>,

    #[props(default = "summary_large_image".to_string())] twitter_card: String,
    twitter_id: Option<String>,
) -> Element {
    let twitter_id = match twitter_id {
        Some(id) => Some(format!("@{}", id.trim_start_matches("@"))),
        None => None,
    };

    rsx! {
        // metadata for SEO
        document::Title { "{title}" }

        if let Some(description) = description {
            document::Meta { name: "description", content: "{description}" }
            document::Meta { property: "og:description", content: "{description}" }
        }

        // metadata for Open Graph
        document::Meta { property: "og:title", content: "{title}" }
        document::Meta { property: "og:type", content: "website" }
        document::Meta { property: "twitter:card", content: "{twitter_card}" }

        // metadata for Twitter
        if let Some(twitter_id) = twitter_id {
            document::Meta { property: "twitter:site", content: "{twitter_id}" }
            document::Meta { property: "twitter:creator", content: "{twitter_id}" }
        }

        if let Some(image) = image {
            document::Meta { property: "og:image", content: "{image}" }
            document::Meta { property: "twitter:image", content: "{image}" }
        }

        if let Some(video) = video {
            document::Meta { property: "og:video", content: "{video}" }
        }

        if let Some(audio) = audio {
            document::Meta { property: "og:audio", content: "{audio}" }
            document::Meta { content: "audio/mpeg", property: "og:audio:type" }
        }
    }
}
