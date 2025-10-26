use crate::publish::generate_google_analytics_id;
use chrono::{DateTime, Datelike, Local};
use dateparser::parse_with_timezone;
use handlebars::{Handlebars};
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml::Value;

fn generate_video_preview(video_path: &str, preview_path: &str) -> Result<(), String> {
    // Check if ffmpeg is installed
    let ffmpeg_check = Command::new("ffmpeg")
        .arg("-version")
        .output();

    if ffmpeg_check.is_err() {
        return Err("ffmpeg is not installed or not found in PATH. Please install ffmpeg to generate video previews.".to_string());
    }

    // Generate preview image from first frame
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg("-frames:v")
        .arg("1")
        .arg("-y") // Overwrite output file if it exists
        .arg(preview_path)
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("Generated preview: {}", preview_path);
                Ok(())
            } else {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                Err(format!("ffmpeg failed to generate preview: {}", error_msg))
            }
        }
        Err(e) => Err(format!("Failed to execute ffmpeg: {}", e))
    }
}

struct Image {
    pub date: DateTime<Local>,
    pub title: String,
    pub file: String,
    pub preview: String,
}

impl Serialize for Image {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(7)).unwrap();

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
        map.serialize_entry("title", titlecase::titlecase(&self.title).as_str())
            .unwrap();

        map.serialize_entry("file", &self.file).unwrap();
        map.serialize_entry("preview", &self.preview).unwrap();
        map.end()
    }
}

struct Gallery {
    pub gallery_title: String,
    pub gallery_description: String,
    pub images: Vec<Image>,
    pub repo: String,
    pub blog_title: String,
    pub blog_description: String,
    pub blog_url: String,
    pub quipquick_version: String,
    pub google_analytics: String,
}

impl Serialize for Gallery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6)).unwrap();
        map.serialize_entry("gallery_description", &self.gallery_description)
            .unwrap();
        map.serialize_entry(
            "gallery_title",
            titlecase::titlecase(&self.gallery_title).as_str(),
        )
        .unwrap();
        map.serialize_entry("images", &self.images).unwrap();

        map.serialize_entry("repo", &self.repo).unwrap();
        map.serialize_entry("blog_url", &self.blog_url).unwrap();
        map.serialize_entry("blog_title", &self.blog_title).unwrap();
        map.serialize_entry("blog_description", &self.blog_description).unwrap();
        map.serialize_entry("quipquick_version", &self.quipquick_version)
            .unwrap();
        map.serialize_entry("google_analytics", &self.google_analytics)
            .unwrap();
        map.end()
    }
}

pub fn generate_gallery(
    target_folder: &str,
    gallery_path: &str,
    repo: &str,
    blog_title: &str,
    blog_description: &str,
    blog_url: &str,
    quipquick_version: &str,
    google_analytics_id: &str,
) {
    if gallery_path == "tags" {
        println!("There shouldn't be a gallery folder named tags");
        return;
    }

    let target_folder_exists =
        Path::new(format!("{}/{}", target_folder, gallery_path).as_str()).exists();

    if !target_folder_exists {
        fs::create_dir(format!("{}/{}", target_folder, gallery_path).as_str())
            .expect(format!("Unable to create gallery folder: {}.", &gallery_path).as_str());
    }

    let contents = fs::read_to_string(format!("{}/content.toml", gallery_path).as_str())
        .expect("Should have been able to read the file");

    let value = match contents.parse::<Value>() {
        Err(error) => {
            println!("Toml Parsing Error: {}", error.to_string());
            return;
        }
        Ok(value) => value,
    };

    if let toml::Value::Array(ref galleries) = value.get("galleries").unwrap() {
        for g in galleries {
            if let toml::Value::Table(gallery) = g {
                let title = gallery
                    .get("title")
                    .and_then(|value| value.as_str())
                    .expect("Gallery title is mandatory!")
                    .to_owned();

                let description = gallery
                    .get("description")
                    .and_then(|value| value.as_str())
                    .expect("Gallery description is mandatory!")
                    .to_owned();

                let images = gallery
                    .get("images")
                    .and_then(|value| value.as_array())
                    .expect("No images in gallery.")
                    .to_owned();

                let mut image_list: Vec<Image> = Vec::new();

                for img in images {
                    if let toml::Value::Table(img) = img {
                        let title = img
                            .get("title")
                            .and_then(|value| value.as_str())
                            .expect("Gallery image title is mandatory!")
                            .to_owned();

                        let file = img
                            .get("file")
                            .and_then(|value| value.as_str())
                            .expect("Gallery image file is mandatory!")
                            .to_owned();

                        let date = img
                            .get("date")
                            .and_then(|value| value.as_str())
                            .expect("Gallery image date is mandatory!")
                            .to_owned();

                        let d = parse_with_timezone(&date, &chrono::offset::Local).unwrap();

                        // Generate preview image name
                        let file_stem = Path::new(&file).file_stem().unwrap().to_str().unwrap();
                        let preview_file = format!("{}_preview.png", file_stem);
                        
                        // Generate preview image from video
                        let video_path = format!("{}/{}", gallery_path, file);
                        let preview_path = format!("{}/{}", gallery_path, preview_file);
                        
                        match generate_video_preview(&video_path, &preview_path) {
                            Ok(()) => {
                                println!("Successfully generated preview for {}", file);
                            }
                            Err(e) => {
                                eprintln!("Error generating preview for {}: {}", file, e);
                                // Continue without preview - fallback to video element
                            }
                        }

                        image_list.push(Image {
                            date: d.into(),
                            title: title,
                            file: file,
                            preview: preview_file,
                        });
                    }
                }

                image_list.sort_by(|a, b| {
                    if a.date < b.date {
                        return Ordering::Greater;
                    } else if a.date == b.date {
                        return Ordering::Equal;
                    } else {
                        return Ordering::Less;
                    }
                });

                for img in &image_list {
                    // Copy video file
                    let copy_from = format!("{}/{}", gallery_path, img.file);
                    let copy_to = format!("{}/{}/{}", target_folder, gallery_path, img.file);
                    std::fs::copy(&copy_from, &copy_to).unwrap();
                    
                    // Copy preview image if it exists
                    let preview_from = format!("{}/{}", gallery_path, img.preview);
                    let preview_to = format!("{}/{}/{}", target_folder, gallery_path, img.preview);
                    
                    if Path::new(&preview_from).exists() {
                        match std::fs::copy(&preview_from, &preview_to) {
                            Ok(_) => println!("Copied preview: {}", img.preview),
                            Err(e) => eprintln!("Failed to copy preview {}: {}", img.preview, e),
                        }
                    }
                }

                let gallery = Gallery {
                    gallery_title: title,
                    gallery_description: description,
                    images: image_list,
                    repo:repo.to_owned(),
                    blog_title: blog_title.to_owned(),
                    blog_description: blog_description.to_owned(),
                    blog_url:blog_url.to_owned(),
                    quipquick_version: quipquick_version.to_owned(),
                    google_analytics: generate_google_analytics_id(&google_analytics_id)
                };

                let reg = Handlebars::new();
                let gallery_template = fs::read_to_string("template/gallery.html")
                    .expect("Should have been able to read the file");

                let gallery_rendered = reg.render_template(&gallery_template, &gallery).unwrap();

                let output_path = format!("{}/{}/index.html", target_folder, gallery_path);
                println!("gallery {}", output_path);
                fs::write(output_path, gallery_rendered).unwrap();
            }
        }
    }
}
