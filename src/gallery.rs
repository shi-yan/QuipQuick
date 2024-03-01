use chrono::{DateTime, Datelike, Local};
use dateparser::parse_with_timezone;
use std::fs;
use std::path::Path;
use toml::Value;

struct Image {
    pub date: DateTime<Local>,
    pub title: String,
    pub file: String,
}

pub fn gallery(target_folder: &str, gallery_path: &str) {
    if gallery_path == "tags" {
        println!("There shouldn't be a gallery folder named tags");
        return;
    }

    let target_folder_exists =
        Path::new(format!("{}/{}", target_folder, gallery_path).as_str()).exists();

    if !target_folder_exists {
        //fs::create_dir(format!("{}/{}", target_folder, gallery_path).as_str())
        //    .expect(format!("Unable to create gallery folder: {}.", &gallery_path).as_str());

        panic!("No gallery folder exist");
    }

    let contents =
        fs::read_to_string(format!("{}/{}/content.toml", target_folder, gallery_path).as_str())
            .expect("Should have been able to read the file");

    let value = match contents.parse::<Value>() {
        Err(error) => {
            println!("Toml Parsing Error: {}", error.to_string());
            return;
        }
        Ok(value) => value,
    };

    if let toml::Value::Array(ref galleries) = value {
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

                        let d =
                            parse_with_timezone(&date, &chrono::offset::Local).unwrap();

                        image_list.push(Image {
                            date: d.into(),
                            title: title,
                            file: file,
                        });
                    }
                }

                for img in image_list {}
            }
        }
    }
}
