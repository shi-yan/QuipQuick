use inquire::{
    validator::{StringValidator, Validation},
    Confirm, Text,
};

use slugify::slugify;
use std::{fs::{self, File}, io::Write};
use std::path::Path;
use std::io::LineWriter;

pub fn new_blog(
    title: Option<String>,
    folder: Option<String>,
    target: Option<String>,
    quiet: bool,
) {
    if quiet && title.is_some() && folder.is_some() && target.is_some() {
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

        let blog_title = if let Ok(blog_title) = Text::new("What is the blog's title?")
            .with_default(if let Some(ref n) = title {
                n
            } else {
                "My Blog"
            })
            .with_validator(non_empty_validator)
            .prompt()
        {
            blog_title
        } else {
            panic!("Wrong blog title!");
        };

        let default_blog_folder = if let Some(f) = folder {
            f
        } else {
            slugify!(&blog_title, separator = "_")
        };

        let blog_folder = if let Ok(blog_folder) = Text::new("What is the blog's work directory?")
            .with_default(&default_blog_folder)
            .with_help_message(format!("This folder is for saving your blog's raw content, such as markdown files. (Enter to accept {})", &default_blog_folder).as_str())
            .with_validator(non_empty_validator)
            .prompt()
        {
            blog_folder
        } else {
            panic!("Wrong blog folder!");
        };

        let blog_folder_exists = Path::new(&blog_folder).exists();

        if blog_folder_exists {
            let target_is_dir: bool = Path::new(&blog_folder).is_dir();
            if !target_is_dir {
                println!("{} is not a folder.", blog_folder);
                return;
            } else {
                let cont = Confirm::new(format!("The specified folder {} exists, please confirm you want to deploy your blog content in this folder?", &blog_folder).as_str()).with_default(false)
                .with_help_message("Existing files under this folder will be overwritten.")
                .prompt();

                if !cont.unwrap() {
                    println!("Terminated due to non-empty folder");
                    return;
                }
            }
        } else {
            fs::create_dir_all(&blog_folder)
                .expect(format!("Unable to create blog folder: {}.", &blog_folder).as_str());
        }

        let default_blog_target = if let Some(t) = target {
            t
        } else {
            format!("../{}_dist", &blog_folder)
        };

        let blog_target = if let Ok(blog_target) = Text::new(format!("What is the blog's target directory (relative to the above folder {})?", &blog_folder).as_str())
            .with_default(&default_blog_target)
            .with_help_message(format!("This folder is for saving rendered HTML pages. Please enter a path relative to {}. Recommend a sibling directory to {}. (Enter to accept {})", &blog_folder, &blog_folder, &default_blog_target).as_str())
            .with_validator(non_empty_validator)
            .prompt()
        {
            blog_target
        } else {
            panic!("Wrong blog target!");
        };

        let full_blog_target = format!("{}/{}", &blog_folder, blog_target);

        let blog_target_exists = Path::new(&full_blog_target).exists();

        if blog_target_exists {
            let target_is_dir: bool = Path::new(&full_blog_target).is_dir();
            if !target_is_dir {
                println!("{} is not a folder.", &full_blog_target);
                return;
            } else {
                let cont = Confirm::new(format!("The specified folder {} exists, please confirm you want to generate your blog in this folder?", &full_blog_target).as_str()).with_default(false)
                .with_help_message("This folder is for saving rendered HTML pages. Existing files under this folder will be overwritten.")
                .prompt();

                if !cont.unwrap() {
                    print!("Terminated due to non-empty folder");
                    return
                }
            }
        } else {
            fs::create_dir_all(&full_blog_target)
                .expect(format!("Unable to create blog target folder: {}.", &full_blog_target).as_str());
        }

        let file = File::create(format!("{}/quipquick.toml", &blog_folder)).unwrap();
        let mut file = LineWriter::new(file);

        file.write_all(format!("title = \"{}\"\n", &blog_title).as_bytes()).unwrap();
        file.write_all(b"# Your github repo\n").unwrap();
        file.write_all(b"repo = \"https://github.com/shi-yan/QuipQuick\"\n").unwrap();
        file.write_all(b"# Url prefix if your blog is not deployed at the root. Need to have the slash /.\n").unwrap();
        file.write_all(b"prefix = \"\"\n").unwrap();
        file.write_all(format!("target = \"{}\"\n", &blog_target).as_bytes()).unwrap();
        file.write_all(b"# Blog url\n").unwrap();
        file.write_all(b"url = \"http://localhost:8000\"\n").unwrap();
        file.write_all(b"description = \"\"\"\n").unwrap();
        file.write_all(b"Your blog's description.\"\"\"\n").unwrap();
        file.write_all(b"# google_analytics_id = \"\"\n").unwrap();
        file.write_all(b"# Your blog's github discussion url\n").unwrap();
        file.write_all(b"# discussion_url = \"https://github.com/shi-yan/shi-yan.github.io/discussions\"\n").unwrap();
        file.write_all(b"logo = \"326807.jpeg\"\n").unwrap();
        file.write_all(b"\ncontent =[\"diffusion_models_the_forward_pass\",]\n").unwrap();
        file.flush();

        println!("Your blog {} has been generated in {}.", &blog_title, &blog_folder);
        println!("Modify the manifest file {}/quipquick.toml to configure your blog.", &blog_folder);

    }
}
