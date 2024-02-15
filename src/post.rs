
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::HashSet;
use chrono::{DateTime, Local,Datelike};

#[derive(Debug, Clone)]
pub struct Tag {
    pub slug: String,
    pub tag: String,
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6)).unwrap();

        map.serialize_entry("slug", &self.slug).unwrap();
        map.serialize_entry("tag", &self.tag).unwrap();

        map.end()
    }
}

#[derive(Debug, Clone)]
pub struct Post {
    pub date: DateTime<Local>,
    pub description: String,
    pub src: String,
    pub md: String,
    pub title: String,
    pub tags: Vec<Tag>,
    pub word_count: usize,
    pub repo: String,
    pub blog_title: String,
    pub blog_url: String,
    pub quipquick_version: String,
    pub current_time: String,
    pub google_analytics: String,
    pub read_time: u32,
    pub older_post: Option<(String, String)>,
    pub newer_post: Option<(String, String)>,
    pub discussion_url: Option<String>,
    pub meta_img: Option<String>,
    pub langs: HashSet<String>,
}

impl Serialize for Post {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6)).unwrap();

        map.serialize_entry(
            "date",
            format!(
                "{}-{:0width$}-{:0width$}",
                self.date.year(),
                self.date.month(),
                self.date.day(),
                width = 2
            )
            .as_str(),
        )
        .unwrap();
        map.serialize_entry("description", &self.description)
            .unwrap();
        map.serialize_entry("src", &self.src).unwrap();
        map.serialize_entry("md", &self.md).unwrap();
        map.serialize_entry("title", &self.title).unwrap();

        map.serialize_entry("tags", &self.tags).unwrap();

        map.serialize_entry("word_count", &self.word_count).unwrap();
        map.serialize_entry("repo", &self.repo).unwrap();
        map.serialize_entry("blog_url", &self.blog_url).unwrap();
        map.serialize_entry("blog_title", &self.blog_title).unwrap();
        map.serialize_entry("quipquick_version", &self.quipquick_version)
            .unwrap();
        map.serialize_entry("current_time", &self.current_time)
            .unwrap();
        map.serialize_entry("google_analytics", &self.google_analytics)
            .unwrap();
        let read_time_str = if self.read_time > 1 {
            format!("{} Mins", &self.read_time)
        } else {
            format!("{} Min", &self.read_time)
        };
        map.serialize_entry("read_time", &read_time_str).unwrap();

        if let Some(newer_post) = &self.newer_post {
            map.serialize_entry("newer_post_title", &newer_post.0)
                .unwrap();
            map.serialize_entry("newer_post_folder", &newer_post.1)
                .unwrap();
        }

        if let Some(older_post) = &self.older_post {
            map.serialize_entry("older_post_title", &older_post.0)
                .unwrap();
            map.serialize_entry("older_post_folder", &older_post.1)
                .unwrap();
        }

        if let Some(discussion_url) = &self.discussion_url {
            map.serialize_entry("discussion_url", discussion_url)
                .unwrap();
        }

        if let Some(mi) = &self.meta_img {
            map.serialize_entry("meta_img", mi).unwrap();
        }

        if self.langs.len() > 0 {
            map.serialize_entry("langs", &self.langs).unwrap();
        }

        map.end()
    }
}
