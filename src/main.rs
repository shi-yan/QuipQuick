use clap::{Parser, Subcommand};
use dateparser::parse_with_timezone;
use handlebars::{handlebars_helper, Handlebars, JsonRender};
use std::cmp::Ordering;
use std::error::Error;
use std::fs::File;
use std::fs::{self, FileType};
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use time::format_description::parse;
use time::OffsetDateTime;
use toml::Value;
extern crate fs_extra;
use chrono::Local;
use chrono::{DateTime, Datelike};
use fs_extra::dir::CopyOptions;
use fs_extra::TransitProcess;
use handlebars::JsonValue;
use image::io::Reader as ImageReader;
use markdown::{Constructs, Options, ParseOptions};
use rust_embed::RustEmbed;
use std::cmp;
use std::collections::{HashMap, HashSet};
extern crate slug;
use itertools::Itertools;
use rss::{ChannelBuilder, GuidBuilder, ImageBuilder, Item, ItemBuilder};
use slug::slugify;
use serde_json::json;

mod frontmatter;
use crate::frontmatter::FrontmatterInfo;
mod post;
use crate::post::{Tag, Post};
mod md2html;
use crate::md2html::{Footnote, SelectedMetaImage, render_markdown};
use markdown::to_mdast;

#[derive(RustEmbed)]
#[folder = "template_src/"]
struct Template;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand, Debug)]
#[command(about, long_about = None)]
enum Commands {
    /// Start a new blog.
    New {
        /// Blog name
        name: String,

        /// Folder for your blog's raw content
        #[arg(short, long)]
        folder: Option<String>,

        /// Target folder for the generated blog
        #[arg(short, long, default_value_t = String::from("../dist"))]
        target: String,
    },
    /// Create a new post.
    Write {
        /// Blog title
        title: String,
    },
    /// Generate your blog.
    Pub {
        /// Output directory
        #[arg(short, long, default_value_t = String::from("dist"))]
        target: String,

        /// Manifest file
        #[arg(short, long, default_value_t = String::from("quipquick.toml"))]
        manifest: String,

        /// Blog url prefix
        #[arg(short, long)]
        prefix: Option<String>,
    },
}

fn generate_google_analytics_id(id: &str) -> String {
    return format!(
        "<!-- Google tag (gtag.js) -->\n\
    <script async src=\"https://www.googletagmanager.com/gtag/js?id={}\"></script>\n\
    <script>\n\
      window.dataLayer = window.dataLayer || [];\n\
      function gtag() {{ dataLayer.push(arguments); }}\n\
      gtag('js', new Date());\n\
      gtag('config', '{}');\n\
    </script>",
        id, id
    );
}

fn populate_templates(force: bool) {
    let target_folder = "template";
    let target_folder_exists = Path::new("template").exists();

    if target_folder_exists {
        let target_is_dir: bool = Path::new(target_folder).is_dir();
        if !target_is_dir {
            println!("template {} is not a folder.", target_folder);
            return;
        }
    } else {
        fs::create_dir(target_folder)
            .expect(format!("Unable to create template folder: {}.", target_folder).as_str());
    }

    let files = ["post.html", "index.html", "style.css"];

    for f in files {
        let file_path = format!("{}/{}", target_folder, f);
        let template_exists = Path::new(&file_path).exists();

        if force || !template_exists {
            let content = Template::get(f).unwrap();
            fs::write(&file_path, content.data).unwrap();
        }
    }
}


fn publish(manifest:String, target: String) {

    let current_time: DateTime<Local> = Local::now();

    populate_templates(true);

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let contents = fs::read_to_string(manifest).expect("Should have been able to read the file");

    let value = match contents.parse::<Value>() {
        Err(error) => {
            println!("Toml Parsing Error: {}", error.to_string());
            return;
        }
        Ok(value) => value,
    };

    if let toml::Value::Table(ref global) = value {
        let mut target_folder = target.clone();
        if global.contains_key("target") {
            if let Some(target) = global.get("target") {
                if let toml::Value::String(target_str) = target {
                    target_folder = target_str.clone();
                }
            }
        }

        let target_folder_exists = Path::new(&target_folder).exists();

        if target_folder_exists {
            let target_is_dir: bool = Path::new(&target_folder).is_dir();
            if !target_is_dir {
                println!("Target {} is not a folder.", &target_folder);
                return;
            }

            let items = fs::read_dir(&target_folder).unwrap();

            for path in items {
                if let Ok(item) = path {
                    if !item.file_name().eq_ignore_ascii_case(".git")
                        && !item.file_name().eq_ignore_ascii_case("README.md")
                    {
                        println!("Removing {:?} {:?}", item.path(), item.file_name());
                        if let Ok(file_type) = item.file_type() {
                            if file_type.is_dir() {
                                fs::remove_dir_all(item.path()).unwrap();
                            } else {
                                fs::remove_file(item.path()).unwrap();
                            }
                        }
                    }
                }
            }
        } else {
            fs::create_dir(&target_folder)
                .expect(format!("Unable to create target folder: {}.", &target_folder).as_str());
        }

        let blog_title = if global.contains_key("title") {
            let mut title = String::new();
            if let Some(title_value) = global.get("title") {
                if let toml::Value::String(title_str) = title_value {
                    title = title_str.clone();
                }
            }
            title
        } else {
            String::new()
        };

        let blog_description = if global.contains_key("description") {
            let mut description = String::new();
            if let Some(description_value) = global.get("description") {
                if let toml::Value::String(description_str) = description_value {
                    description = description_str.clone();
                }
            }
            description
        } else {
            String::new()
        };

        let repo = if global.contains_key("repo") {
            let mut repo = String::new();
            if let Some(repo_value) = global.get("repo") {
                if let toml::Value::String(repo_str) = repo_value {
                    repo = repo_str.clone();
                }
            }
            repo
        } else {
            String::new()
        };

        let blog_url = if global.contains_key("url") {
            let mut url = String::new();
            if let Some(url_value) = global.get("url") {
                if let toml::Value::String(url_str) = url_value {
                    url = url_str.clone();
                }
            }
            url
        } else {
            String::new()
        };

        let meta_image = if global.contains_key("meta_image") {
            let mut meta_image = String::new();
            if let Some(meta_image_value) = global.get("meta_image") {
                if let toml::Value::String(meta_image_str) = meta_image_value {
                    meta_image = meta_image_str.clone();
                }
            }
            meta_image
        } else {
            String::new()
        };

        let google_analytics_id = if global.contains_key("google_analytics_id") {
            let mut google_analytics_id = String::new();
            if let Some(google_analytics_id_value) = global.get("google_analytics_id") {
                if let toml::Value::String(google_analytics_id_str) = google_analytics_id_value {
                    google_analytics_id = google_analytics_id_str.clone();
                }
            }
            google_analytics_id
        } else {
            String::new()
        };

        let discussion_url = if global.contains_key("discussion_url") {
            let mut discussion_url = String::new();
            if let Some(discussion_url_value) = global.get("discussion_url") {
                if let toml::Value::String(discussion_url_str) = discussion_url_value {
                    discussion_url = discussion_url_str.clone();
                }
            }
            Some(discussion_url)
        } else {
            None
        };

        let logo = if global.contains_key("logo") {
            if let Some(logo_value) = global.get("logo") {
                if let toml::Value::String(logo_path) = logo_value {
                    if Path::new(logo_path).exists() {
                        let img = ImageReader::open(logo_path).unwrap().decode().unwrap();
                        let aspect_ratio = ((img.width() as f32 / img.height() as f32) - 1.0).abs();
                        Some(SelectedMetaImage {
                            pixels: img.width() * img.height(),
                            aspect_ratio,
                            url: logo_path.to_string(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let content = match global.get("content") {
            None => {
                println!("Warning: No content detected.");
                return;
            }
            Some(content) => content,
        };

        let template = fs::read_to_string("template/post.html")
            .expect("Should have been able to read the file");

        let reg = Handlebars::new();
        // reg.register_helper("md", Box::new(md));

        let mut post_list: Vec<Post> = Vec::new();

        if let toml::Value::Array(ref content) = content {
            for c in content {
                let folder = c.as_str().unwrap();

                if folder == "tags" {
                    println!("There shouldn't be a content folder named tags");
                    return;
                }

                let target_folder_exists =
                    Path::new(format!("{}/{}", target_folder, folder).as_str()).exists();

                if !target_folder_exists {
                    fs::create_dir(format!("{}/{}", target_folder, folder).as_str())
                        .expect(format!("Unable to create target folder: {}.", &folder).as_str());
                }

                let path = format!("{}/content.md", folder);

                let markdown =
                    fs::read_to_string(path).expect("Should have been able to read the file");

                //println!("markdown {}", markdown);

                let mut options = Options::gfm();
                options.parse.constructs.math_text = true;
                options.parse.constructs.frontmatter = true;
                options.parse.constructs.math_flow = true;

                let ast = to_mdast(&markdown, &options.parse).unwrap();

                //println!("{:?}", ast);
                let mut rendered_string = String::new();
                let mut word_count: usize = 0;
                let mut frontmatter: FrontmatterInfo = FrontmatterInfo {
                    title: String::new(),
                    date: String::new(),
                    description: String::new(),
                    tags: vec![],
                };
                let mut selected_meta_image = logo.clone();
                let mut langs: HashSet<String> = HashSet::new();
                let mut footnotes: HashMap<String, Footnote> = HashMap::new();

                render_markdown(
                    &ast,
                    &mut rendered_string,
                    folder,
                    &target_folder,
                    &mut word_count,
                    &mut frontmatter,
                    &mut langs,
                    &mut selected_meta_image,
                    &mut footnotes,
                );
                if footnotes.len() > 0 {
                    rendered_string += "<table class=\"footnote-def\">";
                    for key in footnotes.keys().sorted() {
                        let f = footnotes.get(key).unwrap();
                        rendered_string += format!(
                            "<tr class=\"footnote-row\" id=\"footnote_{}\"><td>[{}]: </td><td>{}</td></tr>",
                            f.id, f.count, f.html
                        )
                        .as_str();
                    }
                    rendered_string += "</table>";
                }

                let d = parse_with_timezone(&frontmatter.date, &chrono::offset::Local).unwrap();
                let mut tags: Vec<Tag> = Vec::new();
                for t in &frontmatter.tags {
                    tags.push(Tag {
                        slug: slugify(t),
                        tag: t.to_lowercase(),
                    });
                }

                let data = Post {
                    date: d.into(),
                    description: frontmatter.description,
                    src: folder.to_string(),
                    md: rendered_string,
                    title: frontmatter.title,
                    tags: tags,
                    word_count: word_count,
                    blog_title: blog_title.clone(),
                    blog_url: blog_url.clone(),
                    repo: repo.clone(),
                    quipquick_version: VERSION.to_string(),
                    current_time: format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
                    google_analytics: generate_google_analytics_id(&google_analytics_id),
                    read_time: word_count as u32 / 238,
                    older_post: None,
                    newer_post: None,
                    discussion_url: discussion_url.clone(),
                    meta_img: if let Some(si) = selected_meta_image {
                        Some(si.url)
                    } else {
                        None
                    },
                    langs: langs,
                };
                post_list.push(data.clone());
            }
        }

        post_list.sort_by(|a, b| {
            if a.date < b.date {
                return Ordering::Greater;
            } else if a.date == b.date {
                return Ordering::Equal;
            } else {
                return Ordering::Less;
            }
        });

        const PAGE_ITEM_COUNT: u32 = 5;

        let page_size: u32 = (post_list.len() as f32 / PAGE_ITEM_COUNT as f32).ceil() as u32;

        let mut tags: HashMap<String, (String, Vec<u32>)> = HashMap::new();

        let mut rss_items: Vec<Item> = Vec::new();

        for index in 0..post_list.len() {
            if index > 0 {
                post_list[index].newer_post = Some((
                    post_list[index - 1].title.clone(),
                    post_list[index - 1].src.clone(),
                ));
            }

            if index < post_list.len() - 1 {
                post_list[index].older_post = Some((
                    post_list[index + 1].title.clone(),
                    post_list[index + 1].src.clone(),
                ));
            }

            println!(
                "Generating article {} {}",
                format!(
                    "{}-{:0width$}-{:0width$}",
                    &post_list[index].date.year(),
                    &post_list[index].date.month(),
                    &post_list[index].date.day(),
                    width = 2
                )
                .as_str(),
                &post_list[index].title
            );

            let rendered = reg.render_template(&template, &post_list[index]).unwrap();

            let output_path = format!("{}/{}/index.html", target_folder, &post_list[index].src);

            fs::write(output_path, rendered).unwrap();

            for t in &post_list[index].tags {
                if tags.contains_key(&t.slug) {
                    tags.get_mut(&t.slug).unwrap().1.push(index as u32);
                } else {
                    tags.insert(t.slug.clone(), (t.tag.clone(), vec![index as u32]));
                }
            }

            let permanent_link = format!("{}/{}", &blog_url, post_list[index].src);
            let guid = GuidBuilder::default().value(permanent_link.clone()).build();

            let item = ItemBuilder::default()
                .title(Some(post_list[index].title.clone()))
                .link(Some(permanent_link))
                .description(Some(post_list[index].description.clone()))
                .comments(post_list[index].discussion_url.clone())
                .guid(Some(guid))
                .build();
            rss_items.push(item)
        }

        let rss_image = ImageBuilder::default()
            .url(format!("{}/{}", &blog_url, meta_image))
            .title(blog_title.clone())
            .link(blog_url.clone())
            .build();

        let channel = ChannelBuilder::default()
            .title(blog_title.clone())
            .link(blog_url.clone())
            .description(blog_description.clone())
            .items(rss_items)
            .image(Some(rss_image))
            .build();

        let rss_output_path = format!("{}/rss.xml", target_folder);

        // println!("{} {:?}", channel.to_string(), channel.validate().unwrap());

        fs::write(rss_output_path, channel.to_string()).unwrap();

        let index_template = fs::read_to_string("template/index.html")
            .expect("Should have been able to read the file");

        for index in 0..page_size {
            let mut pages = Vec::new();

            for pindex in 0..page_size {
                pages.push(json!({"id":pindex+1,"current":index == pindex, "link":if pindex ==0 {String::from("/index.html")} else {format!("/index{}.html",pindex+1)}}));
            }

            let page_range = (index * PAGE_ITEM_COUNT) as usize
                ..cmp::min((index + 1) * PAGE_ITEM_COUNT, post_list.len() as u32) as usize;

            let mut data = json!({
                "posts": post_list[page_range],
                "repo": repo,
                "pages": pages,
                "blog_title": blog_title,
                "blog_description": blog_description,
                "blog_url":blog_url,
                "quipquick_version": VERSION,
                "current_time": format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
                "google_analytics": generate_google_analytics_id(&google_analytics_id)
            });

            if let Some(logo) = &logo {
                fs::copy(&logo.url, format!("{}/{}", target_folder, logo.url)).unwrap();
                data.as_object_mut()
                    .unwrap()
                    .insert("logo".to_string(), JsonValue::String(logo.url.clone()));
            }

            if index > 0 {
                let prev_path = if index - 1 == 0 {
                    String::from("/index.html")
                } else {
                    format!("/index{}.html", index)
                };
                data.as_object_mut()
                    .unwrap()
                    .insert("prev".to_string(), JsonValue::String(prev_path));
            }

            if index < page_size - 1 {
                let next_path = format!("/index{}.html", index + 2);
                data.as_object_mut()
                    .unwrap()
                    .insert("next".to_string(), JsonValue::String(next_path));
            }

            let index_rendered = reg.render_template(&index_template, &data).unwrap();

            let output_path = if index == 0 {
                format!("{}/index.html", target_folder)
            } else {
                format!("{}/index{}.html", target_folder, index + 1)
            };

            fs::write(output_path, index_rendered).unwrap();
        }

        for t in tags {
            let folder = t.0.as_str();

            let target_folder_exists =
                Path::new(format!("{}/tags/{}", target_folder, folder).as_str()).exists();

            if !target_folder_exists {
                fs::create_dir_all(format!("{}/tags/{}", target_folder, folder).as_str())
                    .expect(format!("Unable to create tag folder: {}.", &folder).as_str());
            }

            let tag_page_size: u32 = (t.1 .1.len() as f32 / PAGE_ITEM_COUNT as f32).ceil() as u32;

            let mut tag_post_list: Vec<Post> = Vec::new();

            for tid in &t.1 .1 {
                tag_post_list.push(post_list[*tid as usize].clone());
            }

            for index in 0..tag_page_size {
                let mut pages = Vec::new();

                for pindex in 0..tag_page_size {
                    pages.push(json!({"id":pindex+1,"current":index == pindex, "link":if pindex ==0 {String::from("/index.html")} else {format!("/index{}.html",pindex+1)}}));
                }

                let page_range = (index * PAGE_ITEM_COUNT) as usize
                    ..cmp::min((index + 1) * PAGE_ITEM_COUNT, t.1 .1.len() as u32) as usize;

                let mut data = json!({
                    "posts": tag_post_list[page_range],
                    "repo": repo,
                    "pages": pages,
                    "blog_title": blog_title,
                    "blog_description": blog_description,
                    "blog_url":blog_url,
                    "quipquick_version": VERSION,
                    "current_time": format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
                    "google_analytics": generate_google_analytics_id(&google_analytics_id)
                });

                if let Some(logo) = &logo {
                    fs::copy(&logo.url, format!("{}/{}", target_folder, logo.url)).unwrap();
                    data.as_object_mut()
                        .unwrap()
                        .insert("logo".to_string(), JsonValue::String(logo.url.clone()));
                }

                if index > 0 {
                    let prev_path = if index - 1 == 0 {
                        format!("/tags/{}/index.html", t.0)
                    } else {
                        format!("/tags/{}/index{}.html", t.0, index)
                    };
                    data.as_object_mut()
                        .unwrap()
                        .insert("prev".to_string(), JsonValue::String(prev_path));
                }

                if index < tag_page_size - 1 {
                    let next_path = format!("/tags/{}/index{}.html", t.0, index + 2);
                    data.as_object_mut()
                        .unwrap()
                        .insert("next".to_string(), JsonValue::String(next_path));
                }

                data.as_object_mut()
                    .unwrap()
                    .insert("page_tag".to_string(), JsonValue::String(t.1 .0.clone()));

                let index_rendered = reg.render_template(&index_template, &data).unwrap();

                let output_path = if index == 0 {
                    format!("{}/tags/{}/index.html", target_folder, folder)
                } else {
                    format!("{}/tags/{}/index{}.html", target_folder, folder, index + 1)
                };

                fs::write(output_path, index_rendered).unwrap();
            }
        }

        fs::write(
            format!("{}/current_time.txt", target_folder).as_str(),
            format!("{}", current_time.format("%Y-%m-%d %H:%M:%S")),
        )
        .unwrap();
        fs::copy("template/style.css", format!("{}/style.css", target_folder)).unwrap();
    }

    /*let mut reg = Handlebars::new();
    handlebars_helper!(date2: |dt: OffsetDateTime, {fmt:str = "[year]-[month]-[day]"}|
        dt.format(&parse(fmt).unwrap()).unwrap()
    );
    reg.register_helper("date2", Box::new(date2));

    // render without register
    println!(
        "{}",
        reg.render_template("Hello {{name}}", &json!({"name": "foo"}))
            .unwrap()
    );*/

    // register template using given name
    /*reg.register_template_string("tpl_1", "Good afternoon, {{name}}")
        .unwrap();
    println!("{}", reg.render("tpl_1", &json!({"name": "foo"})).unwrap());

    let data = OffsetDateTime::now_utc();

    println!(
        "{}",
        reg.render_template("<div>{{date2 this}}</div>", &data)
            .unwrap()
    );*/

}

fn main() {
    println!(
        "         ____       _         ____       _      _    
        /___ \\_   _(_)_ __   /___ \\_   _(_) ___| | __
       //  / / | | | | '_ \\ //  / / | | | |/ __| |/ /
      / \\_/ /| |_| | | |_) / \\_/ /| |_| | | (__|   < 
      \\___,_\\ \\__,_|_| .__/\\___,_\\ \\__,_|_|\\___|_|\\_\\
                     |_|                             "
    );
    //https://patorjk.com/software/taag/#p=display&f=Ogre&t=QuipQuick


    let args = Args::parse();

    match args.command {
        Commands::New { name, folder, target } => {}
        Commands::Pub { target, manifest, prefix } => {
            publish(manifest, target);
        }
        Commands::Write { title } => {}
    }
}
