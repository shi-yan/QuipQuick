use inquire::{
    validator:: Validation,
    Confirm, Text,
};

use slugify::slugify;
use std::io::LineWriter;
use std::path::Path;
use std::{
    fs::{self, File},
    io::Write,
};
use chrono::Local;
use chrono::DateTime;
use toml_edit::{Document, value};

pub fn new_post(
    title: Option<String>,
    quiet: bool,
) {
    if quiet && title.is_some() {
    } else {
        let non_empty_validator = |input: &str| {
            if input.chars().count() == 0 {
                Ok(Validation::Invalid(
                    "You're only allowed 140 characters.".into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        };

        let post_title = if let Ok(post_title) = Text::new("What is the post title?")
            .with_default(if let Some(ref n) = title {
                n
            } else {
                "My New Post"
            })
            .with_validator(non_empty_validator)
            .prompt()
        {
            post_title
        } else {
            panic!("Wrong blog title!");
        };

        let default_post_folder = 
            slugify!(&post_title, separator = "_");

        let post_folder_exists = Path::new(&default_post_folder).exists();

        if post_folder_exists {
            let target_is_dir: bool = Path::new(&default_post_folder).is_dir();
            if !target_is_dir {
                println!("{} is not a folder.", default_post_folder);
                return;
            } else {
                let cont = Confirm::new(format!("The specified folder {} exists, please confirm you want to deploy your content in this folder?", &default_post_folder).as_str()).with_default(false)
                .with_help_message("Existing files under this folder will be overwritten.")
                .prompt();

                if !cont.unwrap() {
                    println!("Terminated due to non-empty folder");
                    return;
                }
            }
        } else {
            fs::create_dir_all(&default_post_folder)
                .expect(format!("Unable to create post folder: {}.", &default_post_folder).as_str());
        }


        let file = File::create(format!("{}/content.md", &default_post_folder)).unwrap();
        let mut file = LineWriter::new(file);
        let current_time: DateTime<Local> = Local::now();

        file.write_all(b"---\n").unwrap();
        file.write_all(format!("title: \"{}\"\n", post_title).as_bytes()).unwrap();
        file.write_all(format!("date: \"{}\"\n", current_time.format("%Y-%m-%d")).as_bytes()).unwrap();
        file.write_all(b"description: \"\"\n").unwrap();
        file.write_all(b"tags: []\n").unwrap();
        file.write_all(b"---\n").unwrap();
        file.write_all(b"\ncontent here\n").unwrap();

        file.flush().unwrap();

        let contents = fs::read_to_string("quipquick.toml").expect("Should have been able to read the file");

        let mut doc = contents.parse::<Document>().expect("Invalid quipquick.toml");

        let content_array = doc["content"].as_array_mut().expect("Not content array in manifest");

        content_array.push(default_post_folder);

        fs::write("quipquick.toml", doc.to_string()).unwrap();
    }
}
